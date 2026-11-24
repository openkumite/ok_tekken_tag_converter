//! Arena geometry: VIF packets for VU1. Each batch is one triangle strip in world space:
//!   STCYCL 4,4   UNPACK S1-32 x1   flags
//!                UNPACK V4-32 x5   GIF tag + ALPHA, CLAMP, TEX0, then the strip's own GIF tag
//!   STCYCL 4,1   UNPACK V4-8  xN   vertex colours (0x80 = 1.0)
//!   STCYCL 4,1   UNPACK V2-32 xN   texture coordinates
//!   STCYCL 4,2   UNPACK V3-32 x2N  normal, position
use crate::glb::{self, Glb};
use crate::model::SCALE;
use crate::moves::VADDR;
use crate::objects;
use crate::{f32le, tex, u16le, u32le};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// The floor is repeated no further from the centre, and to no more vertices in all (the arenas' go 8 tiles out,
/// to 300 000 vertices): a broken grid cannot ask for more memory.
const REACH: i32 = 64;
const FLOOR_VERTICES: usize = 4 << 20;

#[derive(Clone, PartialEq)]
/// `mode`: 0 solid or cut-out, 1 adds to what is behind (lights, gloss), 2 multiplies it (shadow and
/// dirt decals), 3 ordinary blending (clouds, glass, soft edges).
pub struct Batch {
    pub tex0: u64,
    /// the word the microcode dispatches on ([`colour`])
    pub flags: u32,
    pub mode: u8,
    pub floor: bool,
    /// its PRIM's FGE: the GS fogs it
    pub fog: bool,
    /// which texture bank its TEX0 refers to
    pub bank: usize,
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub col: Vec<[u8; 4]>,
    /// a floor cell's: its centre (x, z), its level of detail (0: its levels alike, drawn at every one) and the
    /// floor's count of levels (level 0's included; 0: not a floor cell)
    pub cell: [f32; 4],
}

#[allow(clippy::indexing_slicing)] // (fixed arrays of three within constant bounds)
pub(crate) fn bbox<'a>(points: impl Iterator<Item = &'a [f32; 3]>) -> ([f32; 3], [f32; 3]) {
    points.fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), p| {
        (std::array::from_fn(|k| lo[k].min(p[k])), std::array::from_fn(|k| hi[k].max(p[k])))
    })
}

/// What opens a grid's block (FLUSH, BASE, OFFSET).
const CELL: [u8; 16] = [4, 4, 0, 1, 0, 0, 0, 0x11, 0, 0, 0, 3, 0, 2, 0, 2];

/// The blocks of a grid part, each one's batches (its vertices as stored): an arena's object part is a grid of one
/// cell whose blocks are its models (0x3D0E18 draws them by number).
pub fn blocks(d: &[u8]) -> Vec<Vec<Batch>> {
    let starts: Vec<usize> =
        (0..d.len().saturating_sub(16)).step_by(4).filter(|&o| d.get(o..o + 16) == Some(&CELL[..])).collect();
    let end = |k: usize| starts.get(k + 1).copied().unwrap_or(d.len());
    starts.iter().enumerate().map(|(k, &s)| batches(d.get(s..end(k)).unwrap_or_default())).collect()
}

/// An arena from the parts of its geometry pack: (scenery, objects), both in arena space.
///
/// A part is a grid:
///   u32 levels of detail, u32 size, u32 cells x, u32 cells z, f32 cell width, f32 cell depth, f32 height
/// followed by levels x cells blocks, each opened by FLUSH, BASE, OFFSET, with vertices relative to
/// the centre of their cell. A floor keeps every level the game draws (from 1), others the most detailed.
/// - A grid of several cells, or a flat one, is the floor around the fighters, which the game repeats in every
///   direction as they move. Here it is repeated to a little past the hole in the far ground; where
///   the two overlap they are coplanar, so floor materials are named for the viewer to draw on top.
/// - A tall grid of one cell is a fixed piece, or an object the game moves (a train): the latter is
///   recognised by standing over the centre, and kept apart.
/// - Anything else is scenery in arena space.
pub fn arena(parts: &[(usize, &[u8])]) -> (Vec<Batch>, Vec<Batch>, Vec<Floor>) {
    let (mut scenery, mut objects, mut floors) = (vec![], vec![], vec![]);
    for &(bank, d) in parts {
        if d.len() < 0x30 {
            continue;
        }
        let batches = |d: &[u8]| -> Vec<Batch> {
            let mut b = batches(d);
            for x in &mut b {
                x.bank = bank;
            }
            b
        };
        let word = |o: usize| u32le(d, o).unwrap_or(0);
        let (lods, nx, nz) = (word(0) as usize, word(8) as usize, word(12) as usize);
        let (w, dz) = (f32::from_bits(word(16)), f32::from_bits(word(20)));
        let starts: Vec<usize> =
            (0..d.len() - 16).step_by(4).filter(|&o| d.get(o..o + 16) == Some(&CELL[..])).collect();
        let cells = nx.saturating_mul(nz);
        if !(1..=8).contains(&lods)
            || cells == 0
            || cells > 1024
            || starts.len() != lods * cells
            || !(w > 0.0 && dz > 0.0)
        {
            scenery.extend(batches(d));
            continue;
        }
        let centre = |i: usize| {
            (((i % nx) as f32 - (nx as f32 - 1.0) / 2.0) * w, ((i / nx) as f32 - (nz as f32 - 1.0) / 2.0) * dz)
        };
        // a level's cells
        let level = |l: usize| -> Vec<Vec<Batch>> {
            let cell = |i: usize| {
                let k = l * cells + i;
                let block = starts.get(k).and_then(|&s| d.get(s..starts.get(k + 1).copied().unwrap_or(d.len())));
                let mut b = batches(block.unwrap_or_default());
                let (x, z) = centre(i);
                for p in b.iter_mut().flat_map(|b| b.pos.iter_mut()) {
                    p[0] += x;
                    p[2] += z;
                }
                b
            };
            (0..cells).map(cell).collect()
        };
        let best = (0..lods)
            .map(|l| level(l).concat())
            .max_by_key(|b| b.iter().map(|x| x.pos.len()).sum::<usize>())
            .unwrap_or_default();
        let (lo, hi) = bbox(best.iter().flat_map(|b| &b.pos));
        if cells > 1 || hi[1] < 700.0 {
            // the levels the game draws, from 1 (0x340408 picks a cell's by its depth: level 0 never shows); a
            // cell's levels alike kept once
            let drawn: Vec<usize> = if lods > 1 { (1..lods).collect() } else { vec![0] };
            let levels: Vec<Vec<Vec<Batch>>> = drawn.iter().map(|&l| level(l)).collect();
            let mut tiles = vec![];
            for i in 0..cells {
                let of = |k: usize| levels.get(k).and_then(|l| l.get(i));
                let alike = (1..levels.len()).all(|k| of(k) == of(0));
                let (x, z) = centre(i);
                for (k, &l) in drawn.iter().enumerate().take(if alike { 1 } else { drawn.len() }) {
                    let cell = [x, z, if alike { 0.0 } else { l as f32 }, lods as f32];
                    tiles.extend(of(k).into_iter().flatten().map(|b| Batch { floor: true, cell, ..b.clone() }));
                }
            }
            floors.push((tiles, [nx as f32 * w, nz as f32 * dz], [w, dz]));
        } else if lo[0] < 0.0 && hi[0] > 0.0 && lo[2] < 0.0 && hi[2] > 0.0 && hi[1] > 1500.0 {
            objects.extend(best); // tall and over the centre, where the fighters are: the game moves it (a train)
        } else {
            scenery.extend(best);
        }
    }
    // Things standing where the fighters are (a helicopter, a boat, a carousel): the game moves them
    // or shows them elsewhere, and how is not decoded. Kept apart so that they can be hidden.
    // shortcut: found by position, not by what the game does with them
    let seed = |b: &&Batch| {
        let (lo, hi) = bbox(b.pos.iter());
        lo[0] < 2000.0 && hi[0] > -2000.0 && lo[2] < 2000.0 && hi[2] > -2000.0 && lo[1] < 2500.0 && hi[1] > 300.0
            && hi[1] - lo[1] > 100.0 && hi[0] - lo[0] < 40000.0 && hi[2] - lo[2] < 40000.0
            // and really over the centre: the ground ring around the floor encloses it from several metres away
            && b.pos.windows(3).any(|t| {
                let &[t0, t1, t2] = t else { return false };
                let side = |a: [f32; 3], c: [f32; 3]| a[0] * c[2] - a[2] * c[0] > 0.0;
                // distance from the centre to the edge a-c, seen from above
                let near = |a: [f32; 3], c: [f32; 3]| {
                    let (dx, dz) = (c[0] - a[0], c[2] - a[2]);
                    let k = (-(a[0] * dx + a[2] * dz) / (dx * dx + dz * dz).max(1e-6)).clamp(0.0, 1.0);
                    libm::hypotf(a[0] + k * dx, a[2] + k * dz) < 3000.0
                };
                near(t0, t1) || near(t1, t2) || near(t2, t0) || (side(t0, t1) == side(t1, t2) && side(t1, t2) == side(t2, t0))
            })
    };
    let (lo, hi) = bbox(scenery.iter().filter(seed).flat_map(|b| &b.pos));
    if lo[0] < hi[0] {
        let inside = |b: &Batch| {
            let (l, h) = bbox(b.pos.iter());
            l.iter().zip(&lo).all(|(l, lo)| *l >= lo - 1000.0)
                && h.iter().zip(&hi).all(|(h, hi)| *h <= hi + 1000.0)
                && h[1] > 100.0
        };
        let (moved, kept): (Vec<Batch>, Vec<Batch>) = scenery.into_iter().partition(inside);
        objects.extend(moved);
        scenery = kept;
    }
    // the far ground: batches near floor height that reach far out; their inner edge is the hole
    let low = |p: &&[f32; 3]| p[1] < 300.0;
    let far = scenery.iter().filter(|b| b.pos.iter().any(|p| libm::hypotf(p[0], p[2]) > 10000.0));
    let hole = far.flat_map(|b| b.pos.iter().filter(low).map(|p| libm::hypotf(p[0], p[2]))).fold(f32::MAX, f32::min);
    let mut laid = vec![];
    for (tiles, [px, pz], cell) in floors {
        let radius = if hole < 60000.0 { (hole * 1.5).clamp(16000.0, 40000.0) } else { 16000.0 };
        let reach = (radius / px.min(pz)).ceil() as i32;
        let vertices: usize = tiles.iter().map(|b| b.pos.len()).sum();
        if reach > REACH || vertices.saturating_mul((2 * reach as usize + 1).pow(2)) > FLOOR_VERTICES {
            // not a floor the game has: as it is, once
            laid.push(Floor { tiles, period: [px, pz], at: vec![[0.; 2]], cell });
            continue;
        }
        let at = (-reach..=reach).flat_map(|x| (-reach..=reach).map(move |z| (x, z))).filter_map(|(ix, iz)| {
            let (ox, oz) = (ix as f32 * px, iz as f32 * pz);
            let far = libm::hypotf((ox.abs() - px / 2.0).max(0.0), (oz.abs() - pz / 2.0).max(0.0)) >= radius;
            ((ix, iz) == (0, 0) || !far).then_some([ox, oz])
        });
        laid.push(Floor { tiles, period: [px, pz], at: at.collect(), cell });
    }
    (scenery, objects, laid)
}

/// A floor of an arena: its tiles (at the centre), how far apart their copies are, and where they go (x, z), to a
/// little past the hole in the far ground.
pub struct Floor {
    pub tiles: Vec<Batch>,
    pub period: [f32; 2],
    pub at: Vec<[f32; 2]>,
    /// its cells' width and depth
    pub cell: [f32; 2],
}

impl Floor {
    /// Its tiles at every place.
    pub fn laid(&self) -> impl Iterator<Item = Batch> + '_ {
        self.at.iter().flat_map(|&[ox, oz]| {
            self.tiles.iter().cloned().map(move |mut b| {
                for p in &mut b.pos {
                    p[0] += ox;
                    p[2] += oz;
                }
                b.cell[0] += ox;
                b.cell[1] += oz;
                b
            })
        })
    }
}

/// The arena's floor levels as 0x340070 sets them (floor settings gp-0x6970 +0x30..+0x3C): a cell's level is 1, or
/// k + 1 where its mean depth (of its corners' in front of the camera) passes limit k and the floor has level k + 1
/// (0x340408); the corners are the cell's, times row +0xC of table 0x521700 (an arena's 0x30 bytes). Metres.
/// shortcut: the limits of the mode 0x3AA5A0 asks for (0, 50 m) are not given, upgrade when that mode is known
pub fn levels(elf: &[u8], arena: usize) -> Option<Value> {
    let f = |a: usize| f32le(elf, a.checked_sub(VADDR)?).map(|v| v as f64 * SCALE);
    let gp = |off: usize| f(GP - off);
    let limits = if arena == 0 { [Some(8.0); 4] } else { [gp(0x7D6C), Some(8.0), gp(0x7D68), gp(0x7D64)] };
    let limits = limits.into_iter().collect::<Option<Vec<f64>>>()?;
    let row = (FLOOR_TABLE + 0x30 * arena).checked_sub(VADDR)?;
    let window = [u16le(elf, row)?, u16le(elf, row + 2)?];
    let (reach, corner) = (f32le(elf, row + 8)? as f64 * SCALE, f32le(elf, row + 0xC)?);
    Some(json!({"limits": limits, "corner": corner, "window": window, "reach": reach}))
}

/// The floors as 0x341218 lays them around the fight: a window of N x M cells (`window`, the floors' cell size),
/// cell (j, i) at ((N / 2 - j) x width, (i - M / 2) x depth) (x mirrored: the cells read in a fight's save state)
/// showing cell (j mod n, i mod m) of each floor of n x m
/// cells. None when the floors' cells differ in size or do not tile.
fn windowed(floors: &[Floor], window: [usize; 2]) -> Option<Window> {
    let cell = floors.first()?.cell;
    let mut grids = vec![];
    for f in floors {
        let n = [f.period[0] / f.cell[0], f.period[1] / f.cell[1]];
        let tiles = f.cell == cell && n.iter().all(|n| (1.0..=64.0).contains(n) && (n - n.round()).abs() < 1e-3);
        if !tiles || window.contains(&0) {
            return None;
        }
        grids.push(n.map(|n| n.round() as usize));
    }
    // a cell's index from its centre
    let at = |c: f32, size: f32, n: usize| (c / size + (n as f32 - 1.0) / 2.0).round() as usize;
    let (mut shown, mut cells, mut places) = (BTreeMap::new(), vec![], vec![]);
    for (j, i) in (0..window[0]).flat_map(|j| (0..window[1]).map(move |i| (j, i))) {
        let key: Vec<[usize; 2]> = grids.iter().map(|&[nx, nz]| [j % nx, i % nz]).collect();
        let k = *shown.entry(key).or_insert_with(|| {
            let mut out = vec![];
            for (f, &[nx, nz]) in floors.iter().zip(&grids) {
                let mine = |b: &&Batch| at(b.cell[0], cell[0], nx) == j % nx && at(b.cell[1], cell[1], nz) == i % nz;
                for b in f.tiles.iter().filter(mine) {
                    let mut b = b.clone();
                    for p in &mut b.pos {
                        p[0] -= b.cell[0];
                        p[2] -= b.cell[1];
                    }
                    (b.cell[0], b.cell[1]) = (0.0, 0.0);
                    out.push(b);
                }
            }
            cells.push(out);
            cells.len() - 1
        });
        places
            .push((k, [(window[0] as f32 / 2.0 - j as f32) * cell[0], (i as f32 - window[1] as f32 / 2.0) * cell[1]]));
    }
    Some((cells, places))
}

/// The window's cells: what they show (centred on the origin, each once), and its cells (which, where).
type Window = (Vec<Vec<Batch>>, Vec<(usize, [f32; 2])>);

const GP: usize = 0x5F_D0F0;
pub(crate) const FLOOR_TABLE: usize = 0x52_1700;

/// An arena's lights for its geometry: towards each (the arena's axes), its colour (128 = 1), the ambient (GS units);
/// and its fog ([`fog`]).
pub struct Light {
    pub toward: [[f32; 3]; 3],
    pub colour: [[f32; 3]; 3],
    pub ambient: [f32; 3],
    pub fog: Option<Fog>,
}

/// The arenas' fog: 24 bytes an arena, `u64 FOGCOL` (r, g, b bytes), `f32 near level, near, far level, far` (mm).
const FOG: usize = 0x45_6540;

/// An arena's fog: the GS's FOGCOL, and the coefficient `line[0] + line[1] x w` (w the depth in mm; 255 none, to 0)
/// that 0x3DE6C0 makes from its row (gp-0x67A8, gp-0x67AC) and the VU1 blocks carry (8.w, 9.w).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub colour: [u8; 3],
    pub line: [f32; 2],
}

/// Arena `n`'s fog.
pub fn fog(elf: &[u8], n: usize) -> Option<Fog> {
    let o = (FOG + 24 * n).checked_sub(VADDR)?;
    let colour = [elf.get(o)?, elf.get(o + 1)?, elf.get(o + 2)?].map(|c| *c);
    let [f0, near, f1, far] = [8, 12, 16, 20].map(|k| f32le(elf, o + k));
    let (f0, near, f1, far) = (f0?, near?, f1?, far?);
    let slope = (f1 - f0) / (far - near);
    (n < 23 && slope.is_finite()).then_some(Fog { colour, line: [f0 - slope * near, slope] })
}

/// An arena's texture channel table ([`tex::channels`]): the last one at the end of geometry part 4, after its
/// light records (0x356D40 keeps it at gp-0x60A8).
/// shortcut: found by its shape (each entry's channel its place, PSMT8 or PSMT4, a size of powers of two); upgrade
/// when the part's header is decoded
pub fn channel_table(part: &[u8]) -> Vec<[u16; 8]> {
    let fits = |t: &Vec<[u16; 8]>| {
        !t.is_empty()
            && t.iter().enumerate().all(|(i, e)| {
                e[0] as usize == i
                    && matches!(e[7], 0x13 | 0x14)
                    && [e[4], e[5]].iter().all(|n| n.is_power_of_two() && *n <= 256)
            })
    };
    (0..part.len() / 4)
        .rev()
        .map(|o| tex::channels(part.get(4 * o..).unwrap_or_default()))
        .find(fits)
        .unwrap_or_default()
}

/// The arenas' looping texture channels (0x3A7198 by the arena, table 0x519940; 0x3A7430: state 4, the frame + `speed`
/// / 256 a frame from `start`, back to 0 past the last): (arena, channel, speed, start).
pub const LOOPS: [(usize, usize, u32, u32); 18] = [
    (1, 0, 0x40, 0),
    (2, 0, 0xC0, 0),
    (3, 0, 0xC0, 0),
    (3, 1, 0xC0, 0),
    (3, 2, 0xC0, 0x10),
    (6, 0, 0x80, 0),
    (6, 1, 0x100, 0),
    (6, 2, 0x100, 0),
    (8, 0, 0x100, 0),
    (9, 0, 0x100, 0),
    (9, 1, 0x100, 8),
    (11, 0, 0x80, 0),
    (14, 0, 0x80, 0),
    (14, 1, 0x80, 0),
    (17, 0, 0x100, 0),
    (18, 0, 0x100, 0),
    (18, 1, 0x100, 8),
    (20, 0, 0x100, 0),
];

/// The arenas' texture channels that stand still unless a task of the arena plays them (0x3A7198): (arena,
/// channel, frame, cleared). Arena 4's lightning (0x3A7254: channels 0-3 on frame 1, cleared at the set-up, 4-6 on
/// frame 3; task 0x3A76C0 flashes them), arena 12's neon (three tasks 0x3A7590 from frame 0, now and then played
/// once): OpenKumite plays them (scene extras `channels` with their `channel`, stages.json `weather`).
pub const STILLS: [(usize, usize, usize, bool); 10] = [
    (4, 0, 1, true),
    (4, 1, 1, true),
    (4, 2, 1, true),
    (4, 3, 1, true),
    (4, 4, 3, false),
    (4, 5, 3, false),
    (4, 6, 3, false),
    (12, 0, 0, false),
    (12, 1, 0, false),
    (12, 2, 0, false),
];

/// A looping texture channel: the arena's uploads with each of its frames in place, its speed and start ([`LOOPS`]).
pub struct Anim {
    pub frames: Vec<Vec<Vec<tex::Upload>>>,
    pub speed: u32,
    pub start: u32,
    /// A still channel an arena task plays ([`STILLS`]): its number.
    pub channel: Option<usize>,
}

/// The lights the arena's VU1 blocks carry for its geometry: the first light record of geometry part 4 (after its
/// strips; a second one, where there is one, is the fighters', 0x455180's row): `u32 3, f32 ambient[3]`, then three
/// `{f32 towards[3], f32 colour[3]}`; the blocks have the colours / 128 and half the ambient (seen in PCSX2 on arenas
/// 4 and 8).
/// shortcut: the record is found by its shape, not by the part's header (its words 4-8 not decoded); upgrade when
/// a part has a look-alike before it
pub fn light(part: &[u8]) -> Option<Light> {
    (0..(part.len() + 1).saturating_sub(88)).step_by(4).find_map(|o| {
        (u32le(part, o)? == 3).then_some(())?;
        let v: Vec<f32> = (0..21).map(|k| f32le(part, o + 4 + 4 * k)).collect::<Option<_>>()?;
        let at = |k: usize| -> Option<[f32; 3]> { Some([*v.get(k)?, *v.get(k + 1)?, *v.get(k + 2)?]) };
        let ambient = at(0)?;
        let [Some(t0), Some(t1), Some(t2)] = [3, 9, 15].map(at) else { return None };
        let [Some(c0), Some(c1), Some(c2)] = [6, 12, 18].map(at) else { return None };
        let unit = |t: [f32; 3]| ((t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt() - 1.0).abs() < 0.01;
        let colour = |c: [f32; 3]| c.iter().all(|v| (0.0..=300.0).contains(v));
        ([t0, t1, t2].into_iter().all(unit) && [ambient, c0, c1, c2].into_iter().all(colour)).then_some(Light {
            toward: [t0, t1, t2],
            colour: [c0, c1, c2],
            ambient: ambient.map(|a| a / 2.0),
            fog: None,
        })
    })
}

/// Vertex `i`'s colour as the arena's VU1 microcode makes it (GS units, 128 = 1), and how its texture coordinates
/// are made: 0 as stored, 1 or 2 from its normal in the camera's axes (2: added to the stored ones), 8 from its
/// reflection, 4 (flags 4 and 0x400) from its reflection's bearing and height (a panorama). 0x54 dispatches on the batch's flags, the first of 8, 2, 1, 0x10, 4, 0x400, 0x20, 0x40, 0x100 set:
/// 8 (0x128) the stored colour, the reflection; 2 (0x195), 1 (0x16E), 4 and 0x400 (0x1BC) white (the I register,
/// 128, alpha too); 0x10 (0xB6) and 0x100 (0x102) the stored colour; 0x20 (0xDC) the stored colour + the ambient; 0x40
/// and none (0x8B) lit: the stored colour x the sum of each light's colour / 128 x max(0, n . towards it), + the
/// ambient; each clamped to 255 and truncated (FTOI0), the alpha the stored one (x the block's ambient w, 1).
pub fn colour(b: &Batch, i: usize, light: Option<&Light>) -> ([f32; 4], u8) {
    let c = b.col.get(i).copied().unwrap_or([128; 4]).map(f32::from);
    let f = b.flags;
    let first = [8, 2, 1, 0x10, 4, 0x400, 0x20, 0x40, 0x100].into_iter().find(|k| f & k != 0).unwrap_or(0x40);
    let env = match first {
        1 | 2 | 8 => first as u8,
        4 | 0x400 => 4,
        _ => 0,
    };
    // (TEX0's TFX 1, decal: the GS draws the texel as it is, the vertex colour and alpha unused)
    let decal = b.tex0 >> 35 & 3 == 1;
    let rgb = match (first, light) {
        _ if decal => return ([128.0; 4], env),
        (2 | 1 | 4 | 0x400, _) => return ([128.0; 4], env),
        (0x20, Some(l)) => [c[0] + l.ambient[0], c[1] + l.ambient[1], c[2] + l.ambient[2]],
        (0x40, Some(l)) => {
            let n = b.nrm.get(i).copied().unwrap_or_default();
            let mut sum = [0.0; 3];
            for (to, col) in l.toward.iter().zip(&l.colour) {
                let d = (n[0] * to[0] + n[1] * to[1] + n[2] * to[2]).max(0.0);
                sum = [sum[0] + col[0] / 128.0 * d, sum[1] + col[1] / 128.0 * d, sum[2] + col[2] / 128.0 * d];
            }
            [c[0] * sum[0] + l.ambient[0], c[1] * sum[1] + l.ambient[1], c[2] * sum[2] + l.ambient[2]]
        }
        _ => [c[0], c[1], c[2]],
    };
    let [r, g, b] = rgb.map(|v| v.clamp(0.0, 255.0).trunc());
    ([r, g, b, c[3]], env)
}

/// Every batch found in `d`, as stored.
pub fn batches(d: &[u8]) -> Vec<Batch> {
    // (both read inside `d`: the loops below check their reach first)
    let (word, byte) = (|o: usize| u32le(d, o).unwrap_or(0), |o: usize| d.get(o).copied().unwrap_or(0));
    let mut out = Vec::new();
    let mut p = 0;
    while p + 100 <= d.len() {
        let start = word(p) == 0x0100_0404
            && byte(p + 6) == 1
            && byte(p + 7) & 0x6f == 0x60
            && byte(p + 14) == 5
            && byte(p + 15) & 0x6f == 0x6c;
        if !start {
            p += 4;
            continue;
        }
        let m = p + 16; // the five quadwords
        let tag = word(m + 68);
        let blend = tag >> 15 & 0x40 != 0;
        // ALPHA register: 0x44 ordinary blend, 0x48 Cs*As + Cd, 0x89 Cd*As
        let mode = match (blend, byte(m + 16)) {
            (true, 0x48) => 1,
            (true, 0x89) => 2,
            (true, 0x44) => 3,
            _ => 0,
        };
        let tex0 = word(m + 48) as u64 | (word(m + 52) as u64) << 32;
        let mut b = Batch {
            tex0,
            flags: word(p + 8),
            mode,
            floor: false,
            fog: tag >> 15 & 0x20 != 0,
            bank: 0,
            pos: vec![],
            nrm: vec![],
            uv: vec![],
            col: vec![],
            cell: [0.0; 4],
        };
        let mut o = m + 80;
        while o + 4 <= d.len() {
            let (num, cmd) = (byte(o + 2) as usize, byte(o + 3) & 0x7f);
            if cmd == 1 {
                // STCYCL; 4,4 starts the next batch
                if word(o) == 0x0100_0404 {
                    break;
                }
                o += 4;
                continue;
            }
            if cmd < 0x60 {
                break;
            }
            let (comps, bits) = ((cmd as usize >> 2 & 3) + 1, 32 >> (cmd & 3));
            let size = (comps * bits / 8 * if num == 0 { 256 } else { num }).next_multiple_of(4);
            let Some(data) = d.get(o + 4..o + 4 + size) else { break };
            let f = |c: &[u8], o: usize| f32le(c, o).unwrap_or(0.0);
            match cmd & 0x6f {
                0x6e => b.col.extend(data.as_chunks::<4>().0),
                0x64 => b.uv.extend(data.as_chunks::<8>().0.iter().map(|c| [f(c, 0), f(c, 4)])),
                0x68 => {
                    for c in data.as_chunks::<24>().0 {
                        b.nrm.push([f(c, 0), f(c, 4), f(c, 8)]);
                        b.pos.push([f(c, 12), f(c, 16), f(c, 20)]);
                    }
                }
                _ => {}
            }
            o += 4 + size;
        }
        if b.pos.len() >= 3 && b.pos.iter().flatten().all(|v| v.is_finite() && v.abs() < 1e7) {
            out.push(b);
        }
        p = o.max(p + 4);
    }
    out
}

/// `banks[i]` are the texture uploads batches of bank `i` refer to.
pub fn glb(scenery: &[Batch], objects: &[Batch], banks: &[Vec<tex::Upload>]) -> Vec<u8> {
    let mut g = Glb::default();
    let (mut meshes, mut nodes) = (vec![], vec![]);
    let (mut materials, mut images, mut textures) = (vec![], vec![], vec![]);
    // (TEX0's texture, size and palette pointers, mode, floor, bank) -> (material, the image's part of the texture)
    #[allow(clippy::type_complexity)]
    let mut mat_of: HashMap<(u64, u8, bool, usize), (usize, [f32; 2])> = HashMap::new();
    for (name, batches) in [("arena", scenery), ("objects", objects)] {
        if batches.is_empty() {
            continue;
        }
        let (mut pos, mut nrm, mut uv, mut col) = (vec![], vec![], vec![], vec![]);
        let mut idx: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
        for b in batches {
            let t = tex::tex0(b.tex0);
            let (mat, k) =
                *mat_of.entry((b.tex0 & 0x000f_ffe7_ffff_ffff, b.mode, b.floor, b.bank)).or_insert_with(|| {
                    let mut pbr = json!({"metallicFactor": 0.0, "roughnessFactor": 1.0});
                    let mut k = [1.0, 1.0];
                    if let Some((w, h, px)) = banks.get(b.bank).and_then(|ups| tex::image(ups, &t, t.alpha)) {
                        let view = g.view(&tex::png(w, h, &px));
                        images.push(json!({"bufferView": view, "mimeType": "image/png"}));
                        textures.push(json!({"source": images.len() - 1, "sampler": 0}));
                        glb::set(&mut pbr, "baseColorTexture", json!({"index": textures.len() - 1}));
                        k = [t.tw as f32 / w as f32, t.th as f32 / h as f32];
                    }
                    let mut m = json!({"pbrMetallicRoughness": pbr, "doubleSided": true});
                    // billboards (trees, fences) are cut out by the texture's alpha
                    // glTF has no additive or multiply blending: the name tells the viewer (others see plain blending)
                    let name = match b.mode {
                        0 => b.floor.then_some("floor"),
                        1 => Some("additive"),
                        2 => Some("shade"),
                        _ => Some(if b.floor { "floor" } else { "blend" }),
                    };
                    glb::set(&mut m, "alphaMode", json!(if b.mode == 0 { "MASK" } else { "BLEND" }));
                    if b.mode == 0 {
                        glb::set(&mut m, "alphaCutoff", json!(0.1));
                    }
                    if let Some(name) = name {
                        glb::set(&mut m, "name", json!(name));
                    }
                    materials.push(m);
                    (materials.len() - 1, k)
                });
            let base = (pos.len() / 3) as u32;
            for (i, (p, n)) in b.pos.iter().zip(&b.nrm).enumerate() {
                pos.extend(p.map(|v| (v as f64 * SCALE) as f32));
                nrm.extend(n);
                let st = b.uv.get(i).copied().unwrap_or([0.; 2]);
                uv.extend([st[0] * k[0], st[1] * k[1]]);
                let c = b.col.get(i).copied().unwrap_or([128; 4]);
                col.extend([c[0], c[1], c[2]].map(|v| (v as u32 * 2).min(255) as u8));
                col.push(if b.mode > 0 { (c[3] as u32 * 2).min(255) as u8 } else { 255 });
            }
            let out = idx.entry(mat).or_default();
            for i in 0..(b.pos.len() as u32).saturating_sub(2) {
                out.extend(if i & 1 == 0 {
                    [base + i, base + i + 1, base + i + 2]
                } else {
                    [base + i + 1, base + i, base + i + 2]
                });
            }
        }
        let count = pos.len() / 3;
        let colour = g.accessor(&col, glb::U8, "VEC4", count);
        let attrs = json!({"POSITION": g.ranged(&pos, 3, "VEC3"), "NORMAL": g.accessor(&glb::floats(&nrm), glb::F32, "VEC3", count),
            "TEXCOORD_0": g.accessor(&glb::floats(&uv), glb::F32, "VEC2", count), "COLOR_0": colour});
        let prims: Vec<_> = (idx.into_iter())
            .map(|(mat, ix)| {
                let bytes: Vec<u8> = ix.iter().flat_map(|x| x.to_le_bytes()).collect();
                json!({"attributes": attrs, "indices": g.accessor(&bytes, glb::U32, "SCALAR", ix.len()), "material": mat})
            })
            .collect();
        meshes.push(json!({"name": name, "primitives": prims}));
        nodes.push(json!({"name": name, "mesh": meshes.len() - 1}));
    }
    let all: Vec<usize> = (0..nodes.len()).collect();
    let mut doc =
        json!({"scene": 0, "scenes": [{"nodes": all}], "nodes": nodes, "meshes": meshes, "materials": materials});
    if !images.is_empty() {
        glb::set(&mut doc, "images", json!(images));
        glb::set(&mut doc, "textures", json!(textures));
        // (bilinear without mipmaps, as the arena's TEX1 draws them: 0x61, MXL 0)
        glb::set(&mut doc, "samplers", json!([{"magFilter": 9729, "minFilter": 9729}]));
    }
    g.finish(doc)
}

/// The arena as OpenKumite's stages are laid out (the arcade converter's `write_game`), `name` its scene's name:
///
/// - node `sky`: the scenery, which the game keeps centred on the fighters; scene extras `drum: "model"`, and
///   `drum_cull: false` (the strips' winding is not stored: both sides are drawn), `drum_depth: "floor"` (the PS2
///   draws the arena with one z-buffer: the floor hides the ground below it, unlike the arcade stages' model).
/// - node `floor`: the floor's copies `floor_K`, and the scene extras' `floor_period` (m): the engine moves them by
///   whole periods. An arena's floors are one: the one of the longest period, with each shorter one laid inside
///   it (they divide it: 2, 4 or 8 m in 8 m). Its cells' levels: TEXCOORD_1 a vertex's cell's centre, TEXCOORD_2
///   (its level, 0 for every level; the floor's count of levels), and `levels` as the scene extras' `floor_levels` with the cells' size.
///
/// Every strip: the additive ones (lights, floor gloss) in materials named `additive`, the darkening ones (painted
/// shadows, dirt) in materials named `shade` (since format 14; glTF has neither blending). A batch without a texture draws the first image, a white texel;
/// colours (floats, 1 = 128: up to twice the texture) as the VU1 microcode makes them from the strips' with the arena's
/// `light` ([`colour`]; since format 44); a strip whose texture coordinates it makes from its normals in a material
/// with extras `env` (1, 2 or 8) and NORMAL, its picture whole.
/// - scene extras `channels` (`anims`, the looping texture channels): for each picture one changes, its `image`, the
///   `rect` (x, y, w, h) of it the frames change and the `frames` images from `first`, each that rectangle's texels,
///   played from `start` at `speed` / 256 a frame (0x3A6C80, state 4); a still one an arena task plays (`Anim::channel`)
///   with its `channel`, speed 0, resting on `start`. Channels sharing a picture each change their own rectangle.
#[allow(clippy::too_many_arguments)]
pub fn game_glb(
    name: &str,
    scenery: &[Batch],
    floors: &[Floor],
    objects: &[objects::Object],
    banks: &[Vec<tex::Upload>],
    levels: Option<Value>,
    light: Option<&Light>,
    anims: &[Anim],
) -> Vec<u8> {
    let mut g = Glb::default();
    let (mut sky, mut tiles): (Vec<Batch>, Vec<Batch>) = (scenery.to_vec(), vec![]);
    let floor = floors.iter().max_by(|a, b| (a.period[0] * a.period[1]).total_cmp(&(b.period[0] * b.period[1])));
    let window =
        levels.as_ref().map(|l| [0, 1].map(|k| l["window"].get(k).and_then(Value::as_u64).unwrap_or(0) as usize));
    let window = window.and_then(|w| windowed(floors, w));
    let windowed = window.is_some();
    let periodic = if windowed { &[][..] } else { floors };
    let (cells, places) = window.unwrap_or_default();
    for (f, big) in periodic.iter().zip(std::iter::repeat(floor).flatten()) {
        let n = [big.period[0] / f.period[0], big.period[1] / f.period[1]];
        if !n.iter().all(|n| (1.0..=64.0).contains(n) && (n - n.round()).abs() < 1e-3) {
            // shortcut: a floor whose period does not divide the other's (none seen) is laid once with the
            // scenery, upgrade with a floor node of its own
            sky.extend(f.laid());
            continue;
        }
        // its copies within one period of the other, each place of it once
        let [nx, nz] = n.map(|n| n.round() as i32);
        let at = (0..nx).flat_map(|x| (0..nz).map(move |z| [(x - nx / 2) as f32, (z - nz / 2) as f32]));
        let at = at.map(|[x, z]| [x * f.period[0], z * f.period[1]]).collect();
        tiles.extend(Floor { tiles: f.tiles.clone(), period: f.period, at, cell: f.cell }.laid());
    }
    // each picture once: (TEX0's texture, size and palette pointers, TCC, bank) -> the picture, its part of TW x TH,
    // and how far the batches' coordinates reach in it (in pictures: low x, y, high x, y); TCC kept apart: arena 6's
    // floor shades by a picture's alpha (0x89, TCC 1) that its added layer (0x48, TCC 0) takes opaque; a picture that
    // shades so (Cd x As) apart too, with its alpha as the GS's ([`tex::gs_image`]): above 0x80 it brightens (arena
    // 4's sky glow, 0x80-0xFC)
    let key = |b: &Batch| (b.tex0 & 0x000f_ffe7_ffff_ffff, b.bank, b.mode == 2 && tex::tex0(b.tex0).alpha);
    let read = |ups: &[tex::Upload], t: &tex::Tex0, gs: bool| {
        if gs { tex::gs_image(ups, t) } else { tex::image(ups, t, t.alpha) }
    };
    let st = |b: &Batch, i: usize| b.uv.get(i).copied().unwrap_or_default();
    let mut pics = BTreeMap::new();
    let models: Vec<Vec<Batch>> = objects.iter().map(|o| o.model.clone()).collect();
    for b in sky.iter().chain(&tiles).chain(cells.iter().flatten()).chain(models.iter().flatten()) {
        let pic = pics.entry(key(b)).or_insert_with(|| {
            let t = tex::tex0(b.tex0);
            let (w, h, px) = banks.get(b.bank).and_then(|u| read(u, &t, key(b).2)).filter(|p| p.0 > 0 && p.1 > 0)?;
            Some((w, h, px, [t.tw as f32 / w as f32, t.th as f32 / h as f32], [f32::MAX, f32::MAX, f32::MIN, f32::MIN]))
        });
        for (.., k, r) in pic.iter_mut() {
            // (coordinates made from normals reach all of it: OpenKumite makes them in the picture's)
            let made = (colour(b, 0, None).1 > 0).then_some([-1.0, 2.0]);
            for c in (0..b.pos.len()).map(|i| st(b, i)).chain(made) {
                *r = [r[0].min(c[0] * k[0]), r[1].min(c[1] * k[1]), r[2].max(c[0] * k[0]), r[3].max(c[1] * k[1])];
            }
        }
    }
    // the images: a white texel, then each picture cut to the texels its batches reach (all of it along an axis
    // they repeat it on): (image, its texture coordinates from the picture's)
    let mut images = vec![json!({"bufferView": g.view(&tex::png(1, 1, &[255; 4])), "mimeType": "image/png"})];
    let (mut place, mut animated) = (BTreeMap::new(), vec![]);
    for (k, pic) in &pics {
        let Some((w, h, px, scale, r)) = pic else { continue };
        // the texture channels over it: its frames, the first of them as it shows at first (the one as loaded is
        // overwritten at once); a picture they all leave the same stands so, one they change by frame is animated
        let t = tex::tex0(k.0);
        // (a clear texel's colour is not seen)
        let visible = |p: &[u8]| -> Vec<[u8; 4]> {
            p.as_chunks::<4>().0.iter().map(|&t| if t[3] == 0 { [0; 4] } else { t }).collect()
        };
        let mut frames = vec![];
        let mut px: std::borrow::Cow<[u8]> = px.into();
        for a in anims {
            let f: Vec<Vec<u8>> = (a.frames.iter())
                .filter_map(|ups| read(ups.get(k.1)?, &t, k.2).filter(|f| (f.0, f.1) == (*w, *h)).map(|f| f.2))
                .collect();
            let Some(first) = f.get(a.start as usize).filter(|_| f.len() == a.frames.len()) else { continue };
            if visible(first) == visible(&px) && f.iter().all(|f| visible(f) == visible(&px)) {
                continue;
            }
            if f.iter().all(|x| visible(x) == visible(first)) {
                // (a still channel rests as the uploads stand: a picture its frames all leave alike is not its own)
                if a.channel.is_none() {
                    px = first.clone().into();
                }
            } else {
                frames.push((f, a));
            }
        }
        // (each channel's first frame where its frames change the picture: channels share pictures)
        let loaded = px.to_vec();
        for (f, a) in &frames {
            let Some(first) = f.get(a.start as usize) else { continue };
            for i in (0..loaded.len()).step_by(4) {
                if f.iter().any(|x| x.get(i..i + 4) != loaded.get(i..i + 4))
                    && let (Some(d), Some(s)) = (px.to_mut().get_mut(i..i + 4), first.get(i..i + 4))
                {
                    d.copy_from_slice(s);
                }
            }
        }
        let span = |lo: f32, hi: f32, n: usize| {
            let inside = lo >= 0.0 && hi <= 1.0;
            let (a, b) = if inside { ((lo * n as f32) as usize, (hi * n as f32).ceil() as usize) } else { (0, n) };
            let (a, b) = (a.min(n - 1), b.clamp(a.min(n - 1) + 1, n));
            // (from pictures to the cut image: none along a repeated axis)
            (a..b, if inside { [n as f32 / (b - a) as f32, a as f32 / (b - a) as f32] } else { [1.0, 0.0] })
        };
        let ((x, fx), (y, fy)) = (span(r[0], r[2], *w), span(r[1], r[3], *h));
        let base = tex::cut(*w, *h, &px, x.clone(), y.clone());
        images.push(json!({"bufferView": g.view(&tex::png(x.len(), y.len(), &base)), "mimeType": "image/png"}));
        place.insert(*k, (images.len() - 1, *scale, [fx, fy]));
        // a channel's frames: the rectangle they change, each frame's texels there as an image after the others
        for (f, a) in &frames {
            let cuts: Vec<Vec<u8>> = f.iter().map(|px| tex::cut(*w, *h, px, x.clone(), y.clone())).collect();
            let cw = x.len();
            // (a texel clear in both is the same, whatever its colour)
            let same = |a: Option<&[u8]>, b: Option<&[u8]>| a == b || matches!((a, b), (Some([.., 0]), Some([.., 0])));
            let changed = |i: usize| cuts.iter().any(|c| !same(c.get(4 * i..4 * i + 4), base.get(4 * i..4 * i + 4)));
            let diff: Vec<(usize, usize)> =
                (0..base.len() / 4).filter(|&i| changed(i)).map(|i| (i % cw, i / cw)).collect();
            if let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
                diff.iter().map(|d| d.0).min(),
                diff.iter().map(|d| d.1).min(),
                diff.iter().map(|d| d.0).max(),
                diff.iter().map(|d| d.1).max(),
            ) {
                let image = images.len() - 1;
                for c in &cuts {
                    let rows: Vec<u8> = (y0..=y1)
                        .flat_map(|r| c.get(4 * (r * cw + x0)..4 * (r * cw + x1 + 1)).unwrap_or_default())
                        .copied()
                        .collect();
                    images.push(json!({"bufferView": g.view(&tex::png(x1 - x0 + 1, y1 - y0 + 1, &rows)), "mimeType": "image/png"}));
                }
                let mut c = json!({"image": image, "rect": [x0, y0, x1 - x0 + 1, y1 - y0 + 1], "first": image + 1,
                    "frames": cuts.len(), "speed": a.speed, "start": a.start});
                if let Some(k) = a.channel {
                    glb::set(&mut c, "channel", json!(k));
                }
                animated.push(c);
            }
        }
    }
    let textures: Vec<Value> = (0..images.len()).map(|i| json!({"source": i, "sampler": 0})).collect();
    // a material for each image, opaque (cut out by the texture's alpha) or blended; a primitive for each of a
    // mesh's materials, with vertices of its own
    let (mut materials, mut material) = (vec![], BTreeMap::new());
    let mut meshes = vec![];
    let parts = [("sky".to_string(), &sky), ("floor".to_string(), &tiles)].into_iter();
    let parts = parts.chain(cells.iter().enumerate().map(|(k, c)| (format!("floor_cell_{k}"), c)));
    for (name, batches) in parts.chain(models.iter().enumerate().map(|(k, m)| (format!("object_{k}"), m))) {
        // (positions, texture coordinates, colours, indices, floor cells, normals)
        type Prim = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>, Vec<f32>);
        let mut prims: BTreeMap<usize, Prim> = BTreeMap::new();
        for b in batches.iter() {
            let (image, scale, [fx, fy]) = place.get(&key(b)).copied().unwrap_or((0, [0.0; 2], [[1.0, 0.0]; 2]));
            let env = colour(b, 0, light).1;
            let fogged = b.fog && light.is_some_and(|l| l.fog.is_some());
            let mat = *material.entry((image, b.mode, env, fogged)).or_insert_with(|| {
                let pbr = json!({"baseColorTexture": {"index": image}, "metallicFactor": 0.0, "roughnessFactor": 1.0});
                let mut m =
                    json!({"pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "MASK", "alphaCutoff": 0.1});
                if b.mode > 0 {
                    m = json!({"pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"});
                }
                // glTF has neither added nor multiplying blending: the name tells OpenKumite
                if let Some(name) = [None, Some("additive"), Some("shade")].get(b.mode as usize).copied().flatten() {
                    glb::set(&mut m, "name", json!(name));
                }
                // (`gs_alpha`: its picture's alpha the GS's, 128 is 1)
                let extras: serde_json::Map<String, serde_json::Value> =
                    [("env", env > 0, json!(env)), ("fog", fogged, json!(1)), ("gs_alpha", key(b).2, json!(1))]
                        .into_iter()
                        .filter_map(|(k, on, v)| on.then(|| (k.to_string(), v)))
                        .collect();
                if !extras.is_empty() {
                    glb::set(&mut m, "extras", extras.into());
                }
                materials.push(m);
                materials.len() - 1
            });
            let (pos, uv, col, idx, cells, nrm) = prims.entry(mat).or_default();
            let base = (pos.len() / 3) as u32;
            for (i, p) in b.pos.iter().enumerate() {
                pos.extend(p.map(|v| (v as f64 * SCALE) as f32));
                let c = st(b, i);
                uv.extend([c[0] * scale[0] * fx[0] - fx[1], c[1] * scale[1] * fy[0] - fy[1]]);
                let c = colour(b, i, light).0;
                col.extend([c[0] / 128.0, c[1] / 128.0, c[2] / 128.0]);
                col.push(if b.mode > 0 { (c[3] * 2.0).min(255.0) / 255.0 } else { 1.0 });
                cells.extend([b.cell[0] * SCALE as f32, b.cell[1] * SCALE as f32, b.cell[2], b.cell[3]]);
                if env > 0 {
                    nrm.extend(b.nrm.get(i).copied().unwrap_or_default());
                }
            }
            for i in 0..(b.pos.len() as u32).saturating_sub(2) {
                let (a, c) = (base + i + (i & 1), base + i + 1 - (i & 1));
                idx.extend([a, c, base + i + 2]);
            }
        }
        if prims.is_empty() {
            continue;
        }
        let prims: Vec<Value> = (prims.into_iter())
            .map(|(mat, (pos, uv, col, idx, cells, nrm))| {
                let n = pos.len() / 3;
                let mut attrs = json!({"POSITION": g.ranged(&pos, 3, "VEC3"), "TEXCOORD_0": g.accessor(&glb::floats(&uv), glb::F32, "VEC2", n),
                    "COLOR_0": g.accessor(&glb::floats(&col), glb::F32, "VEC4", n)});
                if nrm.len() == pos.len() {
                    glb::set(&mut attrs, "NORMAL", json!(g.accessor(&glb::floats(&nrm), glb::F32, "VEC3", n)));
                }
                // the floor's cells: TEXCOORD_1 a vertex's cell's centre (x, z), TEXCOORD_2 (its level, the count)
                if name.starts_with("floor") {
                    let half = |k: usize| -> Vec<f32> { cells.chunks(2).skip(k).step_by(2).flatten().copied().collect() };
                    glb::set(&mut attrs, "TEXCOORD_1", json!(g.accessor(&glb::floats(&half(0)), glb::F32, "VEC2", n)));
                    glb::set(&mut attrs, "TEXCOORD_2", json!(g.accessor(&glb::floats(&half(1)), glb::F32, "VEC2", n)));
                }
                let bytes: Vec<u8> = idx.iter().flat_map(|x| x.to_le_bytes()).collect();
                json!({"attributes": attrs, "indices": g.accessor(&bytes, glb::U32, "SCALAR", idx.len()), "material": mat})
            })
            .collect();
        meshes.push(json!({"name": name, "primitives": prims}));
    }
    let mesh = |name: &str| meshes.iter().position(|m| m["name"] == name);
    let mut nodes = vec![json!({"name": "sky"})];
    if let (Some(sky), Some(node)) = (mesh("sky"), nodes.first_mut()) {
        glb::set(node, "mesh", json!(sky));
    }
    let mut extras = json!({"drum": "model", "drum_cull": false, "drum_depth": "floor", "background_rgb": [0, 0, 0]});
    if !animated.is_empty() {
        glb::set(&mut extras, "channels", json!(animated));
    }
    if let Some(f) = light.and_then(|l| l.fog) {
        glb::set(&mut extras, "fog", json!({"colour": f.colour, "line": f.line}));
    }
    if let Some(f) = floor.filter(|_| windowed || mesh("floor").is_some()) {
        if windowed {
            // the game's window: a node a cell, which the engine keeps on the fight
            let first = nodes.len() + 1;
            nodes.push(json!({"name": "floor", "children": (first..first + places.len()).collect::<Vec<usize>>()}));
            for (n, (k, [x, z])) in places.iter().enumerate() {
                let mut node =
                    json!({"name": format!("floor_{n}"), "translation": [*x as f64 * SCALE, 0.0, *z as f64 * SCALE]});
                if let Some(m) = mesh(&format!("floor_cell_{k}")) {
                    glb::set(&mut node, "mesh", json!(m));
                }
                nodes.push(node);
            }
        } else if let Some(tile) = mesh("floor") {
            let copies: Vec<usize> = (0..f.at.len()).map(|k| k + 2).collect();
            nodes.push(json!({"name": "floor", "children": copies}));
            for (k, [x, z]) in f.at.iter().enumerate() {
                nodes.push(json!({"name": format!("floor_{k}"), "mesh": tile, "translation": [*x as f64 * SCALE, 0.0, *z as f64 * SCALE]}));
            }
            glb::set(&mut extras, "floor_period", json!(f.period.map(|p| p as f64 * SCALE)));
        }
        // (the game's cells are the floor's, 0x340070's model)
        if let Some(mut levels) = levels {
            glb::set(&mut levels, "cell", json!(f.cell.map(|v| v as f64 * SCALE)));
            if !windowed {
                glb::set(&mut levels, "window", Value::Null);
            }
            glb::set(&mut extras, "floor_levels", levels);
        }
    }
    let mut roots: Vec<usize> = (0..nodes.len().min(2)).collect();
    // the objects: a node each (its model's mesh, its first pose), under its parent or the node `objects`; the ones
    // that move, an animation's channels (a key a frame, at 60 a second: the last one is where the first goes on)
    let first = nodes.len();
    let (mut samplers, mut channels) = (vec![], vec![]);
    for (k, o) in objects.iter().enumerate() {
        let Some(p) = o.poses.first() else { continue };
        let at = |p: &objects::Pose| p.at.map(|v| v * SCALE);
        let mut node = json!({"name": format!("object_{k}"), "translation": at(p), "rotation": objects::quat(p),
            "scale": p.scale});
        if let Some(m) = mesh(&format!("object_{k}")) {
            glb::set(&mut node, "mesh", json!(m));
        }
        if o.face {
            glb::set(&mut node, "extras", json!({"face": "camera"}));
        }
        let children: Vec<usize> =
            objects.iter().enumerate().filter(|c| c.1.parent == Some(k)).map(|(c, _)| first + c).collect();
        if !children.is_empty() {
            glb::set(&mut node, "children", json!(children));
        }
        nodes.push(node);
        if o.poses.len() > 1 {
            let times: Vec<f32> = (0..o.poses.len()).map(|t| t as f32 / 60.0).collect();
            let input = g.ranged(&times, 1, "SCALAR");
            let mut track = |path: &str, v: Vec<f32>, ty: &str| {
                let output = g.accessor(&glb::floats(&v), glb::F32, ty, o.poses.len());
                samplers.push(json!({"input": input, "output": output}));
                channels.push(json!({"sampler": samplers.len() - 1, "target": {"node": first + k, "path": path}}));
            };
            if o.poses.iter().any(|q| q.at != p.at) {
                track("translation", o.poses.iter().flat_map(|q| at(q).map(|v| v as f32)).collect(), "VEC3");
            }
            if o.poses.iter().any(|q| q.turn != p.turn) {
                track("rotation", o.poses.iter().flat_map(|q| objects::quat(q).map(|v| v as f32)).collect(), "VEC4");
            }
        }
    }
    if !objects.is_empty() {
        let top: Vec<usize> =
            objects.iter().enumerate().filter(|o| o.1.parent.is_none()).map(|(k, _)| first + k).collect();
        nodes.push(json!({"name": "objects", "children": top}));
        roots.push(nodes.len() - 1);
    }
    let mut doc = json!({"scene": 0, "scenes": [{"name": name, "nodes": roots, "extras": extras}], "nodes": nodes,
        "meshes": meshes, "materials": materials, "images": images, "textures": textures,
        "samplers": [{"magFilter": 9729, "minFilter": 9729}]});
    if !channels.is_empty() {
        glb::set(&mut doc, "animations", json!([{"name": "objects", "samplers": samplers, "channels": channels}]));
    }
    g.finish(doc)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn the_channel_table_at_part_4s_end() {
        // a light record's tail, the table (one PSMT4 channel, 64 x 64 at block 0x600), the padding
        let mut part = [1.0f32, 0.5, 0.25].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
        part.extend(1u32.to_le_bytes());
        part.extend([0u16, 0x600, 0, 0, 64, 64, 2, 0x14].iter().flat_map(|v| v.to_le_bytes()));
        part.extend([0xAB; 12]);
        assert_eq!(channel_table(&part), vec![[0, 0x600, 0, 0, 64, 64, 2, 0x14]]);
        assert!(channel_table(&part[..20]).is_empty());
    }

    /// One batch: a strip of `n` vertices at height `y`.
    fn batch(n: u8, y: f32) -> Vec<u8> {
        let mut d = vec![4, 4, 0, 1, 0, 0, 1, 0x60, 0, 0, 0, 0, 0, 0, 5, 0x6c];
        d.resize(16 + 80, 0);
        d.extend([0, 0, n, 0x6e]);
        d.extend((0..n).flat_map(|_| [128u8; 4]));
        d.extend([0, 0, n, 0x64]);
        d.extend((0..n).flat_map(|_| [0u8; 8]));
        d.extend([0, 0, 2 * n, 0x68]);
        for i in 0..n {
            let v = [0.0, 1.0, 0.0, (i / 2) as f32 * 100.0, y, (i % 2) as f32 * 100.0];
            d.extend(v.iter().flat_map(|x: &f32| x.to_le_bytes()));
        }
        d
    }

    #[test]
    fn batches_reads_strips() {
        let mut d = batch(4, 50.0);
        d.extend(batch(3, 60.0));
        let b = batches(&d);
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].pos.len(), b[0].nrm.len(), b[0].uv.len(), b[0].col.len()), (4, 4, 4, 4));
        assert_eq!((b[0].pos[3], b[1].pos[2]), ([100.0, 50.0, 100.0], [100.0, 60.0, 0.0]));
        let (scenery, objects, floors) = arena(&[(0, &d)]);
        assert_eq!((scenery.len(), objects.len(), floors.len()), (2, 0, 0));
        assert!(glb(&scenery, &objects, &[]).starts_with(b"glTF"));
        // cut anywhere, or noise: the whole strips come out, and nothing panics
        for n in 0..d.len() {
            let (s, o, f) = arena(&[(0, &d[..n])]);
            let _ = (glb(&s, &o, &[]), game_glb("", &s, &f, &[], &[], None, None, &[]));
        }
        let noise: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        let _ = arena(&[(0, &noise)]);
    }

    #[test]
    fn strips_are_lit_as_the_microcode_does() {
        // a light record after some strip data: light 0 straight up, white (128), the others away; ambient 40
        let mut part = vec![0u8; 12];
        part.extend(3u32.to_le_bytes());
        let rec = [40.0, 40.0, 40.0, 0.0, 1.0, 0.0, 128.0, 128.0, 128.0, 0.0, -1.0, 0.0, 9.0, 9.0, 9.0, 0.0, -1.0, 0.0];
        part.extend(rec.iter().chain(&[9.0, 9.0, 9.0]).flat_map(|v: &f32| v.to_le_bytes()));
        let l = light(&part).unwrap();
        assert_eq!((l.ambient, l.toward[0], l.colour[0]), ([20.0; 3], [0.0, 1.0, 0.0], [128.0; 3]));
        assert!(light(&part[..40]).is_none());
        // a strip facing the light: its colour x 1 + half the ambient; the alpha kept
        let mut b = batches(&batch(3, 0.0)).remove(0);
        b.col = vec![[100, 50, 0, 64]; 3];
        assert_eq!(colour(&b, 0, Some(&l)), ([120.0, 70.0, 20.0, 64.0], 0));
        // 0x10: as stored; 0x20: + the ambient; 1, 2, 8: their texture coordinates made, 1 and 2 white
        b.flags = 0x210;
        assert_eq!(colour(&b, 0, Some(&l)).0, [100.0, 50.0, 0.0, 64.0]);
        b.flags = 0x20;
        assert_eq!(colour(&b, 0, Some(&l)).0, [120.0, 70.0, 20.0, 64.0]);
        b.flags = 0x201;
        assert_eq!(colour(&b, 0, Some(&l)), ([128.0; 4], 1));
        b.flags = 0x600;
        assert_eq!(colour(&b, 0, Some(&l)), ([128.0; 4], 4));
        b.flags = 0x208;
        assert_eq!(colour(&b, 0, Some(&l)), ([100.0, 50.0, 0.0, 64.0], 8));
        // TFX 1 (decal): the texel as it is, lit or not
        b.flags = 0x200;
        b.tex0 |= 1 << 35;
        assert_eq!(colour(&b, 0, Some(&l)), ([128.0; 4], 0));
        // arena 8's fog row: FOGCOL grey, 255 at 14 m to 32 at 240 m: the blocks' 268.8 - 0.000987 w
        let mut elf = vec![0u8; FOG - VADDR + 24 * 9];
        let row = FOG - VADDR + 24 * 8;
        elf[row..row + 3].copy_from_slice(&[0x80; 3]);
        for (k, v) in [255.0f32, 14000.0, 32.0, 240000.0].iter().enumerate() {
            elf[row + 8 + 4 * k..row + 12 + 4 * k].copy_from_slice(&v.to_le_bytes());
        }
        let f = fog(&elf, 8).unwrap();
        assert!(f.colour == [0x80; 3] && (f.line[0] - 268.81).abs() < 0.01 && (f.line[1] + 0.000987).abs() < 1e-6);
        assert!(fog(&elf, 23).is_none());
    }

    #[test]
    fn a_floor_is_repeated_within_bounds() {
        // a grid of one flat cell, 4 m wide: the floor, repeated to 16 m around
        let grid = |w: f32| {
            let mut d = Vec::new();
            for v in [1u32, 0, 1, 1, w.to_bits(), w.to_bits(), 0, 0] {
                d.extend(v.to_le_bytes());
            }
            d.extend([4, 4, 0, 1, 0, 0, 0, 0x11, 0, 0, 0, 3, 0, 2, 0, 2]);
            d.extend(batch(4, 0.0));
            d
        };
        let (scenery, _, floors) = arena(&[(0, &grid(4000.0))]);
        assert!(scenery.is_empty() && floors.len() == 1);
        let laid: Vec<Batch> = floors[0].laid().collect();
        assert!(laid.len() > 9 && laid.iter().all(|b| b.floor));
        assert_eq!((floors[0].period, floors[0].at[0]), ([4000.0; 2], [-16000.0, -8000.0]));
        // a cell a hair wide would be repeated without end: kept once
        let (_, _, floors) = arena(&[(0, &grid(1e-20))]);
        assert_eq!(floors[0].laid().count(), 1);
        // levels 1 and 2 of a 2 x 1 grid of 2 m cells (level 0 never shows): the first cell's alike, kept once at
        // level 0; the second's both, tagged; each at its cell's centre
        let mut d = Vec::new();
        for v in [3u32, 0, 2, 1, 2000f32.to_bits(), 2000f32.to_bits(), 0, 0] {
            d.extend(v.to_le_bytes());
        }
        for (l, i) in (0..3).flat_map(|l| (0..2).map(move |i| (l, i))) {
            d.extend([4, 4, 0, 1, 0, 0, 0, 0x11, 0, 0, 0, 3, 0, 2, 0, 2]);
            d.extend(batch(if i == 1 && l == 2 { 6 } else { 4 }, 0.0));
        }
        let (_, _, floors) = arena(&[(0, &d)]);
        let cells: Vec<[f32; 4]> = floors[0].tiles.iter().map(|b| b.cell).collect();
        assert_eq!(cells, [[-1000.0, 0.0, 0.0, 3.0], [1000.0, 0.0, 1.0, 3.0], [1000.0, 0.0, 2.0, 3.0]]);
        assert_eq!(floors[0].cell, [2000.0; 2]);
        // laid as the game's window of 3 x 1 cells: cell j at (1.5 - j) x 2 m showing the floor's cell j mod 2,
        // each cell shown once, centred on the origin
        let (cells, places) = windowed(&floors, [3, 1]).unwrap();
        assert_eq!(places, [(0, [3000.0, -1000.0]), (1, [1000.0, -1000.0]), (0, [-1000.0, -1000.0])]);
        let levels: Vec<Vec<f32>> = cells.iter().map(|c| c.iter().map(|b| b.cell[2]).collect()).collect();
        assert_eq!(levels, [vec![0.0], vec![1.0, 2.0]]);
        assert!(cells.iter().flatten().all(|b| b.cell[..2] == [0.0, 0.0]));
        assert_eq!(cells[0][0].pos[0][0] - floors[0].tiles[0].pos[0][0], 1000.0);
        assert!(windowed(&floors, [0, 1]).is_none());
        // OpenKumite's stage: the two floors in one of 4 m, the 2 m one laid 2 x 2 in it; the added and
        // multiplying strips in materials of those names
        let (_, _, mut floors) = arena(&[(0, &grid(4000.0)), (0, &grid(2000.0))]);
        floors[1].tiles[0].mode = 1;
        let mut sky = batches(&batch(5, 3000.0));
        sky.extend(batches(&batch(3, 3000.0)).into_iter().map(|b| Batch { mode: 2, ..b }));
        let doc = gltf::Gltf::from_slice(&game_glb("Arena", &sky, &floors, &[], &[], None, None, &[])).unwrap();
        let extras: Value =
            serde_json::from_str(doc.scenes().next().unwrap().extras().as_ref().unwrap().get()).unwrap();
        assert_eq!(
            extras,
            json!({"drum": "model", "drum_cull": false, "drum_depth": "floor", "background_rgb": [0, 0, 0],
                "floor_period": [4.0, 4.0]})
        );
        let nodes: Vec<_> = doc.nodes().collect();
        assert_eq!(
            (nodes[0].name(), nodes[1].name(), nodes[1].children().count()),
            (Some("sky"), Some("floor"), floors[0].at.len())
        );
        let count = |m: gltf::Mesh| m.primitives().map(|p| p.get(&gltf::Semantic::Positions).unwrap().count()).max();
        assert_eq!(count(nodes[0].mesh().unwrap()), Some(5));
        assert_eq!(count(nodes[2].mesh().unwrap()), Some(4 * 4));
        let names: Vec<_> = doc.materials().filter_map(|m| m.name()).collect();
        assert_eq!(names, ["shade", "additive"]);
        assert_eq!(nodes[2].mesh().unwrap().primitives().next().unwrap().indices().unwrap().count(), 2 * 3);
        // the floor's cells: their centres and levels
        let prim = nodes[2].mesh().unwrap().primitives().next().unwrap();
        assert!(prim.get(&gltf::Semantic::TexCoords(1)).is_some() && prim.get(&gltf::Semantic::TexCoords(2)).is_some());
        assert!(nodes[0].mesh().unwrap().primitives().all(|p| p.get(&gltf::Semantic::TexCoords(1)).is_none()));
        // the untextured strips draw the white texel, the first image
        assert_eq!(doc.images().count(), 1);
        assert!(
            doc.materials().all(|m| m.pbr_metallic_roughness().base_color_texture().unwrap().texture().index() == 0)
        );
    }
}

//! Arena geometry: VIF packets for VU1. Each batch is one triangle strip in world space:
//!   STCYCL 4,4   UNPACK S1-32 x1   flags
//!                UNPACK V4-32 x5   GIF tag + ALPHA, CLAMP, TEX0, then the strip's own GIF tag
//!   STCYCL 4,1   UNPACK V4-8  xN   vertex colours (0x80 = 1.0)
//!   STCYCL 4,1   UNPACK V2-32 xN   texture coordinates
//!   STCYCL 4,2   UNPACK V3-32 x2N  normal, position
use crate::glb::{self, Glb};
use crate::model::SCALE;
use crate::{f32le, tex, u32le};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// The floor is repeated no further from the centre, and to no more vertices in all (the arenas' go 8 tiles out,
/// to 300 000 vertices): a broken grid cannot ask for more memory.
const REACH: i32 = 64;
const FLOOR_VERTICES: usize = 4 << 20;

#[derive(Clone)]
/// `mode`: 0 solid or cut-out, 1 adds to what is behind (lights, gloss), 2 multiplies it (shadow and
/// dirt decals), 3 ordinary blending (clouds, glass, soft edges).
pub struct Batch {
    pub tex0: u64,
    pub mode: u8,
    pub floor: bool,
    /// which texture bank its TEX0 refers to
    pub bank: usize,
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub col: Vec<[u8; 4]>,
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
/// the centre of their cell. Only the most detailed level is kept.
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
        let level = |l: usize| -> Vec<Batch> {
            let cell = |i: usize| {
                let k = l * cells + i;
                let block = starts.get(k).and_then(|&s| d.get(s..starts.get(k + 1).copied().unwrap_or(d.len())));
                let mut b = batches(block.unwrap_or_default());
                let (x, z) =
                    (((i % nx) as f32 - (nx as f32 - 1.0) / 2.0) * w, ((i / nx) as f32 - (nz as f32 - 1.0) / 2.0) * dz);
                for p in b.iter_mut().flat_map(|b| b.pos.iter_mut()) {
                    p[0] += x;
                    p[2] += z;
                }
                b
            };
            (0..cells).flat_map(cell).collect()
        };
        let mut best =
            (0..lods).map(level).max_by_key(|b| b.iter().map(|x| x.pos.len()).sum::<usize>()).unwrap_or_default();
        let (lo, hi) = bbox(best.iter().flat_map(|b| &b.pos));
        if cells > 1 || hi[1] < 700.0 {
            for b in &mut best {
                b.floor = true;
            }
            floors.push((best, nx as f32 * w, nz as f32 * dz));
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
    for (tiles, px, pz) in floors {
        let radius = if hole < 60000.0 { (hole * 1.5).clamp(16000.0, 40000.0) } else { 16000.0 };
        let reach = (radius / px.min(pz)).ceil() as i32;
        let vertices: usize = tiles.iter().map(|b| b.pos.len()).sum();
        if reach > REACH || vertices.saturating_mul((2 * reach as usize + 1).pow(2)) > FLOOR_VERTICES {
            // not a floor the game has: as it is, once
            laid.push(Floor { tiles, period: [px, pz], at: vec![[0.; 2]] });
            continue;
        }
        let at = (-reach..=reach).flat_map(|x| (-reach..=reach).map(move |z| (x, z))).filter_map(|(ix, iz)| {
            let (ox, oz) = (ix as f32 * px, iz as f32 * pz);
            let far = libm::hypotf((ox.abs() - px / 2.0).max(0.0), (oz.abs() - pz / 2.0).max(0.0)) >= radius;
            ((ix, iz) == (0, 0) || !far).then_some([ox, oz])
        });
        laid.push(Floor { tiles, period: [px, pz], at: at.collect() });
    }
    (scenery, objects, laid)
}

/// A floor of an arena: its tiles (at the centre), how far apart their copies are, and where they go (x, z), to a
/// little past the hole in the far ground.
pub struct Floor {
    pub tiles: Vec<Batch>,
    pub period: [f32; 2],
    pub at: Vec<[f32; 2]>,
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
                b
            })
        })
    }
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
        let mut b = Batch { tex0, mode, floor: false, bank: 0, pos: vec![], nrm: vec![], uv: vec![], col: vec![] };
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
                *mat_of.entry((b.tex0 & 0x000f_ffe3_ffff_ffff, b.mode, b.floor, b.bank)).or_insert_with(|| {
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
        glb::set(&mut doc, "samplers", json!([{"magFilter": 9729, "minFilter": 9987}]));
    }
    g.finish(doc)
}

/// The arena as OpenKumite's stages are laid out (the arcade converter's `write_game`), `name` its scene's name:
///
/// - node `sky`: the scenery, which the game keeps centred on the fighters; scene extras `drum: "model"`, and
///   `drum_cull: false` (the strips' winding is not stored: both sides are drawn).
/// - node `floor`: the floor's copies `floor_K`, and the scene extras' `floor_period` (m): the engine moves them by
///   whole periods. An arena's floors are one: the one of the longest period, with each shorter one laid inside
///   it (they divide it: 2, 4 or 8 m in 8 m).
///
/// Only the opaque, cut-out and ordinarily blended strips: the additive (lights, floor gloss) and darkening ones
/// are left out (OpenKumite's stages draw neither). A batch without a texture draws the first image, a white texel;
/// colours are the strips' vertex colours.
pub fn game_glb(name: &str, scenery: &[Batch], floors: &[Floor], banks: &[Vec<tex::Upload>]) -> Vec<u8> {
    let mut g = Glb::default();
    let drawn = |b: &Batch| b.mode == 0 || b.mode == 3;
    let (mut sky, mut tiles): (Vec<Batch>, Vec<Batch>) =
        (scenery.iter().filter(|b| drawn(b)).cloned().collect(), vec![]);
    let floor = floors.iter().max_by(|a, b| (a.period[0] * a.period[1]).total_cmp(&(b.period[0] * b.period[1])));
    for (f, big) in floors.iter().zip(std::iter::repeat(floor).flatten()) {
        let n = [big.period[0] / f.period[0], big.period[1] / f.period[1]];
        if !n.iter().all(|n| (1.0..=64.0).contains(n) && (n - n.round()).abs() < 1e-3) {
            // shortcut: a floor whose period does not divide the other's (none seen) is laid once with the
            // scenery, upgrade with a floor node of its own
            sky.extend(f.laid().filter(drawn));
            continue;
        }
        // its copies within one period of the other, each place of it once
        let [nx, nz] = n.map(|n| n.round() as i32);
        let at = (0..nx).flat_map(|x| (0..nz).map(move |z| [(x - nx / 2) as f32, (z - nz / 2) as f32]));
        let at = at.map(|[x, z]| [x * f.period[0], z * f.period[1]]).collect();
        tiles.extend(Floor { tiles: f.tiles.clone(), period: f.period, at }.laid().filter(drawn));
    }
    // each picture once: (TEX0's texture, size and palette pointers, bank) -> the picture, its part of TW x TH, and
    // how far the batches' coordinates reach in it (in pictures: low x, y, high x, y)
    let key = |b: &Batch| (b.tex0 & 0x000f_ffe3_ffff_ffff, b.bank);
    let st = |b: &Batch, i: usize| b.uv.get(i).copied().unwrap_or_default();
    let mut pics = BTreeMap::new();
    for b in sky.iter().chain(&tiles) {
        let pic = pics.entry(key(b)).or_insert_with(|| {
            let t = tex::tex0(b.tex0);
            let (w, h, px) =
                banks.get(b.bank).and_then(|u| tex::image(u, &t, t.alpha)).filter(|p| p.0 > 0 && p.1 > 0)?;
            Some((w, h, px, [t.tw as f32 / w as f32, t.th as f32 / h as f32], [f32::MAX, f32::MAX, f32::MIN, f32::MIN]))
        });
        for (.., k, r) in pic.iter_mut() {
            for c in (0..b.pos.len()).map(|i| st(b, i)) {
                *r = [r[0].min(c[0] * k[0]), r[1].min(c[1] * k[1]), r[2].max(c[0] * k[0]), r[3].max(c[1] * k[1])];
            }
        }
    }
    // the images: a white texel, then each picture cut to the texels its batches reach (all of it along an axis
    // they repeat it on): (image, its texture coordinates from the picture's)
    let mut images = vec![json!({"bufferView": g.view(&tex::png(1, 1, &[255; 4])), "mimeType": "image/png"})];
    let mut place = BTreeMap::new();
    for (k, pic) in &pics {
        let Some((w, h, px, scale, r)) = pic else { continue };
        let span = |lo: f32, hi: f32, n: usize| {
            let inside = lo >= 0.0 && hi <= 1.0;
            let (a, b) = if inside { ((lo * n as f32) as usize, (hi * n as f32).ceil() as usize) } else { (0, n) };
            let (a, b) = (a.min(n - 1), b.clamp(a.min(n - 1) + 1, n));
            // (from pictures to the cut image: none along a repeated axis)
            (a..b, if inside { [n as f32 / (b - a) as f32, a as f32 / (b - a) as f32] } else { [1.0, 0.0] })
        };
        let ((x, fx), (y, fy)) = (span(r[0], r[2], *w), span(r[1], r[3], *h));
        let png = tex::png(x.len(), y.len(), &tex::cut(*w, *h, px, x.clone(), y.clone()));
        images.push(json!({"bufferView": g.view(&png), "mimeType": "image/png"}));
        place.insert(*k, (images.len() - 1, *scale, [fx, fy]));
    }
    let textures: Vec<Value> = (0..images.len()).map(|i| json!({"source": i, "sampler": 0})).collect();
    // a material for each image, opaque (cut out by the texture's alpha) or blended; a primitive for each of a
    // mesh's materials, with vertices of its own
    let (mut materials, mut material) = (vec![], BTreeMap::new());
    let mut meshes = vec![];
    for (name, batches) in [("sky", &sky), ("floor", &tiles)] {
        // (positions, texture coordinates, colours, indices)
        type Prim = (Vec<f32>, Vec<f32>, Vec<u8>, Vec<u32>);
        let mut prims: BTreeMap<usize, Prim> = BTreeMap::new();
        for b in batches.iter() {
            let (image, scale, [fx, fy]) = place.get(&key(b)).copied().unwrap_or((0, [0.0; 2], [[1.0, 0.0]; 2]));
            let mat = *material.entry((image, b.mode)).or_insert_with(|| {
                let pbr = json!({"baseColorTexture": {"index": image}, "metallicFactor": 0.0, "roughnessFactor": 1.0});
                let mut m =
                    json!({"pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "MASK", "alphaCutoff": 0.1});
                if b.mode == 3 {
                    m = json!({"pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"});
                }
                materials.push(m);
                materials.len() - 1
            });
            let (pos, uv, col, idx) = prims.entry(mat).or_default();
            let base = (pos.len() / 3) as u32;
            for (i, p) in b.pos.iter().enumerate() {
                pos.extend(p.map(|v| (v as f64 * SCALE) as f32));
                let c = st(b, i);
                uv.extend([c[0] * scale[0] * fx[0] - fx[1], c[1] * scale[1] * fy[0] - fy[1]]);
                let c = b.col.get(i).copied().unwrap_or([128; 4]);
                col.extend([c[0], c[1], c[2]].map(|v| (v as u32 * 2).min(255) as u8));
                col.push(if b.mode > 0 { (c[3] as u32 * 2).min(255) as u8 } else { 255 });
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
            .map(|(mat, (pos, uv, col, idx))| {
                let n = pos.len() / 3;
                let attrs = json!({"POSITION": g.ranged(&pos, 3, "VEC3"), "TEXCOORD_0": g.accessor(&glb::floats(&uv), glb::F32, "VEC2", n),
                    "COLOR_0": g.accessor(&col, glb::U8, "VEC4", n)});
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
    let mut extras = json!({"drum": "model", "drum_cull": false, "background_rgb": [0, 0, 0]});
    if let (Some(f), Some(tile)) = (floor, mesh("floor")) {
        let copies: Vec<usize> = (0..f.at.len()).map(|k| k + 2).collect();
        nodes.push(json!({"name": "floor", "children": copies}));
        for (k, [x, z]) in f.at.iter().enumerate() {
            nodes.push(json!({"name": format!("floor_{k}"), "mesh": tile, "translation": [*x as f64 * SCALE, 0.0, *z as f64 * SCALE]}));
        }
        glb::set(&mut extras, "floor_period", json!(f.period.map(|p| p as f64 * SCALE)));
    }
    let roots: Vec<usize> = (0..nodes.len().min(2)).collect();
    g.finish(json!({"scene": 0, "scenes": [{"name": name, "nodes": roots, "extras": extras}], "nodes": nodes,
        "meshes": meshes, "materials": materials, "images": images, "textures": textures,
        "samplers": [{"magFilter": 9729, "minFilter": 9987}]}))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

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
            let _ = (glb(&s, &o, &[]), game_glb("", &s, &f, &[]));
        }
        let noise: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        let _ = arena(&[(0, &noise)]);
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
        // OpenKumite's stage: the two floors in one of 4 m, the 2 m one laid 2 x 2 in it, the additive strip out
        let (_, _, mut floors) = arena(&[(0, &grid(4000.0)), (0, &grid(2000.0))]);
        floors[1].tiles[0].mode = 1;
        let mut sky = batches(&batch(5, 3000.0));
        sky.extend(batches(&batch(3, 3000.0)).into_iter().map(|b| Batch { mode: 2, ..b }));
        let doc = gltf::Gltf::from_slice(&game_glb("Arena", &sky, &floors, &[])).unwrap();
        let extras: Value =
            serde_json::from_str(doc.scenes().next().unwrap().extras().as_ref().unwrap().get()).unwrap();
        assert_eq!(
            extras,
            json!({"drum": "model", "drum_cull": false, "background_rgb": [0, 0, 0], "floor_period": [4.0, 4.0]})
        );
        let nodes: Vec<_> = doc.nodes().collect();
        assert_eq!(
            (nodes[0].name(), nodes[1].name(), nodes[1].children().count()),
            (Some("sky"), Some("floor"), floors[0].at.len())
        );
        let count = |m: gltf::Mesh| m.primitives().map(|p| p.get(&gltf::Semantic::Positions).unwrap().count()).max();
        assert_eq!(count(nodes[0].mesh().unwrap()), Some(5));
        assert_eq!(count(nodes[2].mesh().unwrap()), Some(4));
        assert_eq!(nodes[2].mesh().unwrap().primitives().next().unwrap().indices().unwrap().count(), 2 * 3);
        // the untextured strips draw the white texel, the first image
        assert_eq!(doc.images().count(), 1);
        assert!(
            doc.materials().all(|m| m.pbr_metallic_roughness().base_color_texture().unwrap().texture().index() == 0)
        );
    }
}

//! Arena geometry: VIF packets for VU1. Each batch is one triangle strip in world space:
//!   STCYCL 4,4   UNPACK S1-32 x1   flags
//!                UNPACK V4-32 x5   GIF tag + ALPHA, CLAMP, TEX0, then the strip's own GIF tag
//!   STCYCL 4,1   UNPACK V4-8  xN   vertex colours (0x80 = 1.0)
//!   STCYCL 4,1   UNPACK V2-32 xN   texture coordinates
//!   STCYCL 4,2   UNPACK V3-32 x2N  normal, position
use crate::glb::{self, Glb};
use crate::model::SCALE;
use crate::{f32le, tex, u32le};
use serde_json::json;
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
fn bbox<'a>(points: impl Iterator<Item = &'a [f32; 3]>) -> ([f32; 3], [f32; 3]) {
    points.fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), p| {
        (std::array::from_fn(|k| lo[k].min(p[k])), std::array::from_fn(|k| hi[k].max(p[k])))
    })
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
pub fn arena(parts: &[(usize, &[u8])]) -> (Vec<Batch>, Vec<Batch>) {
    const CELL: [u8; 16] = [4, 4, 0, 1, 0, 0, 0, 0x11, 0, 0, 0, 3, 0, 2, 0, 2];
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
    for (tiles, px, pz) in floors {
        let radius = if hole < 60000.0 { (hole * 1.5).clamp(16000.0, 40000.0) } else { 16000.0 };
        let reach = (radius / px.min(pz)).ceil() as i32;
        let vertices: usize = tiles.iter().map(|b| b.pos.len()).sum();
        if reach > REACH || vertices.saturating_mul((2 * reach as usize + 1).pow(2)) > FLOOR_VERTICES {
            scenery.extend(tiles); // not a floor the game has: as it is, once
            continue;
        }
        for (ix, iz) in (-reach..=reach).flat_map(|x| (-reach..=reach).map(move |z| (x, z))) {
            let (ox, oz) = (ix as f32 * px, iz as f32 * pz);
            if (ix, iz) != (0, 0)
                && libm::hypotf((ox.abs() - px / 2.0).max(0.0), (oz.abs() - pz / 2.0).max(0.0)) >= radius
            {
                continue;
            }
            scenery.extend(tiles.iter().cloned().map(|mut b| {
                for p in &mut b.pos {
                    p[0] += ox;
                    p[2] += oz;
                }
                b
            }));
        }
    }
    (scenery, objects)
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
        let (scenery, objects) = arena(&[(0, &d)]);
        assert_eq!((scenery.len(), objects.len()), (2, 0));
        assert!(glb(&scenery, &objects, &[]).starts_with(b"glTF"));
        // cut anywhere, or noise: the whole strips come out, and nothing panics
        for n in 0..d.len() {
            let (s, o) = arena(&[(0, &d[..n])]);
            let _ = glb(&s, &o, &[]);
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
        let (scenery, _) = arena(&[(0, &grid(4000.0))]);
        assert!(scenery.len() > 9 && scenery.iter().all(|b| b.floor));
        // a cell a hair wide would be repeated without end: kept once
        let (scenery, _) = arena(&[(0, &grid(1e-20))]);
        assert_eq!(scenery.len(), 1);
    }
}

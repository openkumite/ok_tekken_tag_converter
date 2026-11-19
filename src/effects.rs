//! What a fight draws besides the fighters and the arena (docs/EFFECTS.md): the fighters' shadow meshes
//! (`gamedata/shadows.json`), each arena's shadow (`gamedata/stages.json`) and the hit and move effects' pictures
//! (`pictures/effects/`), in the layouts OpenKumite reads Tekken 3's in.

use crate::model::{Model, RIG, SCALE};
use crate::moves::{self, VADDR};
use crate::{archive, f32le, tex, u16le, u32le};
use serde_json::{Value, json};

/// The arenas' lights (0x54 bytes an arena: light 0's direction first, floats) and the shadow's alpha (u16, of 0x80).
const LIGHTS: usize = 0x41_6288;
const ALPHA: usize = 0x46_B280;
/// The arenas' dust (12 bytes an arena: u32 particles, f32 scale, u8 r, g, b; 0x3A2758's kind 3).
const DUST: usize = 0x43_7AC0;
const ARENAS: usize = 23;
/// The lens flare (0x3A1C78, a task of every arena): the jump table by arena - 1 (16), its cases (one a row of the
/// suns 0x437A50, 16 bytes: x, y, z, 1; and of the strengths gp-0x7EB8, floats) and the case of none; the nine discs
/// (0x5196C0, 12 bytes: u32 picture of file 9 (disc k's is the word before its row's size), f32 size, f32 brightness).
const FLARE_CASES: usize = 0x51_9730;
const FLARE_CASE0: u32 = 0x3A_1CEC;
const SUNS: usize = 0x43_7A50;
const DISCS: usize = 0x51_96C0;
const GP: usize = 0x5F_D0F0;

/// A fighter's shadow mesh (the Tekken 3 converter's `shadows.json` entry) from its low-detail model, fighter-pack
/// part 8, which the game draws into its shadow picture (0x3424A8): each vertex on its heavier joint, in that
/// joint's axes (metres), the strips as triangles. `record`: the character's model record (its file).
pub fn shadow(m: &Model, record: usize) -> Value {
    let (mut vertices, mut triangles) = (vec![], vec![]);
    for s in m.sections.iter().flatten() {
        let ids: Vec<Option<u32>> = (s.verts.iter())
            .map(|(_, inf)| {
                let i = inf.first()?;
                // a joint past the 18 (hair, cloth) carried by its parent: its offset added (the low model's joints
                // are not turned)
                let (mut node, mut p) = (i.node, i.p);
                for _ in 0..m.nodes.len() {
                    if node < RIG.len() {
                        break;
                    }
                    let n = m.nodes.get(node)?;
                    p = [p[0] + n.pos[0], p[1] + n.pos[1], p[2] + n.pos[2]];
                    node = usize::try_from(n.parent).ok()?;
                }
                let [x, y, z] = p.map(|x| x as f64 * SCALE);
                (node < RIG.len()).then_some(())?;
                vertices.push(json!([node, x, y, z]));
                Some(vertices.len() as u32 - 1)
            })
            .collect();
        // (a vertex without a joint: none of the strip's triangles through it)
        triangles.extend(ids.windows(3).filter_map(|t| t.iter().copied().collect::<Option<Vec<u32>>>()));
    }
    json!({"models": [record], "joints": RIG.map(|r| r.0), "vertices": vertices, "triangles": triangles, "quads": []})
}

/// `gamedata/stages.json`: each arena's shadow as the Tekken 3 converter writes a stage without a light table, plus
/// its alpha: towards light 0 (the arena's axes, y up; turned up when it points down: the shadow falls along the
/// line either way), black, at the arena's alpha (0x342360); and the fighters' `lights` (0x353918 copies them into
/// each side's light block): towards each of the three lights (the arena's axes, y up), their colours and the
/// ambient (128 = 1, the GS's modulation); and its `dust` (0x3A2758, kind 3 at the floor): so many particles, their
/// size times `scale`, their colour (128 = 1); its `people`, the background characters (0x114FC8, FORMATS.md 12):
/// `model` (`stages/stage_NN_pK.glb`), `at` (metres, the arena's axes), `heading` (degrees about y, 0 = +z), the
/// `motions` played in turn (`stage_NN_mK`), and the frame of them the first one `start`s at; its `flare` (0x3A1C78:
/// none on most arenas): the `sun` (metres, the arena's axes), the `strength` and the nine `discs` [picture of
/// `pictures/effects/flare_K.png`, size, brightness].
pub fn stages(elf: &[u8]) -> Value {
    let discs: Vec<Value> = (0..9)
        .filter_map(|k| {
            let o = DISCS + 12 * k - VADDR;
            Some(json!([u32le(elf, o)?, f32le(elf, o + 4)?, f32le(elf, o + 8)?]))
        })
        .collect();
    let flare = |a: usize| -> Option<Value> {
        let case = u32le(elf, FLARE_CASES + 4 * a.checked_sub(1)? - VADDR).filter(|_| a <= 16)?;
        let row = (case.checked_sub(FLARE_CASE0)? as usize)
            .is_multiple_of(0x10)
            .then(|| (case - FLARE_CASE0) as usize / 0x10)?;
        (row < 7).then_some(())?;
        let sun: Vec<f64> = (0..3)
            .map(|k| f32le(elf, SUNS + 16 * row + 4 * k - VADDR).map(|v| v as f64 * SCALE))
            .collect::<Option<_>>()?;
        Some(json!({"sun": sun, "strength": f32le(elf, GP - 0x7EB8 + 4 * row - VADDR)?, "discs": discs}))
    };
    let stages: Vec<Value> = (0..ARENAS)
        .filter_map(|a| {
            let f: Vec<f64> =
                (0..21).map(|k| f32le(elf, LIGHTS + 0x54 * a + 4 * k - VADDR).map(f64::from)).collect::<Option<_>>()?;
            let v = |k: usize| f.get(3 * k..3 * k + 3).unwrap_or_default();
            let (x, y, z) = (f.first()?, f.get(1)?, f.get(2)?);
            let s = if *y < 0.0 { -1.0 } else { 1.0 };
            let alpha = u16le(elf, ALPHA + 2 * a - VADDR)?;
            let row = DUST + 12 * a - VADDR;
            let dust = json!({"count": u32le(elf, row)?, "scale": f32le(elf, row + 4)?,
                "colour": elf.get(row + 8..row + 11)?});
            let people: Vec<Value> = (moves::cast(elf, a).unwrap_or_default().iter())
                .map(|p| json!({"model": p.model, "at": p.pos.map(|v| (v as f64 * SCALE) as f32), "heading": p.heading, "motions": p.motions, "start": p.start}))
                .collect();
            Some(json!({"id": a, "shadow": {"toward": [x * s, y * s, z * s], "colour": [0, 0, 0], "alpha": alpha as f64 / 128.0},
                "lights": {"toward": [v(0), v(1), v(2)], "colour": [v(3), v(4), v(5)], "ambient": v(6)}, "dust": dust, "people": people, "flare": flare(a)}))
        })
        .collect();
    json!({"stages": stages})
}

/// The atlas's square cells: the largest picture's size (a sparkle's lone texels would not survive shrinking).
const CELL: usize = 128;
const COLS: usize = 8;

/// An effect file's particle paths (part 1, a pack: EFFECTS.md 3): each `u32 frames`, the 100 mm quad's corners,
/// then from +0x38 20 bytes a frame: the position (metres, the arena's axes, from where the effect starts), the
/// quad's size (x 0x90 / 0x48 of the effect, 0x345C10) and its fade (the brightness x 1 - it, 0x345E2C).
pub fn paths(file: &[u8]) -> Vec<Vec<[f64; 5]>> {
    let part = archive::pack(file).and_then(|p| archive::pack(p.get(1)?)).unwrap_or_default();
    (part.iter())
        .filter_map(|e| {
            let frames = (u32le(e, 0)? as usize).min(1000);
            let at = |o: usize| f32le(e, o).map(f64::from).filter(|v| v.is_finite());
            (0..frames)
                .map(|k| 0x38 + 20 * k)
                .map(|o| Some([at(o)? * SCALE, at(o + 4)? * SCALE, at(o + 8)? * SCALE, at(o + 12)?, at(o + 16)?]))
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

/// `pictures/effects/atlas.png` and `effects.json` (the Tekken 3 converter's layout) from the effect files'
/// pictures `pics[k]` (files 4 + k, k 0-4: the four sets' and file 8's 20 fireball frames): the hit spark (set 0),
/// the zero-damage spark (set 3), the guard spark (set 1), the dust (set 2), each its set's frame count; the heavy
/// hits' fireballs, orange and blue (the charged attack's). `sets[k]`: file 4 + k's particle paths, each effect's
/// `bursts` (0x3A2758: a particle on each path, the dust's on paths drawn at random; table 0x436688 rows 0x24-0x26
/// scale the paths and give the brightness, 0x44 of 128; the quad 0x90 = 128 over 0x48 = 400 metres at size 1).
/// None when a picture is missing.
pub fn atlas(pics: &[Vec<(usize, usize, Vec<u8>)>], sets: &[Vec<Vec<[f64; 5]>>]) -> Option<(Vec<u8>, Value)> {
    let cells: Vec<&(usize, usize, Vec<u8>)> =
        (0..4).map(|k| pics.get(k)?.first()).chain(pics.get(4)?.iter().take(20).map(Some)).collect::<Option<_>>()?;
    if cells.len() != 24 {
        return None;
    }
    let rows = cells.len().div_ceil(COLS);
    let (w, h) = (COLS * CELL, rows * CELL);
    let mut rgba = vec![0u8; w * h * 4];
    for (n, (pw, ph, px)) in cells.iter().enumerate() {
        let (cx, cy) = (n % COLS * CELL, n / COLS * CELL);
        // each picture scaled up to the cell (nearest texel)
        for (y, x) in (0..CELL).flat_map(|y| (0..CELL).map(move |x| (y, x))) {
            let at = (y * ph / CELL * pw + x * pw / CELL) * 4;
            let to = ((cy + y) * w + cx + x) * 4;
            if let (Some(s), Some(d)) = (px.get(at..at + 4), rgba.get_mut(to..to + 4)) {
                d.copy_from_slice(s);
            }
        }
    }
    let strip = |cell: usize, life: usize, blend: &str, note: &str| json!({"cells": vec![cell; life], "life": life, "blend": blend, "note": note});
    let round = |v: f64| (v * 10000.0).round() / 10000.0;
    let burst = |cell: usize, scale: f64, level: f64, random: bool, set: usize| {
        let frame = |f: &[f64; 5]| [f[0] * scale, f[1] * scale, f[2] * scale, f[3], f[4]].map(round);
        let paths: Vec<Vec<[f64; 5]>> =
            sets.get(set).into_iter().flatten().map(|p| p.iter().map(frame).collect()).collect();
        json!({"cell": cell, "size": 128.0 / 400.0, "level": level, "random": random, "paths": paths})
    };
    let ramp = |first: usize, note: &str| json!({"cells": (first..first + 10).collect::<Vec<_>>(), "life": 0, "blend": "add", "note": note});
    let json = json!({
        "atlas": {"file": "atlas.png", "cell": CELL, "cols": COLS, "rows": rows},
        "strips": {
            "spark": strip(0, 35, "add", "every damaging hit: kind 0, set 0 (file 4)"),
            "zero": strip(1, 30, "add", "op 9: kind 1, set 3 (file 5)"),
            "block": strip(2, 40, "add", "a guarded hit: kind 2, set 1 (file 6)"),
            "dust": strip(3, 150, "half", "op 0xA, ground impacts: kind 3, set 2 (file 7)"),
            "heavy_a": ramp(4, "damage 21 and more: file 8's orange fireballs (0x3A6110 0x80 / 0x81)"),
            "heavy_b": ramp(14, "the charged attack's: file 8's blue fireballs (0x8B / 0x8C)"),
        },
        "bursts": {
            "spark": burst(0, 0.5, 1.0, false, 0),
            "zero": burst(1, 1.0, 1.0, false, 1),
            "block": burst(2, 1.0, 0.25, false, 2),
            "dust": burst(3, 1.0, 1.0, true, 3),
        },
        "characters": {},
        "ground_offsets": [[0, 0, 0]],
    });
    Some((tex::png(w, h, &rgba), json))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn particle_paths_from_a_pack() {
        // a pack of one path (2 frames) in a pack of two parts
        let mut path = vec![0u8; 0x38 + 40];
        path[..4].copy_from_slice(&2u32.to_le_bytes());
        for (k, v) in [100.0f32, -50.0, 2000.0, 0.5, 0.25].iter().enumerate() {
            path[0x38 + 20 + 4 * k..0x38 + 24 + 4 * k].copy_from_slice(&v.to_le_bytes());
        }
        let pack = |parts: &[&[u8]]| {
            let head = (8 + 8 * parts.len()).next_multiple_of(16);
            let mut out = vec![0u8; head];
            out[..4].copy_from_slice(&(parts.len() as u32).to_le_bytes());
            let mut at = head;
            for (k, p) in parts.iter().enumerate() {
                out[8 + 8 * k..12 + 8 * k].copy_from_slice(&(p.len() as u32).to_le_bytes());
                out[4 + 8 * k..8 + 8 * k].copy_from_slice(&(at as u32).to_le_bytes());
                out.extend_from_slice(p);
                at += p.len();
            }
            let last = 12 + 8 * (parts.len() - 1);
            out[last..last + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            out
        };
        let file = pack(&[&[0; 16], &pack(&[&path])]);
        let p = paths(&file);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0][1].map(|v| (v * 1000.0).round()), [100.0, -50.0, 2000.0, 500.0, 250.0]);
    }
    use crate::model::{Inf, Node, Strip};

    #[test]
    fn a_shadow_mesh_from_the_low_model() {
        let inf = |node: usize| vec![Inf { node, n: [0.0; 3], p: [1000.0, 2000.0, -500.0], w: 1.0 }];
        let verts =
            vec![([0.0; 2], inf(1)), ([0.0; 2], inf(2)), ([0.0; 2], inf(18)), ([0.0; 2], vec![]), ([0.0; 2], inf(5))];
        // node 18 (hair) hangs from the head 100 mm along x
        let mut nodes: Vec<Node> = (0..19).map(|_| Node { parent: 0, pos: [0.0; 3], rot: [0.0; 3] }).collect();
        nodes[18] = Node { parent: 2, pos: [100.0, 0.0, 0.0], rot: [0.0; 3] };
        let m = Model { nodes, sections: vec![vec![Strip { tex0: 0, blend: false, verts }]] };
        let s = shadow(&m, 345);
        assert_eq!(s["models"], json!([345]));
        assert_eq!(s["joints"][2], "head");
        assert_eq!(s["vertices"][0], json!([1, 1.0, 2.0, -0.5]));
        assert_eq!(s["vertices"][2], json!([2, 1.1, 2.0, -0.5]));
        // the strip's three triangles, two of them through the vertex without a joint
        assert_eq!((s["vertices"].as_array().map(Vec::len), &s["triangles"]), (Some(4), &json!([[0, 1, 2]])));
    }

    #[test]
    fn the_atlas_puts_each_picture_in_its_cell() {
        let pic = |w: usize, v: u8| vec![(w, w, vec![v; w * w * 4])];
        let mut pics: Vec<_> = (0..4).map(|k| pic(if k % 2 == 1 { 128 } else { 64 }, k as u8 + 1)).collect();
        pics.push((0..20).map(|k| (32, 32, vec![10 + k as u8; 32 * 32 * 4])).collect());
        let (_, j) = atlas(&pics, &[vec![vec![[0.1, 0.2, 0.3, 1.0, 0.5]]]]).unwrap();
        // (the spark's paths at half their spread)
        assert_eq!(j["bursts"]["spark"]["paths"], json!([[[0.05, 0.1, 0.15, 1.0, 0.5]]]));
        assert_eq!((&j["bursts"]["block"]["level"], &j["bursts"]["zero"]["paths"]), (&0.25.into(), &json!([])));
        assert_eq!((j["atlas"]["cols"].as_u64(), j["atlas"]["rows"].as_u64()), (Some(8), Some(3)));
        assert_eq!(j["strips"]["heavy_b"]["cells"][0], 14);
        assert_eq!(j["strips"]["dust"]["cells"].as_array().map(Vec::len), Some(150));
        pics.pop();
        assert!(atlas(&pics, &[]).is_none());
    }
}

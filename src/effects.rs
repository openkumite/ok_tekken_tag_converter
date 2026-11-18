//! What a fight draws besides the fighters and the arena (docs/EFFECTS.md): the fighters' shadow meshes
//! (`gamedata/shadows.json`), each arena's shadow (`gamedata/stages.json`) and the hit and move effects' pictures
//! (`pictures/effects/`), in the layouts OpenKumite reads Tekken 3's in.

use crate::model::{Model, RIG, SCALE};
use crate::moves::VADDR;
use crate::{f32le, tex, u16le};
use serde_json::{Value, json};

/// The arenas' lights (0x54 bytes an arena: light 0's direction first, floats) and the shadow's alpha (u16, of 0x80).
const LIGHTS: usize = 0x41_6288;
const ALPHA: usize = 0x46_B280;
const ARENAS: usize = 23;

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
        triangles.extend(ids.windows(3).filter_map(|t| Some([t[0]?, t[1]?, t[2]?])));
    }
    json!({"models": [record], "joints": RIG.map(|r| r.0), "vertices": vertices, "triangles": triangles, "quads": []})
}

/// `gamedata/stages.json`: each arena's shadow as the Tekken 3 converter writes a stage without a light table, plus
/// its alpha: towards light 0 (the arena's axes, y up; turned up when it points down: the shadow falls along the
/// line either way), black, at the arena's alpha (0x342360).
pub fn stages(elf: &[u8]) -> Value {
    let stages: Vec<Value> = (0..ARENAS)
        .filter_map(|a| {
            let d = [0, 4, 8].map(|k| f32le(elf, LIGHTS + 0x54 * a + k - VADDR).map(f64::from));
            let [Some(x), Some(y), Some(z)] = d else { return None };
            let s = if y < 0.0 { -1.0 } else { 1.0 };
            let alpha = u16le(elf, ALPHA + 2 * a - VADDR)?;
            Some(json!({"id": a, "shadow": {"toward": [x * s, y * s, z * s], "colour": [0, 0, 0], "alpha": alpha as f64 / 128.0}}))
        })
        .collect();
    json!({"stages": stages})
}

/// The cells of the effects' atlas, 64 texels square.
const CELL: usize = 64;
const COLS: usize = 8;

/// `pictures/effects/atlas.png` and `effects.json` (the Tekken 3 converter's layout) from the effect files'
/// pictures `pics[k]` (files 4 + k, k 0-4: the four sets' and file 8's 20 fireball frames): the hit spark (set 0),
/// the zero-damage spark (set 3), the guard spark (set 1), the dust (set 2), each its set's frame count; the heavy
/// hits' fireballs, orange and blue (the charged attack's). None when a picture is missing.
// shortcut: each effect is one picture facing the camera (Tekken 3's sprites), not its set's particle paths, and the
// dust is not tinted by arena; decode the paths (EFFECTS.md section 3) to draw them as the game does
pub fn atlas(pics: &[Vec<(usize, usize, Vec<u8>)>]) -> Option<(Vec<u8>, Value)> {
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
        // each picture scaled to the cell (nearest texel)
        for (y, x) in (0..CELL).flat_map(|y| (0..CELL).map(move |x| (y, x))) {
            let at = (y * ph / CELL * pw + x * pw / CELL) * 4;
            let to = ((cy + y) * w + cx + x) * 4;
            if let (Some(s), Some(d)) = (px.get(at..at + 4), rgba.get_mut(to..to + 4)) {
                d.copy_from_slice(s);
            }
        }
    }
    let strip = |cell: usize, life: usize, blend: &str, note: &str| json!({"cells": vec![cell; life], "life": life, "blend": blend, "note": note});
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
        "characters": {},
        "ground_offsets": [[0, 0, 0]],
    });
    Some((tex::png(w, h, &rgba), json))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let (_, j) = atlas(&pics).unwrap();
        assert_eq!((j["atlas"]["cols"].as_u64(), j["atlas"]["rows"].as_u64()), (Some(8), Some(3)));
        assert_eq!(j["strips"]["heavy_b"]["cells"][0], 14);
        assert_eq!(j["strips"]["dust"]["cells"].as_array().map(Vec::len), Some(150));
        pics.pop();
        assert!(atlas(&pics).is_none());
    }
}

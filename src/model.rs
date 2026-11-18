//! "PS2TagA" skinned models: a node tree with its bind pose, and triangle strips whose vertices
//! carry up to four (node, normal, position, weight) influences in node-local space.
use crate::glb::{self, Glb, M4};
use crate::{f32le, tex, u32le};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

pub const SCALE: f64 = 0.001; // game units are millimetres
/// No model has more nodes (the fighters have 98 to 160): a broken count cannot ask for more work.
const NODES: usize = 1024;

pub struct Node {
    pub parent: i32,
    pub pos: [f32; 3],
    pub rot: [f32; 3],
}
pub struct Inf {
    pub node: usize,
    pub n: [f32; 3],
    pub p: [f32; 3],
    pub w: f32,
}
pub struct Strip {
    pub tex0: u64,
    pub blend: bool,
    pub verts: Vec<([f32; 2], Vec<Inf>)>,
}
pub struct Model {
    pub nodes: Vec<Node>,
    pub sections: Vec<Vec<Strip>>,
}

/// The nodes every fighter shares: the first of its table.
pub const SKELETON: usize = 98;
/// A fighter's picture is repeated in its atlas as far as its strips reach, to this many texels either way at most.
const REACH: usize = 1024;

/// The 18 animated joints (the arcade engine's matrix slots) in the first 98 nodes, which every
/// fighter shares: (name, nodes). Extra nodes of a slot are its twist copies and follow it. The names are Tekken
/// 3's, slot for slot (the Tekken 3 converter's: OpenKumite finds the head, chest and pelvis by them).
pub const RIG: [(&str, &[usize]); 18] = [
    ("root", &[0]),
    ("chest", &[1]),
    ("head", &[5]),
    ("clavicle_A", &[29, 30]),
    ("upperarm_A", &[31, 32, 33]),
    ("forearm_A", &[34, 35, 36]),
    ("hand_A", &[38]),
    ("clavicle_B", &[54, 55]),
    ("upperarm_B", &[56, 57, 58]),
    ("forearm_B", &[59, 60, 61]),
    ("hand_B", &[63]),
    ("pelvis", &[79]),
    ("thigh_A", &[81, 82, 83]),
    ("shin_A", &[84, 85]),
    ("foot_A", &[87]),
    ("thigh_B", &[90, 91, 92]),
    ("shin_B", &[93, 94]),
    ("foot_B", &[96]),
];

pub fn node_name(i: usize) -> String {
    for (name, nodes) in RIG {
        if let Some(k) = nodes.iter().position(|&n| n == i) {
            return if k == 0 { name.to_string() } else { format!("{name}.{k}") };
        }
    }
    format!("node_{i}")
}

fn v3(c: &[u8], o: usize) -> Option<[f32; 3]> {
    Some([f32le(c, o)?, f32le(c, o + 4)?, f32le(c, o + 8)?])
}

/// None when `c` is not a model, or one that does not hold together.
pub fn parse(c: &[u8]) -> Option<Model> {
    if !c.starts_with(b"PS2TagA") || c.len() < 0x80 {
        return None;
    }
    let count = (u32le(c, 0x14)? as usize).checked_sub(0x7c)? / 0x20;
    if count > NODES {
        return None;
    }
    let mut nodes = Vec::new();
    for o in (0..count).map(|i| 0x7c + 0x20 * i) {
        nodes.push(Node { parent: u32le(c, o)? as i32, pos: v3(c, o + 4)?, rot: v3(c, o + 16)? });
    }
    let mut sections = Vec::new();
    // 2 passes x 2 halves (upper body, legs) x 5 section offsets (the loader at 0x3AF624 relocates them all)
    for s in 0..20 {
        let so = u32le(c, 0x2c + 4 * s)? as usize;
        if so == 0 || so.saturating_add(16) > c.len() {
            continue;
        }
        let (mut o, end) = (so + 16, so.saturating_add(u32le(c, so + 4)? as usize).min(c.len()));
        let (mut pal, mut tex0, mut strips): (Vec<usize>, u64, Vec<Strip>) = (vec![], 0, vec![]);
        while o + 16 <= end {
            let w = [u32le(c, o)?, u32le(c, o + 4)?, u32le(c, o + 8)?, u32le(c, o + 12)?];
            if w[2] == 0x412 && w[3] == 0 && w[1] >> 24 == 0x31 {
                // GIF tag of a strip: ST, RGBAQ, XYZF2
                let mut verts = Vec::new();
                o += 16;
                for _ in 0..w[0] & 0x7fff {
                    if o + 32 > end {
                        return None;
                    }
                    let (uv, n) = ([f32le(c, o)?, f32le(c, o + 4)?], *c.get(o + 12)? as usize);
                    if n > 4 || o + 32 + 32 * n > end {
                        return None;
                    }
                    let mut inf = Vec::new();
                    for k in 0..n {
                        let (idx, b) = (u32le(c, o + 16 + 4 * k)? as usize, o + 32 + 32 * k);
                        let wt = f32le(c, b + 28)?;
                        // positions are stored multiplied by their weight
                        inf.push(Inf {
                            node: *pal.get(idx.wrapping_sub(1))?,
                            n: v3(c, b)?,
                            p: v3(c, b + 16)?.map(|x| x / wt),
                            w: wt,
                        });
                    }
                    verts.push((uv, inf));
                    o += 32 + 32 * n;
                }
                strips.push(Strip { tex0, blend: w[1] >> 15 & 0x40 != 0, verts });
            } else if w == [1, 0x2000_0000, 0x9e, 1] {
                o += 48; // ALPHA + CLAMP
            } else if w == [1, 0x1000_0000, 7, 2] {
                tex0 = u32le(c, o + 16)? as u64 | (u32le(c, o + 20)? as u64) << 32;
                o += 32;
            } else if w[0] <= 20 && w[1..] == [0, 0, 0] {
                // matrix palette: the nodes this block's vertices index (1-based)
                pal =
                    (0..w[0] as usize).map(|k| u32le(c, o + 16 + 8 * k).map(|n| n as usize)).collect::<Option<_>>()?;
                if pal.iter().any(|&n| n >= nodes.len()) {
                    return None;
                }
                o += 16 + 160;
            } else {
                return None;
            }
        }
        sections.push(strips);
    }
    Some(Model { nodes, sections })
}

/// A background character of an arena: 18 nodes, which are the joints of `RIG` in order, then VIF
/// packets for VU1. A strip is UNPACK V4-32 of a GIF tag and five quadwords a vertex:
///   s, t, matrix A, matrix B (address 7 * node - 6, 0 = none) | normal A | normal B | position A, weight | position B, weight
/// after an UNPACK V4-32 x4 with its material (GIF tag, ALPHA, CLAMP, TEX0).
// shortcut: a vertex follows its heavier node only. The file has no rest pose in which its two
// positions meet (a glTF skin needs one); to blend them, fit the rest rotations from the shared vertices.
pub fn parse_low(c: &[u8]) -> Option<Model> {
    const MESH: usize = 0x2b0;
    if c.len() < MESH + 16 || u32le(c, 0)? != 0 || u32le(c, 12)? as usize != MESH {
        return None;
    }
    let nodes = (0..18)
        .map(|i| 0x70 + 0x20 * i)
        .map(|o| Some(Node { parent: u32le(c, o)? as i32, pos: v3(c, o + 4)?, rot: [0.; 3] }));
    let nodes: Vec<Node> = nodes.collect::<Option<_>>()?;
    let (mut o, mut tex0, mut strips) = (MESH, 0, vec![]);
    while let Some(&[_, _, num, cmd]) = c.get(o..).and_then(|c| c.first_chunk::<4>()) {
        let (num, cmd) = (num as usize, cmd & 0x7f);
        o += 4;
        if cmd < 0x60 {
            continue; // FLUSH, BASE, OFFSET, MSCALF: no data
        }
        let (comps, bits) = ((cmd as usize >> 2 & 3) + 1, 32 >> (cmd & 3));
        let size = (comps * bits / 8 * if num == 0 { 256 } else { num }).next_multiple_of(4);
        let d = c.get(o..o + size)?;
        o += size;
        if cmd & 0x6f != 0x6c {
            continue;
        }
        if num == 4 {
            tex0 = u32le(d, 48)? as u64 | (u32le(d, 52)? as u64) << 32;
        } else if num > 5 && (num - 1).is_multiple_of(5) {
            let verts = d.get(16..)?.as_chunks::<80>().0.iter().map(|v| {
                let (u, f) = (|o: usize| u32le(v, o).unwrap_or(0), |o: usize| f32le(v, o).unwrap_or(0.0));
                let k = (u(12) > 0 && f(76) > f(60)) as usize;
                let (m, w) = (u(8 + 4 * k) as usize, f(60 + 16 * k));
                let at = |o: usize| [f(o), f(o + 4), f(o + 8)];
                let inf = if m > 0 && m / 7 < 17 && w > 0.0 {
                    vec![Inf { node: m / 7 + 1, n: at(16 + 16 * k), p: at(48 + 16 * k).map(|x| x / w), w: 1.0 }]
                } else {
                    vec![]
                };
                ([f(0), f(4)], inf)
            });
            strips.push(Strip { tex0, blend: u32le(d, 4)? >> 15 & 0x40 != 0, verts: verts.collect() });
        }
    }
    // shortcut: a model with parts metres from their node is not a person (Law's junks); those come
    // out wrongly posed, so they are left out until what moves them is decoded
    let person = strips.iter().flat_map(|s| &s.verts).flat_map(|v| &v.1).all(|i| i.p.iter().all(|x| x.abs() < 4000.0));
    (person && !strips.is_empty()).then_some(Model { nodes, sections: vec![strips] })
}

impl Model {
    /// The node `i` hangs from; None for a root.
    fn parent(&self, i: usize) -> Option<usize> {
        let p = usize::try_from(self.nodes.get(i)?.parent).ok()?;
        (p < self.nodes.len() && p != i).then_some(p)
    }

    /// Bind-pose world matrix of every node, in metres.
    pub fn world(&self) -> Vec<M4> {
        let local: Vec<M4> =
            self.nodes.iter().map(|n| glb::joint(n.rot.map(|x| x as f64), n.pos.map(|x| x as f64 * SCALE))).collect();
        let world = |i: usize| {
            // from the node up to its root (no further than every node: parents in a loop end there), multiplied
            // from the root down
            let chain: Vec<usize> = std::iter::successors(Some(i), |&at| self.parent(at)).take(local.len()).collect();
            chain.iter().rev().filter_map(|&k| local.get(k).copied()).reduce(|a, b| glb::mul(&a, &b)).unwrap_or(glb::ID)
        };
        (0..local.len()).map(world).collect()
    }

    /// The first `count` nodes as glTF nodes in their bind pose (a node whose parent is not among them is a root),
    /// the roots, and the inverse bind matrices' accessor.
    fn skeleton(&self, g: &mut Glb, world: &[M4], count: usize) -> (Vec<Value>, Vec<usize>, usize) {
        let mut nodes: Vec<Value> = (self.nodes.iter().take(count).enumerate())
            .map(|(i, n)| {
                let l = glb::joint(n.rot.map(|x| x as f64), [0.; 3]);
                let rig = RIG.get(i).filter(|_| self.nodes.len() == RIG.len());
                json!({"name": rig.map_or_else(|| node_name(i), |r| r.0.to_string()), "translation": n.pos.map(|x| x as f64 * SCALE), "rotation": glb::quat(&l)})
            })
            .collect();
        let mut roots = vec![];
        for i in 0..nodes.len() {
            if let Some(p) = self.parent(i).filter(|&p| p < count).and_then(|p| nodes.get_mut(p)?.as_object_mut()) {
                if let Some(a) = p.entry("children").or_insert_with(|| json!([])).as_array_mut() {
                    a.push(json!(i));
                }
            } else {
                roots.push(i);
            }
        }
        let ibm: Vec<f32> = (world.iter().take(count)).flat_map(|w| transposed(glb::inv(w))).collect();
        let ibm = g.accessor(&glb::floats(&ibm), glb::F32, "MAT4", ibm.len() / 16);
        (nodes, roots, ibm)
    }

    /// The fighter as OpenKumite's character: every section (the fight draws them all, docs/FORMATS.md section 7) in
    /// one mesh skinned on the [`SKELETON`] nodes every fighter shares, and one RGBA picture (OpenKumite's characters
    /// have one) with each strip's texture in its palette, cut to the texels its strips reach and repeated as far as
    /// they do (`ups`: the uploads its TEX0 registers refer to). A vertex on a node past the skeleton (cloth, hair)
    /// is on its nearest ancestor in it instead: no motion moves those nodes, so it stays where it was. The blended
    /// strips are a primitive of their own after the opaque ones, their texture's alpha kept. None when the model has
    /// fewer nodes, a skeleton node hangs from one past it, or its textures do not fit one picture.
    pub fn fighter_glb(&self, ups: &[tex::Upload]) -> Option<Vec<u8>> {
        let n = self.nodes.len();
        if n < SKELETON || (0..SKELETON).any(|i| self.parent(i).is_some_and(|p| p >= SKELETON)) {
            return None;
        }
        let world = self.world();
        let up: Vec<u16> = (0..n)
            .map(|i| std::iter::successors(Some(i), |&a| self.parent(a)).take(n).find(|&a| a < SKELETON).unwrap_or(0))
            .map(|a| a as u16)
            .collect();
        let strips: Vec<&Strip> = self.sections.iter().flatten().collect();
        // each texture in its palette once: (TEX0's texture, size and palette pointers, blended) -> picture (width,
        // height, pixels, how far its strips reach: low x, y, high x, y), the picture's part of TW x TH
        let (mut pics, mut pic_of) = (vec![], BTreeMap::new());
        // each strip's picture (none: the white one), its coordinates in pictures, moved by whole pictures to
        // start in the first one (the GS repeats a texture)
        let mut placed = vec![];
        for s in &strips {
            let t = tex::tex0(s.tex0);
            let pic = *pic_of.entry((s.tex0 & 0x000f_ffe3_ffff_ffff, s.blend)).or_insert_with(|| {
                let (w, h, px) = tex::image(ups, &t, s.blend).filter(|p| p.0 > 0 && p.1 > 0)?;
                pics.push((w, h, px, [f32::MAX, f32::MAX, 0.0, 0.0]));
                Some((pics.len() - 1, [t.tw as f32 / w as f32, t.th as f32 / h as f32]))
            });
            let k = pic.map_or([0.; 2], |p| p.1);
            let st: Vec<[f32; 2]> = s.verts.iter().map(|v| [v.0[0] * k[0], v.0[1] * k[1]]).collect();
            let axis = |a: usize| st.iter().filter_map(move |c| c.get(a).copied());
            let off = [0, 1].map(|a| axis(a).fold(f32::MAX, f32::min).floor());
            if let Some((.., r)) = pic.and_then(|p| pics.get_mut(p.0)) {
                for [u, v] in st.iter().map(|c| [c[0] - off[0], c[1] - off[1]]) {
                    *r = [r[0].min(u), r[1].min(v), r[2].max(u), r[3].max(v)];
                }
            }
            placed.push((pic.map(|p| p.0), off, st));
        }
        pics.push((1, 1, vec![255; 4], [0.0, 0.0, 1.0, 1.0]));
        // each picture cut to the texels its strips reach, repeated as far as they do (to REACH texels at most):
        // (the texels across, down)
        let span = |lo: f32, hi: f32, n: usize| {
            let a = ((lo.max(0.0) * n as f32) as usize).min(n - 1);
            a..((hi * n as f32).ceil() as usize).clamp(a + 1, a + REACH.max(n))
        };
        let cuts: Vec<_> = pics.iter().map(|(w, h, _, r)| (span(r[0], r[2], *w), span(r[1], r[3], *h))).collect();
        let cut: Vec<(usize, usize, Vec<u8>)> = (pics.iter().zip(&cuts))
            .map(|((w, h, px, _), (x, y))| (x.len(), y.len(), tex::cut(*w, *h, px, x.clone(), y.clone())))
            .collect();
        let atlas = tex::atlas(&cut)?;
        let (aw, ah) = (atlas.w as f32, atlas.h as f32);
        // the opaque strips and the blended ones, each a primitive with vertices of its own
        type Part = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<f32>, Vec<u8>, Vec<f32>, Vec<u32>);
        let mut parts: [Part; 2] = Default::default();
        for (s, (pic, off, st)) in strips.iter().zip(&placed) {
            let at = pic.unwrap_or(pics.len() - 1);
            let (&(x, y), &(w, h, ..), (cx, cy)) = (atlas.place.get(at)?, pics.get(at)?, cuts.get(at)?);
            let (pos, nrm, uv, jn, wt, idx) = parts.get_mut(s.blend as usize)?;
            let base = pos.len();
            for ((_, inf), c) in s.verts.iter().zip(st) {
                let (p, nm, j, wv) = skin_vertex(&world, inf, |node| up.get(node).copied().unwrap_or(0));
                pos.push(p);
                nrm.push(nm);
                // shortcut: a coordinate past REACH texels of repeats is clamped to their edge, upgrade with more
                // repeats where the atlas has room
                let texel = |c: f32, o: f32, n: usize, r: &std::ops::Range<usize>| {
                    ((c - o) * n as f32).clamp(r.start as f32, r.end as f32) - r.start as f32
                };
                let (u, v) = (texel(c[0], off[0], w, cx), texel(c[1], off[1], h, cy));
                uv.extend([(x as f32 + u) / aw, (y as f32 + v) / ah]);
                jn.extend(j.iter().flat_map(|x| x.to_le_bytes()));
                wt.extend(wv);
            }
            idx.extend(strip_faces(pos, nrm, base));
        }
        let mut g = Glb::default();
        let (mut nodes, mut roots, ibm) = self.skeleton(&mut g, &world, SKELETON);
        let mut prims = vec![];
        for (mat, (pos, nrm, uv, jn, wt, idx)) in parts.iter().enumerate().filter(|p| !p.1.5.is_empty()) {
            let n = pos.len();
            let attrs = json!({"POSITION": g.ranged(pos.as_flattened(), 3, "VEC3"), "NORMAL": g.accessor(&glb::floats(nrm.as_flattened()), glb::F32, "VEC3", n),
                "TEXCOORD_0": g.accessor(&glb::floats(uv), glb::F32, "VEC2", n), "JOINTS_0": g.accessor(jn, glb::U16, "VEC4", n),
                "WEIGHTS_0": g.accessor(&glb::floats(wt), glb::F32, "VEC4", n)});
            let bytes: Vec<u8> = idx.iter().flat_map(|x| x.to_le_bytes()).collect();
            prims.push(json!({"attributes": attrs, "indices": g.accessor(&bytes, glb::U32, "SCALAR", idx.len()), "material": mat}));
        }
        if prims.is_empty() {
            return None;
        }
        let png = g.view(&tex::png(atlas.w, atlas.h, &atlas.rgba));
        nodes.push(json!({"name": "part_0", "mesh": 0, "skin": 0}));
        roots.push(nodes.len() - 1);
        let pbr = json!({"baseColorTexture": {"index": 0}, "metallicFactor": 0.0, "roughnessFactor": 1.0});
        let doc = json!({"scene": 0, "scenes": [{"nodes": roots}], "nodes": nodes,
            "meshes": [{"name": "part_0", "primitives": prims}],
            "materials": [{"name": "opaque", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "MASK", "alphaCutoff": 0.5},
                          {"name": "blend", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"}],
            "skins": [{"joints": (0..SKELETON).collect::<Vec<_>>(), "inverseBindMatrices": ibm}],
            "images": [{"bufferView": png, "mimeType": "image/png"}], "textures": [{"source": 0, "sampler": 0}],
            "samplers": [{"magFilter": 9729, "minFilter": 9729, "wrapS": 33071, "wrapT": 33071}]});
        Some(g.finish(doc))
    }

    /// The model as a skinned GLB; `ups` are the texture uploads its TEX0 registers refer to.
    pub fn glb(&self, ups: &[tex::Upload]) -> Vec<u8> {
        let world = self.world();
        let mut g = Glb::default();
        let (mut nodes, mut roots, ibm) = self.skeleton(&mut g, &world, self.nodes.len());
        let (mut meshes, mut materials, mut images, mut textures) = (vec![], vec![], vec![], vec![]);
        let mut mat_of: HashMap<(u64, bool), (usize, [f32; 2])> = HashMap::new();
        for (si, strips) in self.sections.iter().enumerate() {
            let (mut pos, mut nrm, mut uv, mut jn, mut wt): (Vec<[f32; 3]>, Vec<[f32; 3]>, _, _, _) =
                (vec![], vec![], vec![], vec![], vec![]);
            let mut idx: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
            for s in strips {
                let t = tex::tex0(s.tex0);
                let key = (s.tex0 & 0x000f_ffe3_ffff_ffff, s.blend); // texture, size and palette pointers
                let (mat, k) = *mat_of.entry(key).or_insert_with(|| {
                    let mut pbr = json!({"metallicFactor": 0.0, "roughnessFactor": 1.0});
                    let mut k = [1.0, 1.0];
                    if let Some((w, h, px)) = tex::image(ups, &t, s.blend) {
                        let view = g.view(&tex::png(w, h, &px));
                        images.push(json!({"bufferView": view, "mimeType": "image/png"}));
                        textures.push(json!({"source": images.len() - 1, "sampler": 0}));
                        glb::set(&mut pbr, "baseColorTexture", json!({"index": textures.len() - 1}));
                        k = [t.tw as f32 / w as f32, t.th as f32 / h as f32]; // the image is the uploaded part of tw x th
                    }
                    let mut m = json!({"pbrMetallicRoughness": pbr, "doubleSided": true});
                    if s.blend {
                        glb::set(&mut m, "alphaMode", json!("BLEND"));
                    }
                    materials.push(m);
                    (materials.len() - 1, k)
                });
                let base = pos.len();
                for (st, inf) in &s.verts {
                    let (p, n, j, w) = skin_vertex(&world, inf, |node| node as u16);
                    pos.push(p);
                    nrm.push(n);
                    uv.extend([st[0] * k[0], st[1] * k[1]]);
                    jn.extend(j.iter().flat_map(|x| x.to_le_bytes()));
                    wt.extend(w);
                }
                idx.entry(mat).or_default().extend(strip_faces(&pos, &nrm, base));
            }
            if pos.is_empty() {
                continue;
            }
            let count = pos.len();
            let attrs = json!({"POSITION": g.ranged(pos.as_flattened(), 3, "VEC3"), "NORMAL": g.accessor(&glb::floats(nrm.as_flattened()), glb::F32, "VEC3", count),
                "TEXCOORD_0": g.accessor(&glb::floats(&uv), glb::F32, "VEC2", count), "JOINTS_0": g.accessor(&jn, glb::U16, "VEC4", count),
                "WEIGHTS_0": g.accessor(&glb::floats(&wt), glb::F32, "VEC4", count)});
            let mut prims = vec![];
            for (mat, ix) in idx {
                let bytes: Vec<u8> = ix.iter().flat_map(|x| x.to_le_bytes()).collect();
                prims.push(json!({"attributes": attrs, "indices": g.accessor(&bytes, glb::U32, "SCALAR", ix.len()), "material": mat}));
            }
            meshes.push(json!({"name": format!("part_{si}"), "primitives": prims}));
            nodes.push(json!({"name": format!("part_{si}"), "mesh": meshes.len() - 1, "skin": 0}));
            roots.push(nodes.len() - 1);
        }
        let mut doc = json!({"scene": 0, "scenes": [{"nodes": roots}], "nodes": nodes, "meshes": meshes, "materials": materials,
            "skins": [{"joints": (0..self.nodes.len()).collect::<Vec<_>>(), "inverseBindMatrices": ibm}]});
        if !images.is_empty() {
            glb::set(&mut doc, "images", json!(images));
            glb::set(&mut doc, "textures", json!(textures));
            glb::set(&mut doc, "samplers", json!([{"magFilter": 9729, "minFilter": 9987}]));
        }
        g.finish(doc)
    }
}

/// A vertex in the bind pose from its influences (`world`: the nodes' bind matrices): its position (metres) and
/// normal, and its joints (`joint`: a node's) and weights, the weights summing to one.
fn skin_vertex(world: &[M4], inf: &[Inf], joint: impl Fn(usize) -> u16) -> ([f32; 3], [f32; 3], [u16; 4], [f32; 4]) {
    let (mut p, mut n) = ([0f64; 3], [0f64; 3]);
    let (mut j, mut w) = ([0u16; 4], [0f32; 4]);
    for ((f, j), w) in inf.iter().zip(&mut j).zip(&mut w) {
        let Some(m) = world.get(f.node) else { continue };
        let (a, b) = (glb::point(m, f.p.map(|x| (x as f64 * SCALE) as f32)), glb::dir(m, f.n));
        for ((p, n), (a, b)) in p.iter_mut().zip(&mut n).zip(a.iter().zip(b)) {
            *p += a * f.w as f64;
            *n += b;
        }
        (*j, *w) = (joint(f.node), f.w);
    }
    let (sw, ln) = (w.iter().sum::<f32>().max(1e-6), libm::sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).max(1e-9));
    (p.map(|x| (x / sw as f64) as f32), n.map(|x| (x / ln) as f32), j, w.map(|x| x / sw))
}

/// The triangles of the strip whose vertices are `pos[base..]`. A strip's winding is not stored: each triangle
/// faces the way its vertex normals point, or a two-sided material lights it from behind.
fn strip_faces(pos: &[[f32; 3]], nrm: &[[f32; 3]], base: usize) -> Vec<u32> {
    let mut out = vec![];
    let (strip_pos, strip_nrm) = (pos.get(base..).unwrap_or_default(), nrm.get(base..).unwrap_or_default());
    for (i, (tri, normals)) in strip_pos.windows(3).zip(strip_nrm.windows(3)).enumerate() {
        let (&[p, q, r], &[na, nb, nc]) = (tri, normals) else { continue };
        let (a, b, c) = ((base + i) as u32, (base + i + 1) as u32, (base + i + 2) as u32);
        let (e, f) = ([q[0] - p[0], q[1] - p[1], q[2] - p[2]], [r[0] - p[0], r[1] - p[1], r[2] - p[2]]);
        let face = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
        let n = [na[0] + nb[0] + nc[0], na[1] + nb[1] + nc[1], na[2] + nb[2] + nc[2]];
        out.extend(if face[0] * n[0] + face[1] * n[1] + face[2] * n[2] >= 0.0 { [a, b, c] } else { [b, a, c] });
    }
    out
}

/// A matrix's 16 numbers column by column, as glTF stores them.
#[allow(clippy::indexing_slicing)] // (a fixed 4 x 4 array within constant bounds)
fn transposed(m: M4) -> impl Iterator<Item = f32> {
    (0..16).map(move |k| m[k % 4][k / 4] as f32)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A model of three nodes (the second and third hang from the first) and one section: a palette of node 1, a
    /// strip of three vertices on it.
    fn model() -> Vec<u8> {
        let mut c = b"PS2TagA\0".to_vec();
        c.resize(0x7c + 3 * 0x20, 0);
        let put = |c: &mut Vec<u8>, o: usize, v: u32| c[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut c, 0x14, 0x7c + 3 * 0x20);
        put(&mut c, 0x7c, u32::MAX); // the root
        put(&mut c, 0x7c + 4, 1000f32.to_bits());
        put(&mut c, 0x7c + 0x20 + 4, 500f32.to_bits());
        let so = c.len().next_multiple_of(16);
        put(&mut c, 0x34, so as u32);
        c.resize(so + 16, 0);
        let mut q = |w: [u32; 4]| c.extend(w.iter().flat_map(|v| v.to_le_bytes()));
        q([1, 0, 0, 0]); // the palette: node 1
        q([1, 0, 0, 0]);
        for _ in 0..9 {
            q([0; 4]);
        }
        q([3, 0x31 << 24, 0x412, 0]);
        for x in [0f32, 1.0, 0.0] {
            q([0, 0, 0, 1]); // s, t, one influence
            q([1, 0, 0, 0]); // of the palette's first node
            q([0, 0, 1f32.to_bits(), 0]);
            q([x.to_bits(), (1.0 - x).to_bits(), 0, 1f32.to_bits()]);
        }
        let size = (c.len() - so) as u32;
        put(&mut c, so + 4, size);
        c
    }

    #[test]
    fn parse_reads_nodes_and_strips() {
        let c = model();
        let m = parse(&c).unwrap();
        assert_eq!((m.nodes.len(), m.sections.len(), m.sections[0].len()), (3, 1, 1));
        let v = &m.sections[0][0].verts;
        assert_eq!((v.len(), v[1].1[0].node, v[1].1[0].p), (3, 1, [1.0, 0.0, 0.0]));
        // node 1 hangs from node 0: half a metre from a metre
        assert_eq!(m.world()[1][0][3], 1.5);
        assert!(m.glb(&[]).starts_with(b"glTF"));
        // cut anywhere: not a model (or one with fewer sections), never a panic
        for n in 0..c.len() {
            if let Some(m) = parse(&c[..n]) {
                let _ = m.glb(&[]);
            }
        }
        assert!(parse(&c[..c.len() - 16]).is_none());
        // a palette naming a node that is not there; too many nodes; not a model
        let mut bad = c.clone();
        let so = u32le(&c, 0x34).unwrap() as usize;
        bad[so + 32] = 9;
        assert!(parse(&bad).is_none());
        let mut many = c.clone();
        many[0x14..0x18].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&many).is_none());
        assert!(parse(b"PS2TagB").is_none());
        assert!(parse_low(&c).is_none());
        assert!(parse_low(&[0; 0x2b0 + 15]).is_none());
    }

    #[test]
    fn joints_named_as_tekken_3s() {
        let names: Vec<String> = [0, 1, 5, 29, 30, 79, 96, 2, 98].map(node_name).to_vec();
        assert_eq!(
            names,
            ["root", "chest", "head", "clavicle_A", "clavicle_A.1", "pelvis", "foot_B", "node_2", "node_98"]
        );
        // slot for slot the Tekken 3 converter's names (its SLOT_NAMES_18)
        let t3 = [
            "root",
            "chest",
            "head",
            "clavicle_A",
            "upperarm_A",
            "forearm_A",
            "hand_A",
            "clavicle_B",
            "upperarm_B",
            "forearm_B",
            "hand_B",
            "pelvis",
            "thigh_A",
            "shin_A",
            "foot_A",
            "thigh_B",
            "shin_B",
            "foot_B",
        ];
        assert_eq!(RIG.map(|r| r.0), t3);
    }

    #[test]
    fn a_fighter_as_openkumites_character() {
        // 99 nodes in a chain, the last hanging from the head (5); a strip on it, on a 16 x 16 PSMT4 texture of
        // colour 1 (red), its u reaching 2.5 pictures: the picture is repeated to 40 texels in the atlas
        let node = |i: i32| Node { parent: if i == 98 { 5 } else { i - 1 }, pos: [0.0, 100.0, 0.0], rot: [0.; 3] };
        let verts = || {
            [[0.0, 0.0], [2.5, 0.0], [0.0, 1.0]].map(|uv| {
                (uv, vec![Inf { node: 98, n: [0.0, 0.0, 1.0], p: [uv[0] * 100.0, uv[1] * 100.0, 0.0], w: 1.0 }])
            })
        };
        let tex0 = 0x100 | 2 << 14 | 0x14 << 20 | 4 << 26 | 4 << 30 | 0x200 << 37;
        // the strip again in a second section (as the legs are): in the same primitive
        let section = || vec![Strip { tex0, blend: false, verts: verts().into() }];
        let m = Model { nodes: (0..99).map(node).collect(), sections: vec![section(), section()] };
        let mut clut = vec![0u8; 64];
        clut[4..8].copy_from_slice(&[255, 0, 0, 0x80]);
        let up = |dbp, psm, w, h, data| tex::Upload { set: 0, dbp, dbw: 128, psm, w, h, data };
        let ups = [up(0x100, 0x14, 16, 16, vec![0x11; 128]), up(0x200, 0, 8, 2, clut)];
        let (doc, buffers, images) = gltf::import_slice(m.fighter_glb(&ups).unwrap()).unwrap();
        // as OpenKumite reads it: the first mesh, the first skin, the first image
        let skin = doc.skins().next().unwrap();
        let joints: Vec<_> = skin.joints().map(|j| j.name().unwrap().to_string()).collect();
        assert_eq!(
            (joints.len(), &joints[0], &joints[5], &joints[79]),
            (SKELETON, &"root".into(), &"head".into(), &"pelvis".into())
        );
        assert_eq!((doc.meshes().count(), images.len(), images[0].width, images[0].height), (1, 1, 1024, 32));
        let prim = doc.meshes().next().unwrap().primitives().next().unwrap();
        let r = prim.reader(|b| Some(&buffers[b.index()]));
        // the vertices on node 98 are on the head (it never moves apart from it), where they were: 5 nodes up a
        // chain of 0.1 m, then 0.1 m more
        let jn: Vec<[u16; 4]> = r.read_joints(0).unwrap().into_u16().collect();
        let pos: Vec<[f32; 3]> = r.read_positions().unwrap().collect();
        assert_eq!((jn[0], pos[0], pos.len()), ([5, 0, 0, 0], [0.0, 0.7, 0.0], 6));
        // the u of 2.5 pictures lands in the third copy, the last texels; they are red, the white texel beside them
        let uv: Vec<[f32; 2]> = r.read_tex_coords(0).unwrap().into_f32().collect();
        assert_eq!(uv[1], [(1.0 + 2.5 * 16.0) / 1024.0, 1.0 / 32.0]);
        let px = |x: usize, y: usize| &images[0].pixels[(y * 1024 + x) * 4..][..4];
        assert_eq!(
            (px(1, 1), px(40, 16), px(41, 16), px(42, 1)),
            (&[255, 0, 0, 255][..], &[255, 0, 0, 255][..], &[255, 0, 0, 255][..], &[255; 4][..])
        );
        // fewer nodes than the skeleton, or a skeleton node hanging from one past it: not a fighter
        assert!(Model { nodes: (0..97).map(node).collect(), sections: vec![] }.fighter_glb(&ups).is_none());
        let mut bad = Model { nodes: (0..99).map(node).collect(), sections: vec![] };
        bad.nodes[7].parent = 98;
        assert!(bad.fighter_glb(&ups).is_none());
    }

    #[test]
    fn parents_in_a_loop_end() {
        let node = |parent| Node { parent, pos: [1000.0, 0.0, 0.0], rot: [0.; 3] };
        let m = Model { nodes: vec![node(1), node(2), node(0), node(7), node(3)], sections: vec![] };
        let w = m.world();
        assert_eq!(w.len(), 5);
        assert_eq!((w[3][0][3], w[4][0][3]), (1.0, 2.0)); // a parent that is not there: a root
        assert!(m.glb(&[]).starts_with(b"glTF"));
    }
}

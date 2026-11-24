//! "PS2TagA" skinned models: a node tree with its bind pose, and triangle strips whose vertices
//! carry up to four (node, normal, position, weight) influences in node-local space.
use crate::glb::{self, Glb, M4};
use crate::{f32le, tex, u16le, u32le};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

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
#[derive(Default)]
pub struct Strip {
    pub tex0: u64,
    pub blend: bool,
    pub verts: Vec<([f32; 2], Vec<Inf>)>,
    /// How the fighter draw (0x3439B0) draws it: the section's blend state before it ([`Draw`])
    pub draw: Draw,
    /// Each vertex's depth bias (+0xC, the float under its influence count; mm, 0 or negative: the VU0 programs
    /// divide its depth by w less this, so it lies that much farther: Paul's trousers 80, his shoes 50 behind his
    /// jacket's hem)
    pub bias: Vec<f32>,
}

/// A strip's blend state record (`{1, 0x20000000, 0x9E, 1}`: ALPHA_2 and CLAMP_2 by A+D, and two words the fighter
/// draw keeps for itself): how 0x3439B0 draws it.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Draw {
    /// ALPHA_2: 0x44 ordinary blending, 0x48 additive (Cs x As + Cd), 0x89 multiplying (Cd x As); used when PRIM has ABE
    pub alpha: u8,
    /// CLAMP_2's region repeat masks (WMS / WMT 3: the texels `x & u`, `y & v`; 0: not repeated so)
    pub repeat: [u16; 2],
    /// CLAMP_2's clamps by axis (u, v): WMS / WMT 1 (CLAMP: the texture's edges, `[0, u16::MAX]`) or 2 (REGION_CLAMP:
    /// `[MIN, MAX]`): the GS reads a texel past them at the edge, never repeats it (hair cards cut from a sheet of them
    /// would take their neighbour's texels)
    pub clamp: [Option<[u16; 2]>; 2],
    /// The ALPHA qword's top word not 0: 0x3438B8 and VU0 program 0x186: an environment map (its texture
    /// coordinates from the normal and the way to the eye, the colour 128)
    pub env: bool,
    /// The CLAMP qword's upper half, 0x3436B8 (its low word, VU0 program 0x4A) or 0x3437B8 (its high word, 0x0EA): a
    /// highlight of light 0 by (R . L)^(2^n) x strength added to the lighting, or alone: (strength, n, alone)
    pub spec: Option<(f32, u8, bool)>,
}

impl Draw {
    /// The lighting the GS shows: 0 lit (the VU0 program's colour modulating the texture), 1 the texture as it is
    /// (TEX0's TFX decal), 2 environment-mapped (decal), 3 lit with a highlight, 4 the highlight alone.
    pub fn light(&self, tex0: u64) -> u8 {
        match (self.env, self.spec) {
            (true, _) => 2,
            (_, Some((.., false))) => 3,
            (_, Some((.., true))) => 4,
            _ if tex0 >> 35 & 3 == 1 => 1,
            _ => 0,
        }
    }
}
pub struct Model {
    pub nodes: Vec<Node>,
    pub sections: Vec<Vec<Strip>>,
    /// True Ogre's and Devil's wings: the nodes whose +0x1C is 0x1F, 0x22, 0x25 (one wing: its root, middle and tip)
    /// and 0x20, 0x23, 0x26 (the other), the order of the six slots of table 0x46B010 (0x3565D8).
    pub wings: Option<[usize; 6]>,
}

/// A wing track (a fighter pack's part 1 for a winged fighter: `u16 frames, u16 triples` 6, then 6 s16 a wing node a
/// frame) as each frame's six wing nodes' local turn and scale (x, y, z, w, sx, sy, sz), slot by slot ([`Model::wings`]):
/// 0x3DE4E0 takes node pair k % 3's 12 bytes, scales (1/4096, 0x3DD800 the matrix's rows) then a turn (65536 a turn,
/// 0x349D70 as a joint's): the first wing (-x, -y, z) as a motion's joints (0x3DD788), the other (x, y, z) (0x3DE428).
pub fn wing_frames(t: &[u8]) -> Vec<Vec<[f32; 7]>> {
    if u16le(t, 2) != Some(6) {
        return vec![];
    }
    let frames = (u16le(t, 0).unwrap_or(0) as usize).min(t.len().saturating_sub(4) / 36);
    let at = |f: usize, i: usize| u16le(t, 4 + 2 * (18 * f + i)).unwrap_or(0);
    let k = std::f64::consts::TAU / 65536.0;
    (0..frames)
        .map(|f| {
            (0..6)
                .map(|slot| {
                    let b = 6 * (slot % 3);
                    let [x, y, z] = [3, 4, 5].map(|i| at(f, b + i) as i16 as f64 * k);
                    let r = if slot < 3 { [-x, -y, z] } else { [x, y, z] };
                    let [qx, qy, qz, qw] = glb::quat(&glb::joint(r, [0.; 3]));
                    let s = [0, 1, 2].map(|i| at(f, b + i) as f32 / 4096.0);
                    [qx, qy, qz, qw, s[0], s[1], s[2]]
                })
                .collect()
        })
        .collect()
}

/// A fighter's swaying nodes (belts, hair, cloth, tails) as 0x34D258 moves them each drawn frame: the nodes past the
/// shared skeleton whose id (+0x1C) has a record in its pack's part 10 (0x34E130 matches them; 0x20 bytes a record,
/// to `u16` 0xFFFF: `s8 keep, gravity` hundredths, `s16 frames, length`, `s16` limits in degrees (y max, min, z max,
/// min; equal: none), `s16 id`, `u8 flags`, `s8 wind` hundredths). `{node, keep, gravity, frames, length (m), limits
/// (radians), flags, wind}` each; not those whose record keeps them still (no keep, no gravity: True Ogre's wings,
/// which their track turns) nor `held` ids (taken from the strike slots past 17 instead, 0x34D038).
pub fn cloth(model: &[u8], records: &[u8], held: &[i32]) -> Vec<Value> {
    let count = u32le(model, 0x14).and_then(|e| e.checked_sub(0x7c)).map_or(0, |e| e as usize / 0x20).min(NODES);
    let s8 = |r: &[u8], o: usize| r.get(o).map_or(0, |&b| b as i8);
    let s16 = |r: &[u8], o: usize| u16le(r, o).map_or(0, |v| v as i16);
    let recs: Vec<&[u8; 0x20]> =
        records.as_chunks::<0x20>().0.iter().take_while(|r| u16le(*r, 0) != Some(0xffff)).collect();
    (SKELETON..count)
        .filter_map(|i| {
            let id = Some(u32le(model, 0x7c + 0x20 * i + 0x1c)? as i32).filter(|id| *id != -1 && !held.contains(id))?;
            let r: &[u8] = recs.iter().copied().find(|r| i32::from(s16(*r, 0xe)) == id && (r[0] != 0 || r[1] != 0))?;
            let deg = |o: usize| f64::from(s16(r, o)) * f64::from(std::f32::consts::PI) / 180.0;
            Some(json!({"node": i, "keep": f64::from(s8(r, 0)) / 100.0, "gravity": f64::from(s8(r, 1)) / 100.0,
                "frames": s16(r, 2), "length": f64::from(s16(r, 4)) * SCALE, "limits": [deg(6), deg(8), deg(10), deg(12)],
                "flags": r.get(0x10).copied().unwrap_or(0), "wind": f64::from(s8(r, 0x11)) / 100.0}))
        })
        .collect()
}

/// The nodes every fighter shares: the first of its table.
pub const SKELETON: usize = 98;
/// A fighter's picture is repeated in its atlas as far as its strips reach, to this many texels either way at most.
const REACH: usize = 1024;
/// OpenKumite draws a skinned model with this many joints at most.
const JOINTS: usize = 160;

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
    let wing = |k: u32| (0..count).find(|i| u32le(c, 0x7c + 0x20 * i + 0x1c) == Some(k));
    let wings = [0x1f, 0x22, 0x25, 0x20, 0x23, 0x26].map(wing);
    let wings = wings.iter().all(Option::is_some).then(|| wings.map(|w| w.unwrap_or(0)));
    let mut sections = Vec::new();
    // 2 passes x 2 halves (upper body, legs) x 5 section offsets (the loader at 0x3AF624 relocates them all)
    for s in 0..20 {
        let so = u32le(c, 0x2c + 4 * s)? as usize;
        if so == 0 || so.saturating_add(16) > c.len() {
            continue;
        }
        let (mut o, end) = (so + 16, so.saturating_add(u32le(c, so + 4)? as usize).min(c.len()));
        let (mut pal, mut tex0, mut strips): (Vec<usize>, u64, Vec<Strip>) = (vec![], 0, vec![]);
        let mut draw = Draw::default();
        while o + 16 <= end {
            let w = [u32le(c, o)?, u32le(c, o + 4)?, u32le(c, o + 8)?, u32le(c, o + 12)?];
            if w[2] == 0x412 && w[3] == 0 && w[1] >> 24 == 0x31 {
                // GIF tag of a strip: ST, RGBAQ, XYZF2
                let (mut verts, mut bias) = (Vec::new(), Vec::new());
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
                    bias.push(f32::from_bits(u32le(c, o + 12)? & 0xffff_ff00));
                    o += 32 + 32 * n;
                }
                strips.push(Strip { tex0, blend: w[1] >> 15 & 0x40 != 0, verts, draw, bias });
            } else if w == [1, 0x2000_0000, 0x9e, 1] {
                // ALPHA (A+D), then CLAMP (packed): the upper words are the draw's own
                let clamp = u32le(c, o + 32)? as u64 | (u32le(c, o + 36)? as u64) << 32;
                let own = [u32le(c, o + 40)?, u32le(c, o + 44)?];
                let mask = |wm: u64, m: u64| if wm & 3 == 3 { (m & 0x3ff) as u16 } else { 0 };
                let range = |wm: u64, lo: u64, hi: u64| match wm & 3 {
                    1 => Some([0, u16::MAX]),
                    2 => Some([(lo & 0x3ff) as u16, (hi & 0x3ff) as u16]),
                    _ => None,
                };
                let spec = |w: u32, alone| Some((f32::from_bits(w), w as u8, alone));
                draw = Draw {
                    alpha: *c.get(o + 16)?,
                    repeat: [mask(clamp, clamp >> 4), mask(clamp >> 2, clamp >> 24)],
                    clamp: [range(clamp, clamp >> 4, clamp >> 14), range(clamp >> 2, clamp >> 24, clamp >> 34)],
                    env: u32le(c, o + 28)? != 0,
                    spec: if own[0] != 0 {
                        spec(own[0], false)
                    } else if own[1] != 0 {
                        spec(own[1], true)
                    } else {
                        None
                    },
                };
                o += 48;
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
    Some(Model { nodes, sections, wings })
}

/// A background character of an arena, or a fighter's low-detail model (its shadow, pack part 8): 18 nodes, which
/// are the joints of `RIG` in order, and as many more as `+8` says (hair, cloth: a few fighters'), then VIF
/// packets for VU1. A strip is UNPACK V4-32 of a GIF tag and five quadwords a vertex:
///   s, t, matrix A, matrix B (address 7 * node - 6, 0 = none) | normal A | normal B | position A, weight | position B, weight
/// after an UNPACK V4-32 x4 with its material (GIF tag, ALPHA, CLAMP, TEX0).
// shortcut: a person's vertex follows its heavier node only. The file has no rest pose in which its two
// positions meet (a glTF skin needs one); to blend them, give them joints of their own as `fighter_glb` does the junks'.
pub fn parse_low(c: &[u8]) -> Option<Model> {
    // 18 joints, and as many more as +8 says (a fighter's hair or cloth: the shadow's)
    let count = 18 + (u32le(c, 8)? as usize).min(16);
    let mesh = 0x70 + 0x20 * count;
    if c.len() < mesh + 16 || u32le(c, 0)? != 0 || u32le(c, 12)? as usize != mesh {
        return None;
    }
    let nodes = (0..count)
        .map(|i| 0x70 + 0x20 * i)
        .map(|o| Some(Node { parent: u32le(c, o)? as i32, pos: v3(c, o + 4)?, rot: [0.; 3] }));
    let nodes: Vec<Node> = nodes.collect::<Option<_>>()?;
    let (mut o, mut tex0, mut strips) = (mesh, 0, vec![]);
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
                let at = |o: usize| [f(o), f(o + 4), f(o + 8)];
                // the heavier first
                let order = if u(12) > 0 && f(76) > f(60) { [1, 0] } else { [0, 1] };
                let inf = order.into_iter().filter_map(|k| {
                    let (m, w) = (u(8 + 4 * k) as usize, f(60 + 16 * k));
                    (m > 0 && m / 7 + 1 < count && w > 0.0).then(|| Inf {
                        node: m / 7 + 1,
                        n: at(16 + 16 * k),
                        p: at(48 + 16 * k).map(|x| x / w),
                        w,
                    })
                });
                ([f(0), f(4)], inf.collect::<Vec<_>>())
            });
            strips.push(Strip {
                tex0,
                blend: u32le(d, 4)? >> 15 & 0x40 != 0,
                verts: verts.collect(),
                ..Default::default()
            });
        }
    }
    // a person follows its heavier node only (near enough); a model with parts metres from their node is not one
    // (Law's junks: their sails stretch between nodes), and keeps both for `fighter_glb` to blend as the PS2 does
    let person =
        strips.iter().flat_map(|s| &s.verts).filter_map(|v| v.1.first()).all(|i| i.p.iter().all(|x| x.abs() < 4000.0));
    if person {
        for v in strips.iter_mut().flat_map(|s| &mut s.verts) {
            v.1.truncate(1);
            v.1.iter_mut().for_each(|i| i.w = 1.0);
        }
    }
    (!strips.is_empty()).then_some(Model { nodes, sections: vec![strips], wings: None })
}

impl Model {
    /// The node `i` hangs from; None for a root.
    fn parent(&self, i: usize) -> Option<usize> {
        let p = usize::try_from(self.nodes.get(i)?.parent).ok()?;
        (p < self.nodes.len() && p != i).then_some(p)
    }

    /// Draw the joints where the PS2 does (0x3478F0 gives each node of `RIG` its slot's matrix, 0x439A80's joint
    /// place `places` for its offset): each slot's node and twist copies at its place, the clavicles' less node 2's
    /// offset (they hang from it); the head keeps its own (it takes its slot's turn only), the root its motion's.
    pub fn place_joints(&mut self, places: &[[i16; 3]]) {
        let n2 = self.nodes.get(2).map_or([0.0; 3], |n| n.pos);
        for (s, (_, nodes)) in RIG.iter().enumerate().filter(|&(s, _)| s != 0 && s != 2) {
            let Some(p) = places.get(s) else { continue };
            let less = if s == 3 || s == 7 { n2 } else { [0.0; 3] };
            let ([x, y, z], [a, b, c]) = (*p, less);
            for &i in *nodes {
                if let Some(n) = self.nodes.get_mut(i) {
                    n.pos = [f32::from(x) - a, f32::from(y) - b, f32::from(z) - c];
                }
            }
        }
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
    /// the roots, and the inverse bind matrices' accessor (then a translation for each of `extra`'s joints).
    fn skeleton(
        &self,
        g: &mut Glb,
        world: &[M4],
        count: usize,
        extra: &[(usize, [f64; 3])],
    ) -> (Vec<Value>, Vec<usize>, usize) {
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
        let moved =
            extra.iter().map(|&(_, [x, y, z])| [[1., 0., 0., x], [0., 1., 0., y], [0., 0., 1., z], [0., 0., 0., 1.]]);
        let ibm: Vec<f32> = (world.iter().take(count).map(glb::inv)).chain(moved).flat_map(transposed).collect();
        let ibm = g.accessor(&glb::floats(&ibm), glb::F32, "MAT4", ibm.len() / 16);
        (nodes, roots, ibm)
    }

    /// The fighter as OpenKumite's character: every section (the fight draws them all, docs/FORMATS.md section 7) in
    /// one mesh skinned on the [`SKELETON`] nodes every fighter shares, and one RGBA picture (OpenKumite's characters
    /// have one) with each strip's texture in its palette, cut to the texels its strips reach and repeated as far as
    /// they do (`ups`: the uploads its TEX0 registers refer to). A vertex on a node past the skeleton (cloth, hair)
    /// is on its nearest ancestor in it instead: no motion moves those nodes, so it stays where it was. The blended
    /// strips are a primitive of their own after the opaque ones, their texture's alpha kept. None when the model has
    /// fewer nodes, a skeleton node hangs from one past it, or its textures do not fit one picture. `alt`: the uploads
    /// with another texture channel frame (Jin's mark, BOWL_ENDINGS.md 7.4), or none: the texels it changes go in the
    /// picture too, the texture's extras `mark` `[x, y, w, h, from x, from y]` (the rectangle, where its other texels are).
    /// A winged fighter's skeleton reaches its last wing node, and `wing_track` (its pack's part 1) goes in the skin's
    /// extras `wings` ([`wing_frames`]). A fighter with swaying nodes (`cloth`, [`cloth`]) has all its nodes in its
    /// skeleton, and them in the skin's extras `cloth`.
    pub fn fighter_glb(
        &self,
        ups: &[tex::Upload],
        alt: &[tex::Upload],
        wing_track: &[u8],
        cloth: &[Value],
    ) -> Option<Vec<u8>> {
        let n = self.nodes.len();
        // (an arena's background character: the 18 joints of `RIG` alone)
        let wings = self.wings.filter(|w| w.iter().all(|&i| i < 128));
        let skeleton = match wings {
            _ if n == RIG.len() || !cloth.is_empty() => n,
            Some(w) => w.iter().max().map_or(SKELETON, |&m| (m + 1).max(SKELETON)),
            None => SKELETON,
        };
        if n < skeleton || (0..skeleton).any(|i| self.parent(i).is_some_and(|p| p >= skeleton)) {
            return None;
        }
        let world = self.world();
        let up: Vec<u16> = (0..n)
            .map(|i| std::iter::successors(Some(i), |&a| self.parent(a)).take(n).find(|&a| a < skeleton).unwrap_or(0))
            .map(|a| a as u16)
            .collect();
        let strips: Vec<&Strip> = self.sections.iter().flatten().collect();
        // each texture in its palette once: (TEX0's texture, size and palette pointers, blended) -> picture (width,
        // height, pixels, how far its strips reach: low x, y, high x, y), the picture's part of TW x TH
        let (mut pics, mut pic_of, mut alts, mut clamps) = (vec![], BTreeMap::new(), vec![], vec![]);
        // each strip's picture (none: the white one), its coordinates in pictures, moved by whole pictures to
        // start in the first one where the GS repeats the texture
        let mut placed = vec![];
        for s in &strips {
            let t = tex::tex0(s.tex0);
            // (TCC 0: the vertex's alpha, 0x80, for every texel)
            let (alpha, env) = (s.blend && t.alpha, s.draw.env);
            let clamp = if env { [None; 2] } else { s.draw.clamp };
            let key = (s.tex0 & 0x000f_ffe3_ffff_ffff, alpha, env.then_some(s.draw.repeat), clamp);
            let pic = *pic_of.entry(key).or_insert_with(|| {
                let (w, h, px) = tex::image(ups, &t, alpha).filter(|p| p.0 > 0 && p.1 > 0)?;
                if env {
                    // an environment map's coordinates span its whole TW x TH, the GS repeating texels x & u, y & v
                    // (CLAMP_2's region repeat): that picture, whole
                    let m = |r: u16, n: usize| if r > 0 { r as usize } else { n - 1 };
                    let (mu, mv) = (m(s.draw.repeat[0], t.tw), m(s.draw.repeat[1], t.th));
                    let at = |x: usize, y: usize| ((y & mv) % h * w + (x & mu) % w) * 4;
                    let px = (0..t.th * t.tw).flat_map(|k| {
                        px.get(at(k % t.tw, k / t.tw)..).and_then(|p| p.first_chunk::<4>()).copied().unwrap_or([0; 4])
                    });
                    alts.push(None);
                    clamps.push(clamp);
                    pics.push((t.tw, t.th, px.collect(), [0.0, 0.0, 1.0, 1.0]));
                    return Some((pics.len() - 1, [0.0; 2]));
                }
                alts.push(tex::image(alt, &t, alpha).filter(|a| (a.0, a.1) == (w, h) && a.2 != px).map(|a| a.2));
                clamps.push(clamp);
                pics.push((w, h, px, [f32::MAX, f32::MAX, 0.0, 0.0]));
                Some((pics.len() - 1, [t.tw as f32 / w as f32, t.th as f32 / h as f32]))
            });
            let k = pic.map_or([0.; 2], |p| p.1);
            let st: Vec<[f32; 2]> = s.verts.iter().map(|v| [v.0[0] * k[0], v.0[1] * k[1]]).collect();
            let axis = |a: usize| st.iter().filter_map(move |c| c.get(a).copied());
            let off = [0, 1].map(|a| {
                if clamp.get(a).is_some_and(Option::is_some) { 0.0 } else { axis(a).fold(f32::MAX, f32::min).floor() }
            });
            if let Some((.., r)) = pic.filter(|_| !env).and_then(|p| pics.get_mut(p.0)) {
                for [u, v] in st.iter().map(|c| [c[0] - off[0], c[1] - off[1]]) {
                    *r = [r[0].min(u), r[1].min(v), r[2].max(u), r[3].max(v)];
                }
            }
            placed.push((pic.map(|p| p.0), off, st));
        }
        pics.push((1, 1, vec![255; 4], [0.0, 0.0, 1.0, 1.0]));
        clamps.push([None; 2]);
        // each picture cut to the texels its strips reach, repeated as far as they do (to REACH texels at most):
        // (the texels across, down)
        let span = |lo: f32, hi: f32, n: usize| {
            let a = ((lo.max(0.0) * n as f32) as usize).min(n - 1);
            a..((hi * n as f32).ceil() as usize).clamp(a + 1, a + REACH.max(n))
        };
        let cuts: Vec<_> = pics.iter().map(|(w, h, _, r)| (span(r[0], r[2], *w), span(r[1], r[3], *h))).collect();
        let mut cut: Vec<(usize, usize, Vec<u8>)> = (pics.iter().zip(&cuts).zip(&clamps))
            .map(|(((w, h, px, _), (x, y)), c)| (x.len(), y.len(), clamped_cut(*w, *h, px, x.clone(), y.clone(), *c)))
            .collect();
        // the texels `alt` changes in a picture: their rectangle (x, y, w, h in the cut) as a picture of its own
        let mut marks = vec![];
        for (i, a) in alts.iter().enumerate().filter_map(|(i, a)| Some((i, a.as_ref()?))) {
            let (&(w, h, ..), (cx, cy), (cw, _, px)) = (pics.get(i)?, cuts.get(i)?, cut.get(i)?);
            let (cw, other) = (*cw, clamped_cut(w, h, a, cx.clone(), cy.clone(), *clamps.get(i)?));
            let diff: Vec<(usize, usize)> = (0..px.len() / 4)
                .filter(|&k| px.get(4 * k..4 * k + 4) != other.get(4 * k..4 * k + 4))
                .map(|k| (k % cw, k / cw))
                .collect();
            let (Some(x0), Some(y0)) = (diff.iter().map(|d| d.0).min(), diff.iter().map(|d| d.1).min()) else {
                continue;
            };
            let (x1, y1) =
                (diff.iter().map(|d| d.0).max().unwrap_or(x0) + 1, diff.iter().map(|d| d.1).max().unwrap_or(y0) + 1);
            let rows = (y0..y1).flat_map(|y| other.get(4 * (y * cw + x0)..4 * (y * cw + x1)).unwrap_or_default());
            cut.push((x1 - x0, y1 - y0, rows.copied().collect()));
            marks.push((i, [x0, y0, x1 - x0, y1 - y0], cut.len() - 1));
        }
        let atlas = tex::atlas(&cut)?;
        let mark = marks.iter().find_map(|&(i, [x, y, w, h], m)| {
            let (&(px, py), &(mx, my)) = (atlas.place.get(i)?, atlas.place.get(m)?);
            Some([px + x, py + y, w, h, mx, my])
        });
        let (aw, ah) = (atlas.w as f32, atlas.h as f32);
        // the opaque strips and the blended ones by their ALPHA (ordinary, additive, multiplying), each a primitive with
        // vertices of its own; each vertex's depth bias and lighting (TEXCOORD_1: metres farther, [`Draw::light`]) and
        // its highlight (TEXCOORD_2: strength, squarings) or, environment-mapped, its picture's size in the atlas
        // (TEXCOORD_0 its corner)
        type Part = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<f32>, Vec<u8>, Vec<f32>, Vec<u32>, Vec<f32>, Vec<f32>);
        let mut parts: [Part; 4] = Default::default();
        // joints past the skeleton: (node, the bind's translation)
        let mut extra: Vec<(usize, [f64; 3])> = vec![];
        for (s, (pic, off, st)) in strips.iter().zip(&placed) {
            let at = pic.unwrap_or(pics.len() - 1);
            let (&(x, y), &(w, h, ..), (cx, cy)) = (atlas.place.get(at)?, pics.get(at)?, cuts.get(at)?);
            let kind = match (s.blend, s.draw.alpha) {
                (false, _) => 0,
                (_, 0x48) => 2,
                (_, 0x89) => 3,
                _ => 1,
            };
            let (pos, nrm, uv, jn, wt, idx, look, more) = parts.get_mut(kind)?;
            let (env, light) = (s.draw.env && pic.is_some(), s.draw.light(s.tex0));
            let base = pos.len();
            for (k, ((_, inf), c)) in s.verts.iter().zip(st).enumerate() {
                look.extend([-s.bias.get(k).copied().unwrap_or(0.0) * SCALE as f32, f32::from(light)]);
                more.extend(match (env, s.draw.spec) {
                    (true, _) => [w as f32 / aw, h as f32 / ah],
                    (_, Some((strength, n, _))) => [strength, f32::from(n)],
                    _ => [0.0; 2],
                });
                let (p, nm, mut j, wv) = skin_vertex(&world, inf, |node| up.get(node).copied().unwrap_or(0));
                // a vertex between nodes of a background thing (Law's junks' sails): the PS2 blends its place on each
                // node, which no rest pose makes meet, so each is a joint of its own whose bind takes it to that place
                for (f, j) in inf.iter().zip(&mut j).filter(|_| n == RIG.len() && inf.len() > 1) {
                    let ([a, b, c], [x, y, z]) = (f.p, p);
                    let t = [a as f64 * SCALE - x as f64, b as f64 * SCALE - y as f64, c as f64 * SCALE - z as f64];
                    let k = extra.iter().position(|e| *e == (f.node, t)).unwrap_or(extra.len());
                    if skeleton + k < JOINTS {
                        if k == extra.len() {
                            extra.push((f.node, t));
                        }
                        *j = (skeleton + k) as u16;
                    }
                }
                pos.push(p);
                nrm.push(nm);
                // shortcut: a coordinate past REACH texels of repeats is clamped to their edge, upgrade with more
                // repeats where the atlas has room
                let texel = |c: f32, o: f32, n: usize, r: &std::ops::Range<usize>| {
                    ((c - o) * n as f32).clamp(r.start as f32, r.end as f32) - r.start as f32
                };
                let (u, v) = if env { (0.0, 0.0) } else { (texel(c[0], off[0], w, cx), texel(c[1], off[1], h, cy)) };
                uv.extend([(x as f32 + u) / aw, (y as f32 + v) / ah]);
                jn.extend(j.iter().flat_map(|x| x.to_le_bytes()));
                wt.extend(wv);
            }
            idx.extend(strip_faces(pos, nrm, base));
        }
        let mut g = Glb::default();
        let (mut nodes, mut roots, ibm) = self.skeleton(&mut g, &world, skeleton, &extra);
        let mut prims = vec![];
        for (mat, (pos, nrm, uv, jn, wt, idx, look, more)) in parts.iter().enumerate().filter(|p| !p.1.5.is_empty()) {
            let n = pos.len();
            let attrs = json!({"POSITION": g.ranged(pos.as_flattened(), 3, "VEC3"), "NORMAL": g.accessor(&glb::floats(nrm.as_flattened()), glb::F32, "VEC3", n),
                "TEXCOORD_0": g.accessor(&glb::floats(uv), glb::F32, "VEC2", n), "JOINTS_0": g.accessor(jn, glb::U16, "VEC4", n),
                "WEIGHTS_0": g.accessor(&glb::floats(wt), glb::F32, "VEC4", n),
                "TEXCOORD_1": g.accessor(&glb::floats(look), glb::F32, "VEC2", n), "TEXCOORD_2": g.accessor(&glb::floats(more), glb::F32, "VEC2", n)});
            let bytes: Vec<u8> = idx.iter().flat_map(|x| x.to_le_bytes()).collect();
            prims.push(json!({"attributes": attrs, "indices": g.accessor(&bytes, glb::U32, "SCALAR", idx.len()), "material": mat}));
        }
        if prims.is_empty() {
            return None;
        }
        let joints: Vec<usize> = (0..skeleton).chain(extra.iter().map(|e| e.0)).collect();
        let mut skin = json!({"joints": joints, "inverseBindMatrices": ibm});
        if let Some(w) = wings.filter(|_| !wing_track.is_empty()) {
            glb::set(&mut skin, "extras", json!({"wings": {"nodes": w, "frames": wing_frames(wing_track)}}));
        }
        if !cloth.is_empty() {
            let mut extras = skin.get("extras").cloned().unwrap_or_else(|| json!({}));
            glb::set(&mut extras, "cloth", json!(cloth));
            glb::set(&mut skin, "extras", extras);
        }
        let png = g.view(&tex::png(atlas.w, atlas.h, &atlas.rgba));
        nodes.push(json!({"name": "part_0", "mesh": 0, "skin": 0}));
        roots.push(nodes.len() - 1);
        let pbr = json!({"baseColorTexture": {"index": 0}, "metallicFactor": 0.0, "roughnessFactor": 1.0});
        let mut texture = json!({"source": 0, "sampler": 0});
        if let Some(m) = mark {
            glb::set(&mut texture, "extras", json!({"mark": m}));
        }
        let doc = json!({"scene": 0, "scenes": [{"nodes": roots}], "nodes": nodes,
            "meshes": [{"name": "part_0", "primitives": prims}],
            "materials": [{"name": "opaque", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "MASK", "alphaCutoff": 0.5},
                          {"name": "blend", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"},
                          {"name": "additive", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"},
                          {"name": "shade", "pbrMetallicRoughness": pbr, "doubleSided": true, "alphaMode": "BLEND"}],
            "skins": [skin],
            "images": [{"bufferView": png, "mimeType": "image/png"}], "textures": [texture],
            "samplers": [{"magFilter": 9729, "minFilter": 9729, "wrapS": 33071, "wrapT": 33071}]});
        Some(g.finish(doc))
    }

    /// The model as a skinned GLB; `ups` are the texture uploads its TEX0 registers refer to.
    pub fn glb(&self, ups: &[tex::Upload]) -> Vec<u8> {
        let world = self.world();
        let mut g = Glb::default();
        let (mut nodes, mut roots, ibm) = self.skeleton(&mut g, &world, self.nodes.len(), &[]);
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
/// The texels `x` x `y` of a `w` x `h` RGBA picture as CLAMP_2 reads them: on an axis with a clamp ([`Draw::clamp`])
/// held within it and the picture, else repeated (as [`tex::cut`]).
fn clamped_cut(
    w: usize,
    h: usize,
    px: &[u8],
    x: Range<usize>,
    y: Range<usize>,
    clamp: [Option<[u16; 2]>; 2],
) -> Vec<u8> {
    let at = |i: usize, n: usize, c: Option<[u16; 2]>| match c {
        Some([lo, hi]) => i.min(usize::from(hi)).max(usize::from(lo)).min(n.saturating_sub(1)),
        None => i % n.max(1),
    };
    let mut out = Vec::with_capacity(x.len() * y.len() * 4);
    for y in y {
        let row = at(y, h, clamp[1]) * w;
        for x in x.clone() {
            out.extend(px.get((row + at(x, w, clamp[0])) * 4..).and_then(|p| p.first_chunk::<4>()).unwrap_or(&[0; 4]));
        }
    }
    out
}

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
    fn blend_states_and_depth_biases_read() {
        // the model's strip after a blend state: additive, its top words the draw's own (an environment map, a
        // highlight of 0.5 squared 8 times), CLAMP_2 a region repeat of 32 x 16; its first vertex 80 mm farther
        let mut c = model();
        let so = u32le(&c, 0x34).unwrap() as usize;
        let at = so + 16 + 176;
        let words = [1u32, 0x2000_0000, 0x9e, 1, 0x48, 0, 0x43, 0x8000_0000, 0x1f00_00ff, 0, 0x3f00_0008, 0];
        let rec: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        c.splice(at..at, rec);
        let v = at + 48 + 16 + 12;
        c[v..v + 4].copy_from_slice(&((-80f32).to_bits() & 0xffff_ff00 | 1).to_le_bytes());
        let size = (c.len() - so) as u32;
        c[so + 4..so + 8].copy_from_slice(&size.to_le_bytes());
        let m = parse(&c).unwrap();
        let s = &m.sections[0][0];
        let d = s.draw;
        assert_eq!((d.alpha, d.repeat, d.env), (0x48, [15, 31], true));
        assert_eq!(d.spec.map(|(k, n, alone)| ((k * 2.0).round(), n, alone)), Some((1.0, 8, false)));
        assert_eq!((s.bias[0], s.bias[1]), (-80.0, 0.0));
        assert_eq!(d.light(0), 2);
        assert_eq!(Draw { env: false, ..d }.light(0), 3);
        assert_eq!(Draw::default().light(1 << 35), 1);
        assert_eq!(d.clamp, [None; 2]);
        // CLAMP_2 a region clamp across (MINU 4, MAXU 63) and a clamp down (Bryan's hair cards: a sheet of them)
        c[at + 32..at + 36].copy_from_slice(&(2u32 | 1 << 2 | 4 << 4 | 63 << 14).to_le_bytes());
        assert_eq!(parse(&c).unwrap().sections[0][0].draw.clamp, [Some([4, 63]), Some([0, u16::MAX])]);
        // read through them: held at the region's and the picture's edges, never repeated
        let px: Vec<u8> = (0..4 * 2).flat_map(|k| [k as u8, 0, 0, 255]).collect();
        let cut = clamped_cut(4, 2, &px, 0..6, 1..3, [Some([1, 2]), Some([0, u16::MAX])]);
        assert_eq!(cut.iter().step_by(4).copied().collect::<Vec<_>>(), [5, 5, 6, 6, 6, 6, 5, 5, 6, 6, 6, 6]);
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
    fn a_junks_vertex_between_nodes_is_where_each_node_puts_it() {
        // an arena's 18-node model, node 1 2 m up, a vertex 0.85 on node 1 (1 m along x) and 0.15 on node 2 (3 m
        // along z): two joints of its own, each binding it to that node's place for it
        let nodes =
            (0..18).map(|i| Node { parent: i - 1, pos: [0.0, if i == 1 { 2000.0 } else { 0.0 }, 0.0], rot: [0.; 3] });
        let inf = |node, p, w| Inf { node, n: [0.0, 0.0, 1.0], p, w };
        let two = vec![inf(1, [1000.0, 0.0, 0.0], 0.85), inf(2, [0.0, 0.0, 3000.0], 0.15)];
        let verts =
            vec![([0.0; 2], two), ([0.0; 2], vec![inf(1, [0.0; 3], 1.0)]), ([0.0; 2], vec![inf(2, [0.0; 3], 1.0)])];
        let strip = Strip { tex0: 0, blend: false, verts, ..Default::default() };
        let m = Model { nodes: nodes.collect(), sections: vec![vec![strip]], wings: None };
        let (doc, buffers, _) = gltf::import_slice(m.fighter_glb(&[], &[], &[], &[]).unwrap()).unwrap();
        let skin = doc.skins().next().unwrap();
        let joints: Vec<usize> = skin.joints().map(|j| j.index()).collect();
        let ibm: Vec<[[f32; 4]; 4]> =
            skin.reader(|b| Some(&buffers[b.index()])).read_inverse_bind_matrices().unwrap().collect();
        let prim = doc.meshes().next().unwrap().primitives().next().unwrap();
        let r = prim.reader(|b| Some(&buffers[b.index()]));
        let (p, jn) =
            (r.read_positions().unwrap().next().unwrap(), r.read_joints(0).unwrap().into_u16().next().unwrap());
        assert_eq!((joints.len(), &joints[18..], [jn[0], jn[1]]), (20, &[1, 2][..], [18, 19]));
        // (column-major: the translation is the last column)
        let local = |k: usize| [0, 1, 2].map(|a| p[a] + ibm[k][3][a]);
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
        assert!(near(local(18), [1.0, 0.0, 0.0]) && near(local(19), [0.0, 0.0, 3.0]), "{p:?}");
        // the vertex on one node keeps the node's own joint
        assert_eq!(r.read_joints(0).unwrap().into_u16().nth(1).unwrap()[0], 1);
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
        let section = || vec![Strip { tex0, blend: false, verts: verts().into(), ..Default::default() }];
        let m = Model { nodes: (0..99).map(node).collect(), sections: vec![section(), section()], wings: None };
        let mut clut = vec![0u8; 64];
        clut[4..8].copy_from_slice(&[255, 0, 0, 0x80]);
        let up = |dbp, psm, w, h, data| tex::Upload { set: 0, dbp, dbw: 128, psm, w, h, data };
        let ups = [up(0x100, 0x14, 16, 16, vec![0x11; 128]), up(0x200, 0, 8, 2, clut)];
        let (doc, buffers, images) = gltf::import_slice(m.fighter_glb(&ups, &[], &[], &[]).unwrap()).unwrap();
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
        // another channel frame with texels (3, 3) and (4, 4) changed (to colour 0): their rectangle, over the three
        // copies, in the picture too, as the texture's extras say (atlas texels)
        let mut other = vec![0x11; 128];
        (other[3 * 8 + 1], other[4 * 8 + 2]) = (0x01, 0x10);
        let alt = [up(0x100, 0x14, 16, 16, other), ups[1].clone()];
        let (doc, _, images) = gltf::import_slice(m.fighter_glb(&ups, &alt, &[], &[]).unwrap()).unwrap();
        let extras = doc.textures().next().unwrap().extras().as_ref().unwrap().get().to_string();
        let mark: serde_json::Value = serde_json::from_str(&extras).unwrap();
        let [x, y, w, h, sx, sy] = serde_json::from_value::<[usize; 6]>(mark["mark"].clone()).unwrap();
        assert_eq!([x, y, w, h], [4, 4, 34, 2]);
        let px = |x: usize, y: usize| &images[0].pixels[(y * images[0].width as usize + x) * 4..][..4];
        let red = &[255, 0, 0, 255][..];
        let black = &[0, 0, 0, 255][..];
        assert_eq!((px(sx, sy), px(sx + 1, sy), px(sx + 1, sy + 1), px(x, y)), (black, red, black, red));
        // fewer nodes than the skeleton, or a skeleton node hanging from one past it: not a fighter
        assert!(
            Model { nodes: (0..97).map(node).collect(), sections: vec![], wings: None }
                .fighter_glb(&ups, &[], &[], &[])
                .is_none()
        );
        let mut bad = Model { nodes: (0..99).map(node).collect(), sections: vec![], wings: None };
        bad.nodes[7].parent = 98;
        assert!(bad.fighter_glb(&ups, &[], &[], &[]).is_none());
    }

    #[test]
    fn a_wing_track_turns_one_wing_mirrored() {
        // one frame: node pair 0 a quarter turn about z at scale 1.25 (5120), the others still at 1
        let mut t = vec![1, 0, 6, 0];
        for v in [5120i16, 5120, 5120, 0, 0, 16384, 4096, 4096, 4096, 0, 0, 0, 4096, 4096, 4096, 0, 0, 0] {
            t.extend(v.to_le_bytes());
        }
        let f = wing_frames(&t);
        let h = std::f32::consts::FRAC_1_SQRT_2;
        // z is not negated on either wing: both +90 degrees about z
        assert!((f[0][0][2] - h).abs() < 1e-6 && (f[0][3][2] - h).abs() < 1e-6 && f[0][0][4] == 1.25, "{:?}", f[0]);
        assert_eq!(f[0][1], [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);
        // a cut frame, or not six triples: none
        assert!(wing_frames(&t[..20]).is_empty() && wing_frames(&[1, 0, 15, 0]).is_empty());
    }

    #[test]
    fn cloth_matched_by_id() {
        // 100 nodes, node 99 with id 5 (the others -1); Paul's first belt record, then an unmatched one, then the end
        let mut model = vec![0u8; 0x7c + 0x20 * 100];
        model[0x14..0x18].copy_from_slice(&(0x7c + 0x20 * 100u32).to_le_bytes());
        for i in 0..100 {
            let id: i32 = if i == 99 { 5 } else { -1 };
            model[0x7c + 0x20 * i + 0x1c..][..4].copy_from_slice(&id.to_le_bytes());
        }
        let mut recs = vec![0u8; 0x60];
        recs[..14].copy_from_slice(&[40, 35, 2, 0, 120, 0, 150, 0, 20, 0, 0xe2, 0xff, 0x6a, 0xff]);
        recs[0xe] = 5;
        (recs[0x11], recs[0x20 + 0xe], recs[0x40], recs[0x41]) = (5, 7, 0xff, 0xff);
        let c = cloth(&model, &recs, &[]);
        assert!(cloth(&model, &recs, &[5]).is_empty());
        assert_eq!(c.len(), 1);
        assert_eq!(
            (c[0]["node"].as_u64(), c[0]["keep"].as_f64(), c[0]["gravity"].as_f64()),
            (Some(99), Some(0.4), Some(0.35))
        );
        assert_eq!(
            (c[0]["frames"].as_i64(), c[0]["length"].as_f64(), c[0]["wind"].as_f64()),
            (Some(2), Some(0.12), Some(0.05))
        );
        assert!((c[0]["limits"][3].as_f64().unwrap() + 150f64.to_radians()).abs() < 1e-6);
        assert!(cloth(&model[..0x40], &recs, &[]).is_empty() && cloth(&model, &[], &[]).is_empty());
    }

    #[test]
    fn joints_drawn_at_the_games_places() {
        // Jack-2's: the knee 390 down the thigh (his model's 440), the clavicle 432 from the chest (node 2 at 286)
        let node = |i: usize| Node { parent: 0, pos: [if i == 2 { 286.0 } else { 440.0 }, 0.0, 0.0], rot: [0.; 3] };
        let mut m = Model { nodes: (0..98).map(node).collect(), sections: vec![], wings: None };
        let mut places = vec![[0i16; 3]; 24];
        (places[2], places[3], places[13]) = ([522, 0, 0], [432, 0, -100], [390, 0, 0]);
        m.place_joints(&places);
        let x = |i: usize| m.nodes[i].pos;
        assert_eq!(
            (x(84), x(85), x(29), x(30)),
            ([390.0, 0.0, 0.0], [390.0, 0.0, 0.0], [146.0, 0.0, -100.0], [146.0, 0.0, -100.0])
        );
        assert_eq!((x(5), x(0), x(2)), ([440.0, 0.0, 0.0], [440.0, 0.0, 0.0], [286.0, 0.0, 0.0])); // head, root, node 2 kept
    }

    #[test]
    fn parents_in_a_loop_end() {
        let node = |parent| Node { parent, pos: [1000.0, 0.0, 0.0], rot: [0.; 3] };
        let m = Model { nodes: vec![node(1), node(2), node(0), node(7), node(3)], sections: vec![], wings: None };
        let w = m.world();
        assert_eq!(w.len(), 5);
        assert_eq!((w[3][0][3], w[4][0][3]), (1.0, 2.0)); // a parent that is not there: a root
        assert!(m.glb(&[]).starts_with(b"glTF"));
    }
}

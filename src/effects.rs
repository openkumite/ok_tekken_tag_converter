//! What a fight draws besides the fighters and the arena (docs/EFFECTS.md): the fighters' shadow meshes
//! (`gamedata/shadows.json`), each arena's shadow (`gamedata/stages.json`) and the hit and move effects' pictures
//! (`pictures/effects/`), in the layouts OpenKumite reads Tekken 3's in.

use crate::model::{Model, RIG, SCALE};
use crate::moves::{self, VADDR};
use crate::{archive, f32le, tex, u16le, u32le};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// The arenas' lights (0x54 bytes an arena: light 0's direction first, floats) and the shadow's alpha (u16, of 0x80).
pub(crate) const LIGHTS: usize = 0x41_6288;
const ALPHA: usize = 0x46_B280;
/// The arenas' dust (12 bytes an arena: u32 particles, f32 scale, u8 r, g, b; 0x3A2758's kind 3).
const DUST: usize = 0x43_7AC0;
pub(crate) const ARENAS: usize = 23;
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
/// `pictures/effects/flare_K.png`, size, brightness]; its `weather` ([`weather`]) from `fx`: each arena's entries and
/// their pictures' atlas cells; `scales`, each arena's people's models' scale of their motions and walks (`scales`).
pub fn stages(
    elf: &[u8],
    fx: &BTreeMap<usize, (Vec<Entry>, Vec<Option<usize>>)>,
    scales: &BTreeMap<usize, Vec<f64>>,
) -> Value {
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
            let (people, walk) = people(elf, a, a);
            Some(json!({"id": a, "shadow": {"toward": [x * s, y * s, z * s], "colour": [0, 0, 0], "alpha": alpha as f64 / 128.0},
                "lights": {"toward": [v(0), v(1), v(2)], "colour": [v(3), v(4), v(5)], "ambient": v(6)}, "dust": dust, "people": people, "walk": walk, "scales": scales.get(&a), "objects": crate::objects::motion(elf, a), "flare": flare(a),
                "weather": fx.get(&a).and_then(|(e, cells)| weather(elf, a, e, cells))}))
        })
        .collect();
    json!({"stages": stages})
}

/// People set `set` of 0x114FC8 (an arena's own; 23-30 some endings', 0x3D85D8) on arena `arena`: stages.json's
/// `people` and their `walk` (how far each of their motions takes them: metres, degrees).
pub fn people(elf: &[u8], set: usize, arena: usize) -> (Vec<Value>, serde_json::Map<String, Value>) {
    let cast = moves::cast(elf, set).unwrap_or_default();
    // (0x3D86D8: arenas 7 and 21 draw each person with the model after its own)
    let next = usize::from(arena == 7 || arena == 21);
    let people: Vec<Value> = (cast.iter())
        .map(|p| {
            let script: Vec<Value> = (p.script.iter())
                .map(|s| json!({"motions": s.motions, "model": s.model, "repeat": s.repeat, "back": s.back, "wait": s.wait, "start": s.start, "frames": s.frames}))
                .collect();
            json!({"model": p.model + next, "at": p.pos.map(|v| (v as f64 * SCALE) as f32), "heading": p.heading, "motions": p.motions,
                "wait": p.wait, "start": p.start, "repeat": p.repeat, "script": script, "stop": p.stop})
        })
        .collect();
    let mut walk = serde_json::Map::new();
    for m in cast.iter().flat_map(|p| p.motions.iter().copied()) {
        if let Some([x, z, turn]) = moves::walk(elf, set, m) {
            walk.insert(m.to_string(), json!([x as f64 * SCALE, z as f64 * SCALE, turn]));
        }
    }
    (people, walk)
}

/// An arena's effect entry (its geometry pack's part 1 is a pack of up to three, each a pack of a texture pack and a
/// pack of particle paths; 0x345070 puts entry k's picture in texture slot 6 + 2k and its paths in set 6 + k): its
/// first picture and its paths ([`paths`]).
pub type Entry = (Option<(usize, usize, Vec<u8>)>, Vec<Vec<[f64; 5]>>);

/// The entries of an arena's geometry part 1.
pub fn entries(part: &[u8]) -> Vec<Entry> {
    (archive::pack(part).unwrap_or_default().iter())
        .map(|e| {
            let tex = archive::pack(e).and_then(|p| archive::pack(p.first()?));
            let pic = tex.and_then(|t| tex::pictures(&tex::uploads(t.first()?), &|_| None).into_iter().next());
            (pic, paths(e))
        })
        .collect()
}

/// The arenas' effect tasks' rows (0x3A2640 starts the tasks by arena; EFFECTS.md 5).
const SMOKE_4: usize = 0x51_94E0;
const SMOKE_15: usize = 0x51_95A0;
const STEAM: usize = 0x51_9660;
const JIN_A: usize = 0x43_6A30;
const JIN_B: usize = 0x43_6AC0;
const BEACONS_NINA: usize = 0x43_6B50;
const BEACONS_KING: usize = 0x43_6EE0;
/// Arena 4's lightning (task 0x3A76C0): the bolts' points (channels 0-3) and the flickers' (channels 4-6), 16 bytes
/// each (x, y, z, 1); the frame lists (s16 frames, one a frame, to a negative) in the order of EMBU op 7's table
/// 0x519B00 (its cases 0x3A800C.. load these).
const BOLTS: usize = 0x43_7C80;
const FLICKERS: usize = 0x43_7C50;
const FLASH_LISTS: [usize; 9] =
    [0x51_9990, 0x51_99A8, 0x51_99C0, 0x51_99D8, 0x51_99F0, 0x51_9A08, 0x51_9A20, 0x51_9A68, 0x51_9AB0];
/// The arenas' wind (0x3D3630: 12 bytes an arena, `u32 waves, *waves, u32`; a wave 32 bytes: `i32 step, f32 speed,
/// f32 strength, u32, f32 direction[4]`).
const WIND: usize = 0x44_7EC0;
/// An arena effect's quad (metres across at size 1 and 0x90 / 0x48 of 1): 0x345C10 makes its half 0x90 x size x z / 4
/// / 0x48 GS units (a sixteenth of a pixel), z its depth through the arena's view (0x346A24: 0x879420, projection
/// 0x433480's z = 12586753 / w less 64), its 768.75 pixels across a unit at w: 2 x 12586753 / (64 x 768.75) mm.
const ARENA_QUAD: f64 = 2.0 * 12_586_753.0 / (64.0 * 768.75) * SCALE;
/// The other effects' (the hits', through 0x433440 x the view, 0x346A40: z = 16782424 / w).
const HIT_QUAD: f64 = 2.0 * 16_782_424.0 / (64.0 * 768.75) * SCALE;
/// A particle's quad (metres at size 1), 0x90 / 0x48 of the particle, with its 0x90 0x400 and the 0x48 of 400 it is
/// made with.
const QUAD: f64 = 1024.0 / 400.0 * ARENA_QUAD;

/// Arena `arena`'s weather, the effect tasks 0x3A2640 starts on it (none on most arenas), in metres, frames and
/// radians, from its `entries` and their pictures' atlas `cells`:
/// - `sets`: entry k's `cell` and particle `paths` (set 6 + k, [`paths`]);
/// - `streams`: every `every` frames of the task's count a particle on a path of `set` drawn at random (none past
///   `most`), from `at`, the path scaled by `scale` per axis and turned `heading` about y (0x34ACB8), `size` metres
///   at size 1, its `colour` (1 = the texture's) and `level` (of 128), `add`ed (ALPHA mode 1, Cs x FIX + Cd, FIX
///   the level x (1 - the fade)) or blended (mode 0, the level x (1 - the fade) its alpha); `wind`: pushed each
///   frame by the wind at it times rand01 x that (0x3A08D8); `prewarm`: started as it stands a path's length on
///   (0x3A0CE8's first frame: a particle every `every` frames of the first path's);
/// - `wind` (0x3D3630, the waves summed by 0x3D3D08): each wave a Lorenz series (`dt`, sigma 10, rho 28, `beta`,
///   from (1, 1, 1); (x + 40) / 80; `step` values a frame, or one every -step frames drawn on between: 0x3D4178)
///   kept 1024 frames back; a point takes the value of `d` / `speed` frames on from the oldest, d its distance along
///   `toward` from the line across it `reach` from the origin (0x3D3F48), times `strength` along `toward`;
/// - `wisps` (0x3A16C0, 0x3A18F8): `count` particles circling the centre, k from angle k x `step`, at `radius`
///   (base + rand x spread), height `height` (base + rand x spread) + sin(phase) x `swing` (the phase from rand x
///   2pi, + rand x `bob` a frame), turning (2 rand - 1) x `turn`[0] (+- `turn`[1] more) a frame; their fade
///   (sin(f) + 1) / 2 (f from rand x `fade`[0], + rand x `fade`[1] a frame), size `size`[0] + the fade x `size`[1];
/// - `beacons` (0x3A1378, 0x3A1298): added, each at `at`, `size`, `level`, their fade (cos(phase) + 1) / 2, the
///   phase going 2pi / `period` a frame from `phase` (null: rand x pi); a period of 0 always lit.
// (the sizes and colours the code writes are constants in it)
#[allow(clippy::too_many_lines)]
pub fn weather(elf: &[u8], arena: usize, entries: &[Entry], cells: &[Option<usize>]) -> Option<Value> {
    let f = |a: usize| f32le(elf, a.checked_sub(VADDR)?).map(f64::from);
    let gp = |off: usize| f(GP - off);
    let u = |a: usize| u32le(elf, a.checked_sub(VADDR)?);
    let at = |a: usize| -> Option<Vec<f64>> { (0..3).map(|k| f(a + 4 * k).map(|v| v * SCALE)).collect() };
    // (a stream as the task makes its particles: its 0x48 `divisor`, its brightness and colour of 128)
    let stream = |every: u32,
                  at: Vec<f64>,
                  heading: f64,
                  scale: Vec<f64>,
                  colour: [f64; 3],
                  level: f64,
                  divisor: f64,
                  add: bool| {
        json!({"set": 0, "every": every, "at": at, "heading": heading, "scale": scale, "colour": colour.map(|c| c / 128.0),
            "level": level / 128.0, "size": 1024.0 / divisor * ARENA_QUAD, "add": add, "wind": 0.0, "most": null, "prewarm": false})
    };
    // (and with `more` in place of what it says)
    let with = |mut s: Value, more: Value| {
        if let (Some(s), Value::Object(more)) = (s.as_object_mut(), more) {
            s.extend(more);
        }
        s
    };
    let (mut streams, mut wisps, mut beacons, mut wind) = (vec![], vec![], vec![], Value::Null);
    match arena {
        // 0x3A0A90: six rows `{x, y, z, 1, u32 every}`; the paths 4 times their size, 0x48 450, colour (160, 160, 170),
        // the wind's push 6 (arena 4) or 25
        4 | 15 => {
            let (rows, gain) = if arena == 4 { (SMOKE_4, 6.0) } else { (SMOKE_15, 25.0) };
            for k in 0..6 {
                let r = rows + 32 * k;
                let s = stream(u(r + 16)?, at(r)?, 0.0, vec![4.0; 3], [160.0, 160.0, 170.0], 128.0, 450.0, false);
                streams.push(with(s, json!({"wind": gain, "most": 15})));
            }
            wind = self::wind(elf, arena)?;
        }
        // 0x3A0CE8: every 34 frames a particle from each of three rows `{x, y, z, 1, heading}`, the path 1.2 times
        // as long along x (gp-0x7F44), 0x48 400 x 1.43 (gp-0x7F40), added
        5 => {
            for k in 0..3 {
                let r = STEAM + 32 * k;
                let s = stream(
                    34,
                    at(r)?,
                    f(r + 16)?,
                    vec![gp(0x7F44)?, 1.0, 1.0],
                    [128.0; 3],
                    128.0,
                    400.0 * gp(0x7F40)?,
                    true,
                );
                streams.push(with(s, json!({"prewarm": true})));
            }
        }
        // 0x3A1068: every 32 frames a particle from each of three rows (48 bytes: `{x, y, z, 1}`, the scale, the
        // colour (u8 x 4), the brightness, the 0x48), turned gp-0x7F3C (pi / 2)
        8 | 17 | 20 => {
            let rows = if arena == 8 { JIN_A } else { JIN_B };
            for k in 0..3 {
                let r = rows + 48 * k;
                let c: [u8; 3] = elf.get(r + 0x20 - VADDR..r + 0x23 - VADDR)?.try_into().ok()?;
                let colour = c.map(f64::from);
                let scale = vec![f(r + 16)?, f(r + 20)?, f(r + 24)?];
                streams.push(stream(32, at(r)?, gp(0x7F3C)?, scale, colour, f(r + 0x24)?, f(r + 0x28)?, false));
            }
        }
        // 0x3A1510: every 32 frames a particle of set 7 from gp-0x7F24 (mm), scaled (20, gp-0x7F18, gp-0x7F14), 0x48 140
        11 => {
            let place = vec![gp(0x7F24)? * SCALE, gp(0x7F20)? * SCALE, gp(0x7F1C)? * SCALE];
            let s = stream(32, place, 0.0, vec![20.0, gp(0x7F18)?, gp(0x7F14)?], [128.0; 3], 128.0, 140.0, false);
            streams.push(with(s, json!({"set": 1})));
        }
        // 0x3A1B48: twenty of 0x3A16C0 and thirty of 0x3A18F8, colour (64, 64, 128), brightness 128
        14 => {
            let group = |count: usize,
                         step,
                         radius: [f64; 2],
                         height: [f64; 2],
                         swing: f64,
                         bob,
                         turn: [f64; 2],
                         fade: [f64; 2],
                         size: [f64; 2]| {
                json!({"set": 0, "count": count, "step": step, "radius": radius.map(|v| v * SCALE), "height": height.map(|v| v * SCALE),
                    "swing": swing * SCALE, "bob": bob, "turn": turn, "fade": fade, "size": size.map(|v| v * QUAD), "colour": [0.5, 0.5, 1.0], "level": 1.0})
            };
            wisps.push(group(
                20,
                gp(0x7F00)?,
                [gp(0x7F04)?, gp(0x7F08)?],
                [0.0, gp(0x7F10)?],
                gp(0x7EE8)?,
                gp(0x7EE4)?,
                [gp(0x7EFC)?, gp(0x7EF8)?],
                [gp(0x7EF0)?, gp(0x7EEC)?],
                [10.0, 8.0],
            ));
            wisps.push(group(
                30,
                gp(0x7ED4)?,
                [gp(0x7ED8)?, 2000.0],
                [gp(0x7EE0)?, 500.0],
                500.0,
                gp(0x7EBC)?,
                [gp(0x7ED0)?, gp(0x7ECC)?],
                [gp(0x7EC4)?, gp(0x7EC0)?],
                [5.0, 3.0],
            ));
        }
        _ => {}
    }
    // 0x3A1378: Nina's and King's beacons, 48-byte rows up to one whose period is negative: `u16 kind`, +0x10 the place
    // (m), +0x20 the period, +0x24 the size; kinds 0 and 1 of slot 6 (entry 0) at 160, kind 2 of slot 10 (entry 2) at
    // 240; the phase from pi (kind 1, and any of period 0) or rand x pi (kind 0)
    let rows = match arena {
        9 | 18 => BEACONS_NINA,
        11 => BEACONS_KING,
        _ => 0,
    };
    for r in (0..256).map(|k| rows + 48 * k).take_while(|&r| rows > 0 && f(r + 0x20).is_some_and(|p| p >= 0.0)) {
        let (kind, period) = (u16le(elf, r - VADDR)?, f(r + 0x20)?);
        let phase = (kind == 1 || period == 0.0).then_some(PI);
        beacons.push(json!({"set": if kind < 2 { 0 } else { 2 }, "at": [f(r + 0x10)?, f(r + 0x14)?, f(r + 0x18)?], "period": period,
            "phase": phase, "size": f(r + 0x24)? * QUAD, "level": if kind < 2 { 160.0 } else { 240.0 } / 128.0}));
    }
    // 0x3A76C0 (arena 4): the lists op 7 names, the
    // bolts' points and the flickers' (each with the bolt it lights: 0x3A7D64's channel 4 -> 0, 5 -> 3, 6 -> 1), the
    // thunder (0x1072, 0x1074, 0x1075 with the bolt lists 6-8; 0x1073 with the long flicker, list 0), how near the
    // screen's centre (pixels) a point is heard and lights the fighters (gp-0x7DF0, gp-0x7DEC: GS units of 1/16)
    let lightning = (arena == 4).then(|| -> Option<Value> {
        let lists: Vec<Vec<i16>> = (FLASH_LISTS.iter())
            .map(|&list| {
                let mut out = vec![];
                for i in 0..64 {
                    let v = i16::from_le_bytes([*elf.get(list + 2 * i - VADDR)?, *elf.get(list + 2 * i + 1 - VADDR)?]);
                    out.push(v);
                    if v < 0 {
                        break;
                    }
                }
                Some(out)
            })
            .collect::<Option<_>>()?;
        let bolts: Vec<Vec<f64>> = (0..4).map(|k| at(BOLTS + 16 * k)).collect::<Option<_>>()?;
        let flickers: Vec<Value> = [0, 3, 1].iter().enumerate().map(|(k, b)| Some(json!({"at": at(FLICKERS + 16 * k)?, "bolt": b}))).collect::<Option<_>>()?;
        Some(json!({"lists": lists, "bolts": bolts, "flickers": flickers, "thunder": [0x1072, 0x1074, 0x1075], "roll": 0x1073,
            "near": [gp(0x7DF0)? / 16.0, gp(0x7DEC)? / 16.0]}))
    }).flatten();
    // 0x3A7380 (arena 12): three tasks 0x3A7590, one a channel, now and then playing their signs' frames once
    let signs = (arena == 12).then(|| json!([0, 1, 2]));
    if streams.is_empty() && wisps.is_empty() && beacons.is_empty() && lightning.is_none() && signs.is_none() {
        return None;
    }
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let sets: Vec<Value> = (entries.iter().zip(cells))
        .map(|((_, paths), cell)| {
            let paths: Vec<Vec<[f64; 5]>> = paths.iter().map(|p| p.iter().map(|f| f.map(round)).collect()).collect();
            json!({"cell": cell, "paths": paths})
        })
        .collect();
    Some(
        json!({"sets": sets, "streams": streams, "wind": wind, "wisps": wisps, "beacons": beacons, "lightning": lightning, "signs": signs}),
    )
}

/// Arena `arena`'s wind (0x3D3630, the waves 0x3D3D08 sums; [`weather`]'s `wind`): `dt`, `beta`, `reach` and its
/// `waves` (`step`, `speed`, `strength`, `toward`), metres.
pub fn wind(elf: &[u8], arena: usize) -> Option<Value> {
    let f = |a: usize| f32le(elf, a.checked_sub(VADDR)?).map(f64::from);
    let u = |a: usize| u32le(elf, a.checked_sub(VADDR)?);
    let gp = |off: usize| f(GP - off);
    let row = WIND + 12 * arena;
    let list = u(row + 4)? as usize;
    let waves: Vec<Value> = (0..(u(row)? as usize).min(3))
        .map(|k| {
            let w = list + 32 * k;
            let (x, z) = (f(w + 16)?, f(w + 24)?);
            let n = (x * x + z * z).sqrt();
            (n > 0.0).then_some(())?;
            Some(json!({"step": u(w)? as i32, "speed": f(w + 4)? * SCALE, "strength": f(w + 8)? * SCALE, "toward": [x / n, z / n]}))
        })
        .collect::<Option<_>>()?;
    Some(json!({"dt": gp(0x7D30)?, "beta": gp(0x7D2C)?, "reach": gp(0x7D4C)?.sqrt() * SCALE, "waves": waves}))
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
/// The arenas' own pictures `arena` follow, from cell 24 ([`weather`]). None when a picture is missing.
pub fn atlas(
    pics: &[Vec<(usize, usize, Vec<u8>)>],
    sets: &[Vec<Vec<[f64; 5]>>],
    arena: &[(usize, usize, Vec<u8>)],
) -> Option<(Vec<u8>, Value)> {
    let mut cells: Vec<&(usize, usize, Vec<u8>)> =
        (0..4).map(|k| pics.get(k)?.first()).chain(pics.get(4)?.iter().take(20).map(Some)).collect::<Option<_>>()?;
    if cells.len() != 24 {
        return None;
    }
    cells.extend(arena);
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
        json!({"cell": cell, "size": 128.0 / 400.0 * HIT_QUAD, "level": level, "random": random, "paths": paths})
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
        let m = Model { nodes, sections: vec![vec![Strip { tex0: 0, blend: false, verts }]], wings: None };
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
        let (_, j) = atlas(&pics, &[vec![vec![[0.1, 0.2, 0.3, 1.0, 0.5]]]], &pic(64, 9)).unwrap();
        // (the spark's paths at half their spread)
        assert_eq!(j["bursts"]["spark"]["paths"], json!([[[0.05, 0.1, 0.15, 1.0, 0.5]]]));
        assert_eq!((&j["bursts"]["block"]["level"], &j["bursts"]["zero"]["paths"]), (&0.25.into(), &json!([])));
        assert_eq!((j["atlas"]["cols"].as_u64(), j["atlas"]["rows"].as_u64()), (Some(8), Some(4)));
        assert_eq!(j["strips"]["heavy_b"]["cells"][0], 14);
        assert_eq!(j["strips"]["dust"]["cells"].as_array().map(Vec::len), Some(150));
        pics.pop();
        assert!(atlas(&pics, &[], &[]).is_none());
    }
}

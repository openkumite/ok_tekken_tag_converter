//! The endings, the staff roll and Theater (docs/BOWL_ENDINGS.md 7-8): the executable's tables as
//! `assets/gamedata/endings.json`, and each ending's scene files (camera path 2e + 47, motion pack 2e + 48) as
//! `assets/endings/`: the camera path as on the disc, the body tracks as clips, the face and hand tracks as on the
//! disc (docs/GAMEDATA.md "endings").

use crate::archive;
use crate::moves::VADDR;
use crate::{anim, glb, u16_at, u16le, u32_at};
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// The ending scenes (descriptors 0x51B488).
pub const COUNT: usize = 38;
/// The EMBU's scene files (scene 1, 0x3A97B0): its 22 camera paths and its motion pack.
pub const EMBU: (usize, usize) = (41, 42);
/// The scene files of ending `e`: its camera path and its motion pack.
pub fn files(e: usize) -> (usize, usize) {
    (2 * e + 47, 2 * e + 48)
}

/// A string of the executable (None for a null pointer or one outside it).
fn string(elf: &[u8], pointer: u32) -> Option<String> {
    let at = (pointer as usize).checked_sub(VADDR)?;
    let raw: Vec<u8> = elf.get(at..)?.iter().take(96).take_while(|&&c| c != 0).copied().collect();
    Some(String::from_utf8_lossy(&raw).to_string())
}

/// `gamedata/endings.json`: the ending table 0x51B678 (character x costume -> ending, -1 none), the 38 descriptors
/// (arena, cast `[character, costume]` with -1 the player's, script `[frame, op, a, b, c]` to op -1), the staff roll
/// 0x51B880, Theater's labels 0x455E50 and the music player's list 0x455D80, and the playback's constants.
pub fn endings(elf: &[u8]) -> Result<Value> {
    let what = "the endings' tables";
    let a = |addr: usize| addr - VADDR;
    let s16 = |at: usize| -> Result<i16> { Ok(u16_at(elf, at, what)? as i16) };
    let ptr = |at: usize| -> Result<usize> { (u32_at(elf, at, what)? as usize).checked_sub(VADDR).context(what) };
    // a script: 10-byte records to op -1
    let script_at = |at: usize| -> Result<Vec<Vec<i16>>> {
        let mut records = vec![];
        for k in 0..512 {
            let r: Vec<i16> = (0..5).map(|i| s16(at + 10 * k + 2 * i)).collect::<Result<_>>()?;
            let end = r.get(1) == Some(&-1);
            records.push(r);
            if end {
                break;
            }
        }
        Ok(records)
    };
    let table: Vec<Vec<i8>> = (0..34)
        .map(|c| (0..8).map(|k| crate::u8_at(elf, a(0x51B678) + 8 * c + k, what).map(|v| v as i8)).collect())
        .collect::<Result<_>>()?;
    // the people an ending shows (0x3D85D8: case e - 2 of jump table 0x521C30, `addiu v0, zero, set` in its delay
    // slot; set -1, and endings 0 and 1, none), and the EMBU's (0x3D868C: set 30 on arena 13, else none)
    let people = |set: Option<i16>, arena: usize| {
        let set = usize::try_from(set?).ok()?;
        let (people, walk) = crate::effects::people(elf, set, arena);
        Some(json!({"set": set, "people": people, "walk": walk}))
    };
    let set = |e: usize| -> Option<i16> {
        let k = e.checked_sub(2).filter(|&k| k < 0x23)?;
        let case = (u32_at(elf, a(0x521C30) + 4 * k, what).ok()? as usize).checked_sub(VADDR)?;
        let op = u32_at(elf, case + 4, what).ok()?;
        (op >> 16 == 0x2402).then_some(op as u16 as i16)
    };
    let mut scenes = vec![];
    for e in 0..COUNT {
        let r = a(0x51B488) + 12 * e;
        let (script, arena, cast) = (ptr(r)?, u16_at(elf, r + 4, what)?, ptr(r + 8)?);
        let mut members = vec![];
        for k in 0..8 {
            let c = s16(cast + 4 * k)?;
            if c < 0 {
                break;
            }
            members.push(json!([c, s16(cast + 4 * k + 2)?]));
        }
        let records = script_at(script)?;
        let (camera, pack) = files(e);
        scenes.push(json!({"arena": arena, "cast": members, "script": records, "camera_file": camera, "pack_file": pack,
            "people": people(set(e), arena as usize),
            "camera": format!("ending_{e:02}_camera.bin"), "body": [format!("ending_{e:02}_0.glb"), format!("ending_{e:02}_1.glb")],
            "face": format!("ending_{e:02}_face.bin"), "hands": format!("ending_{e:02}_hands.bin")}));
    }
    // the staff roll: {u32 kind, char *text, u32 font}; font bit 16 marks the pair of which one shows
    let mut roll = vec![];
    for k in 0..434 {
        let r = a(0x51B880) + 12 * k;
        let (kind, text, font) = (u32_at(elf, r, what)?, u32_at(elf, r + 4, what)?, u32_at(elf, r + 8, what)?);
        roll.push(json!({"kind": kind, "text": string(elf, text).unwrap_or_default(), "font": font & 0xFFFF,
            "alternative": font & 0x10000 != 0}));
    }
    let labels: Vec<Vec<Option<String>>> = (0..34)
        .map(|c| (0..8).map(|k| u32_at(elf, a(0x455E50) + 4 * (8 * c + k), what).map(|p| string(elf, p))).collect())
        .collect::<Result<_>>()?;
    let mut music = vec![];
    for k in 0..32 {
        let r = a(0x455D80) + 8 * k;
        let (bgm, name) = (u32_at(elf, r, what)? as i32, u32_at(elf, r + 4, what)?);
        music.push(json!({"bgm": bgm, "name": string(elf, name).unwrap_or_default()}));
        if bgm == 254 {
            break;
        }
    }
    // the EMBU (0x3A9AB8): its two casts of six (0x437CE0), arena 4 for the first and 13 for the second (0x3A97CC,
    // 0x3A9820), its script (0x519B90, interpreted by 0x3A9E48: table 0x519B50)
    let casts: Vec<Vec<[i16; 2]>> = (0..2)
        .map(|c| {
            (0..6)
                .map(|k| Ok([s16(a(0x437CE0) + 0x18 * c + 4 * k)?, s16(a(0x437CE0) + 0x18 * c + 4 * k + 2)?]))
                .collect()
        })
        .collect::<Result<_>>()?;
    let embu = json!({"casts": casts, "arenas": [4, 13], "script": script_at(a(0x519B90))?, "bgm": 1,
        "camera_file": EMBU.0, "pack_file": EMBU.1, "camera": "embu_camera.bin", "body": [],
        "face": "embu_face.bin", "hands": "embu_hands.bin", "people": [people(None, 4), people(Some(30), 13)]});
    Ok(json!({
        "table": table, "endings": scenes, "embu": embu,
        "staff_roll": {"lines": roll, "cards": ["TEKKEN TAG TOURNAMENT", "NAMCO PRODUCTION STAFF"], "card_frames": 170,
            "frames": 4620, "scroll_px": 1.5, "line_px": 32, "start_y": 480, "bgm": 6},
        "playback": {"ending_bgm": 7, "fade_in": 45, "skip_fade": 4, "max_frames": 7200, "unknown": 37, "unknown_movie": 16},
        "theater": {"labels": labels, "music": music,
            "extras": [{"label": "OPENING MOVIE", "movie": 14}, {"label": "ARCADE MOVIE", "movie": 15},
                {"label": "EMBU", "scene": 1}, {"label": "EMBU2", "scene": 1, "cast": 1}]},
    }))
}

/// A scene's motion pack: its three parts (body, face, hands), each a pack of tracks `u16 frames, u16 triples`, then
/// the frames. The body tracks (19 triples, a motion's 57 channels) as clips `NAME_K` (an ending's `ending_EE`, K
/// the actor slot; the EMBU's `embu`, K the track its script names), with track K's hands: hand tracks 2K and 2K + 1.
pub fn body_clips(name: &str, d: &[u8]) -> Vec<Vec<u8>> {
    let parts = archive::pack(d).unwrap_or_default();
    let tracks = parts.first().and_then(|p| archive::pack(p)).unwrap_or_default();
    let faces = parts.get(1).and_then(|p| archive::pack(p)).unwrap_or_default();
    let hands = parts.get(2).and_then(|p| archive::pack(p)).unwrap_or_default();
    tracks
        .iter()
        .enumerate()
        .filter_map(|(k, t)| {
            let mut fingers = faces.get(k).map(|f| face(f)).unwrap_or_default();
            for side in 0..2 {
                fingers.extend(hands.get(2 * k + side).map(|h| hand(h, side == 1)).unwrap_or_default());
            }
            Some(anim::clip_glb_with(&format!("{name}_{k}"), &anim::raw(t)?, &fingers))
        })
        .collect()
}

/// A hand track (15 triples a frame: the 5 fingers' 3 joints, 65536 a turn) as its finger nodes' rotations
/// (0x3DE0D8): track 2K turns hand_A's nodes 39-53, 2K + 1 hand_B's 64-78; each triple's matrix (0x349D70, as a
/// joint's) replaces the node's turn, its bind position kept (0x34A180 at weight 1 in a scene, 0x3634F4). hand_A
/// takes (-x, -y, z) as the body does (0x3DD788); hand_B mirrors by joint (table 0x522CC0): its first (x - pi, -y,
/// -z), the next two as hand_A, then (x, -y, -z), (x, -y, -z), (-x, -y, -z) a finger.
fn hand(t: &[u8], b: bool) -> Vec<(String, anim::Keys)> {
    if u16le(t, 2) != Some(15) {
        return vec![];
    }
    // (whole frames only)
    let frames = (u16le(t, 0).unwrap_or(0) as usize).min(t.len().saturating_sub(4) / 90);
    let k = std::f64::consts::TAU / 65536.0;
    let at = |f: usize, j: usize, i: usize| u16le(t, 4 + 2 * (3 * (15 * f + j) + i)).map(|v| v as i16 as f64 * k);
    (0..15)
        .map(|j| {
            let q = (0..frames)
                .map_while(|f| {
                    let [x, y, z] = [at(f, j, 0)?, at(f, j, 1)?, at(f, j, 2)?];
                    let r = match (b, j) {
                        (false, _) | (true, 1 | 2) => [-x, -y, z],
                        (true, 0) => [x - std::f64::consts::PI, -y, -z],
                        (true, _) if j % 3 == 2 => [-x, -y, -z],
                        (true, _) => [x, -y, -z],
                    };
                    Some(glb::quat(&glb::joint(r, [0.; 3])))
                })
                .collect();
            (format!("node_{}", if b { 64 } else { 39 } + j), anim::Keys::Rotation(q))
        })
        .collect()
}

/// A face track (33 triples a frame, BOWL_ENDINGS.md 7.3) as the face nodes 7-28's keys (0x3DD918, table 0x522C60):
/// a triple's turn (0x3DD788, as the hands': (-x, -y, z), 65536 a turn) replaces the node's; a scale (0x3DD800: u16 /
/// 4096 a row of the node's bind turn) and an offset (0x3DD890: s16 x fighter +0x500, 4096, / 40960 mm, the node's
/// place under the head) replace its scale and place. The eyes (13, 14) take their turn and their rows' scale (t3, t4;
/// the VU0 code's blend with the last frame is at weight 1 in a scene).
// shortcut: node 14's place from t3 times the fighter's eye vector (0x46AFD0 + 16 x +0x20, set as the eyes aim) is
// left at its bind place; node 13's rows scale in its parent's axes (0x34A9F0), taken in its own.
pub fn face(t: &[u8]) -> Vec<(String, anim::Keys)> {
    if u16le(t, 2) != Some(33) {
        return vec![];
    }
    let frames = (u16le(t, 0).unwrap_or(0) as usize).min(t.len().saturating_sub(4) / 198);
    let at = |f: usize, j: usize, i: usize| u16le(t, 4 + 2 * (3 * (33 * f + j) + i)).unwrap_or(0);
    let k = std::f64::consts::TAU / 65536.0;
    let turn = |j: usize| {
        let keys = (0..frames).map(|f| {
            let [x, y, z] = [0, 1, 2].map(|i| at(f, j, i) as i16 as f64 * k);
            glb::quat(&glb::joint([-x, -y, z], [0.; 3]))
        });
        anim::Keys::Rotation(keys.collect())
    };
    let scale =
        |j: usize| anim::Keys::Scale((0..frames).map(|f| [0, 1, 2].map(|i| at(f, j, i) as f32 / 4096.0)).collect());
    let place = |j: usize| {
        anim::Keys::Translation(
            (0..frames).map(|f| [0, 1, 2].map(|i| at(f, j, i) as i16 as f32 / 10.0 / 1000.0)).collect(),
        )
    };
    // (node, its keys by triple)
    let mut out: Vec<(usize, anim::Keys)> = vec![];
    for (n, j) in [(7, 0), (21, 7), (22, 8), (25, 9), (26, 10), (17, 19), (18, 20), (13, 5), (14, 6)] {
        out.push((n, turn(j)));
    }
    for (n, s, p) in [(8, 29, 31), (9, 30, 32), (10, 1, 2), (11, 25, 27), (12, 26, 28), (15, 21, 23), (16, 22, 24)] {
        out.push((n, scale(s)));
        out.push((n, place(p)));
    }
    for (n, p) in [(17, 17), (18, 18), (19, 13), (20, 14), (23, 11), (24, 12), (27, 15), (28, 16)] {
        out.push((n, place(p)));
    }
    out.push((13, scale(3)));
    out.push((14, scale(4)));
    out.into_iter().map(|(n, keys)| (format!("node_{n}"), keys)).collect()
}

/// A fighter's hand poses (its pack's part 4, a track of 15 triples a pose: 0 open to 16 the fist, then its own; the
/// fight reads pose +0x306C of it, 0x364390) as clip `hands` of a key a pose: hand_A's nodes and hand_B's as a
/// scene's two tracks turn them (the body's channels at rest). None without a pose.
pub fn hands_glb(t: &[u8]) -> Option<Vec<u8>> {
    let mut fingers = hand(t, false);
    fingers.extend(hand(t, true));
    let poses = fingers.first().map_or(0, |f| f.1.len());
    (poses > 0).then(|| anim::clip_glb_with("hands", &vec![[0; anim::CHANNELS]; poses], &fingers))
}

/// Part `k` (1: the face, 2: the hands) of a scene's motion pack, as on the disc.
pub fn part(d: &[u8], k: usize) -> Option<&[u8]> {
    archive::pack(d)?.get(k).copied()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn scene_files_and_broken_data() {
        assert_eq!(files(0), (47, 48));
        assert_eq!(files(37), (121, 122));
        assert!(body_clips("ending_00", &[0; 40]).is_empty());
        assert!(endings(&[0; 64]).is_err());
    }

    #[test]
    fn hands_by_side() {
        // one frame, 15 triples: the thumb's root turned a quarter about z
        let mut t = vec![1, 0, 15, 0];
        t.extend([0u8; 90]);
        t[8..10].copy_from_slice(&16384i16.to_le_bytes());
        let rot = |v: Vec<(String, anim::Keys)>| -> Vec<(String, Vec<[f32; 4]>)> {
            v.into_iter().map(|(n, k)| (n, if let anim::Keys::Rotation(q) = k { q } else { vec![] })).collect()
        };
        let (a, b) = (rot(hand(&t, false)), rot(hand(&t, true)));
        assert_eq!((a[0].0.as_str(), b[14].0.as_str()), ("node_39", "node_78"));
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let near = |q: [f32; 4], r: [f32; 4]| q.iter().zip(r).all(|(x, y)| (x - y).abs() < 1e-4);
        // hand_A: +z; hand_B's first: x - pi, then -z
        assert!(near(a[0].1[0], [0.0, 0.0, h, h]), "{:?}", a[0].1);
        assert!(near(b[0].1[0], [-h, h, 0.0, 0.0]) || near(b[0].1[0], [h, -h, 0.0, 0.0]), "{:?}", b[0].1);
        assert!(near(a[1].1[0], [0.0, 0.0, 0.0, 1.0]) && near(b[1].1[0], [0.0, 0.0, 0.0, 1.0]));
        assert!(rot(hand(&t[..50], false)).iter().all(|(_, q)| q.is_empty()));
        // a fighter's poses: 30 finger nodes, a key a pose
        let (doc, _, _) = gltf::import_slice(hands_glb(&t).unwrap()).unwrap();
        let a = doc.animations().next().unwrap();
        assert_eq!(
            (a.name(), a.channels().filter(|c| c.target().node().name().unwrap().starts_with("node_")).count()),
            (Some("hands"), 30)
        );
        assert!(hands_glb(&t[..50]).is_none());
    }

    /// The face: node 7 turned by triple 0, node 10 scaled by triple 1 and placed by triple 2 (tenths of a mm), one
    /// node a name in the clip.
    #[test]
    fn a_face_track_turns_scales_and_places() {
        let mut t = vec![1, 0, 33, 0];
        t.extend([0u8; 198]);
        let put = |t: &mut Vec<u8>, j: usize, v: [i16; 3]| {
            for (i, x) in v.iter().enumerate() {
                t[4 + 6 * j + 2 * i..6 + 6 * j + 2 * i].copy_from_slice(&x.to_le_bytes());
            }
        };
        put(&mut t, 0, [0, 0, 16384]);
        put(&mut t, 1, [8192, 4096, 4096]);
        put(&mut t, 2, [1019, -480, 849]);
        let f = face(&t);
        let of = |n: &str| f.iter().filter(|e| e.0 == n).map(|e| &e.1).collect::<Vec<_>>();
        let h = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            matches!(of("node_7")[..], [anim::Keys::Rotation(q)] if (q[0][2] - h).abs() < 1e-4 && (q[0][3] - h).abs() < 1e-4)
        );
        assert!(matches!(of("node_10")[..], [anim::Keys::Scale(s), anim::Keys::Translation(p)]
            if s[0] == [2.0, 1.0, 1.0] && (p[0][0] - 0.1019).abs() < 1e-6 && (p[0][1] + 0.048).abs() < 1e-6));
        assert!(face(&t[..100]).iter().all(|(_, k)| k.is_empty()));
        let glb = anim::clip_glb_with("x", &[[0; anim::CHANNELS]], &f);
        let (doc, _, _) = gltf::import_slice(glb).unwrap();
        let named: Vec<_> =
            doc.nodes().filter_map(|n| n.name().map(str::to_string)).filter(|n| n == "node_10").collect();
        assert_eq!(named.len(), 1);
    }
}

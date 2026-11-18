//! The endings, the staff roll and Theater (docs/BOWL_ENDINGS.md 7-8): the executable's tables as
//! `assets/gamedata/endings.json`, and each ending's scene files (camera path 2e + 47, motion pack 2e + 48) as
//! `assets/endings/`: the camera path as on the disc, the body tracks as clips, the face and hand tracks as on the
//! disc (docs/GAMEDATA.md "endings").

use crate::archive;
use crate::moves::VADDR;
use crate::{anim, u16_at, u32_at};
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// The ending scenes (descriptors 0x51B488).
pub const COUNT: usize = 38;
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
    let table: Vec<Vec<i8>> = (0..34)
        .map(|c| (0..8).map(|k| crate::u8_at(elf, a(0x51B678) + 8 * c + k, what).map(|v| v as i8)).collect())
        .collect::<Result<_>>()?;
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
        let mut records = vec![];
        for k in 0..512 {
            let r: Vec<i16> = (0..5).map(|i| s16(script + 10 * k + 2 * i)).collect::<Result<_>>()?;
            let end = r.get(1) == Some(&-1);
            records.push(r);
            if end {
                break;
            }
        }
        let (camera, pack) = files(e);
        scenes.push(json!({"arena": arena, "cast": members, "script": records, "camera_file": camera, "pack_file": pack,
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
    Ok(json!({
        "table": table, "endings": scenes,
        "staff_roll": {"lines": roll, "cards": ["TEKKEN TAG TOURNAMENT", "NAMCO PRODUCTION STAFF"], "card_frames": 170,
            "frames": 4620, "scroll_px": 1.5, "line_px": 32, "start_y": 480, "bgm": 6},
        "playback": {"ending_bgm": 7, "fade_in": 45, "skip_fade": 4, "max_frames": 7200, "unknown": 37, "unknown_movie": 16},
        "theater": {"labels": labels, "music": music,
            "extras": [{"label": "OPENING MOVIE", "movie": 14}, {"label": "ARCADE MOVIE", "movie": 15},
                {"label": "EMBU", "scene": 1}, {"label": "EMBU2", "scene": 1, "cast": 1}]},
    }))
}

/// A scene's motion pack: its three parts (body, face, hands), each a pack of tracks `u16 frames, u16 triples`, then
/// the frames. The body tracks (19 triples, a motion's 57 channels) as clips `ending_EE_K` (K: the actor slot).
pub fn body_clips(e: usize, d: &[u8]) -> Vec<Vec<u8>> {
    let parts = archive::pack(d).unwrap_or_default();
    let tracks = parts.first().and_then(|p| archive::pack(p)).unwrap_or_default();
    tracks
        .iter()
        .enumerate()
        .filter_map(|(k, t)| Some(anim::clip_glb(&format!("ending_{e:02}_{k}"), &anim::raw(t)?)))
        .collect()
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
        assert!(body_clips(0, &[0; 40]).is_empty());
        assert!(endings(&[0; 64]).is_err());
    }
}

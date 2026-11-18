//! The sound and music tables in the executable (docs/FIGHT_DATA.md section 14), written as OpenKumite's
//! `gamedata/sound.json` and `gamedata/music.json` (docs/GAMEDATA.md "assets/"): Tekken 3's keys where the game kept
//! the concept, but every number is Tekken Tag's own (sound ids, not Tekken 3's SePlay codes).

use crate::fight::{SOUNDS, h, until};
use crate::moves::{MOVE_COUNT, MOVES, VADDR};
use crate::{bytes, u8_at, u16_at, u32_at};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

// (address in memory - VADDR = offset in the file; entries)
const GROUPS: (usize, usize) = (0x42ebd8 - VADDR, 62); // 16 bytes: u32 samples, u32 bank bytes, u16 kind, u16 first id, u16 count
const VOICE_GROUP: usize = 23; // groups 23-61 are the 39 voice rows' banks
const VOICES: usize = 0x501808 - VADDR; // a voice row's 16 bytes: u32 -> 4 x s16 counts then the ids; u16 cooldown at +0xC
const HIT_ARMS: (usize, usize) = (0x500d30 - VADDR, 0x500d10 - VADDR); // 4 damage bands x 4 u16 entries: most bodies, bodies 5 7 10 18
const FLOORS: (usize, usize) = (0x5007f8 - VADDR, 21); // u16 landing, step, step per arena
const ROUND_NUMBERS: usize = 0x500458 - VADDR; // 9 u16 entries, by round
const SCRIPTS: (usize, usize) = (0x505c10 - VADDR, 0x501a78 - VADDR); // u16 first step; steps {u8 frame, u8 kind, u16 sound} to a zero one
const BGM: (usize, usize) = (0x50cd00 - VADDR, 28); // 10 bytes: u8 kind, s8, u16, s16, u16 file of TEKKEN.BIN, s16 loop sector
const BGM_BY_ARENA: (usize, usize) = (0x4fb9c0 - VADDR, 23); // s16 per arena
/// BGM id 7 is the piece of its file from this sector on (0x397AB8: the endings' music within the staff roll's file).
const ENDINGS_PIECE: (usize, i64) = (7, 0xcf7);
/// A stream sector: 1024 bytes a channel, 64 ADPCM frames of 28 samples, at 44 100 Hz.
const SECTOR_SECONDS: f64 = 64.0 * 28.0 / 44100.0;

/// The bank file of a non-voice group: 0 the common one, 1-20 arena k - 1's (187 + 7k), 21 Tekken Bowl's, 22 file 45.
fn group_bank(g: usize) -> Option<usize> {
    match g {
        0 => Some(1),
        1..=20 => Some(187 + 7 * (g - 1)),
        21 => Some(327), // (334 holds the same)
        22 => Some(45),
        _ => None,
    }
}

/// u16 entries of the executable, as a JSON list.
fn words(elf: &[u8], at: usize, n: usize, what: &str) -> Result<Vec<u16>> {
    (0..n).map(|k| u16_at(elf, at + 2 * k, what)).collect()
}

/// sound.json. `banks`: each sound bank written (its file in TEKKEN.BIN, its bytes, its sample count); `characters`:
/// the character files written (`id` their file number: their voice bank is the file before).
pub fn sound(elf: &[u8], banks: &[(usize, usize, usize)], characters: &[Value]) -> Result<Value> {
    let what = "the sound groups";
    let mut groups = Vec::with_capacity(GROUPS.1);
    let mut sizes = Vec::with_capacity(GROUPS.1); // (bank bytes, count)
    for g in 0..GROUPS.1 {
        let e = bytes(elf, GROUPS.0 + 16 * g, 16, what)?;
        let (p, size, kind, first, count) =
            (u32_at(e, 0, what)?, u32_at(e, 4, what)?, u16_at(e, 8, what)?, u16_at(e, 10, what)?, u16_at(e, 12, what)?);
        let samples = (0..count as usize)
            .map(|k| {
                let s = bytes(elf, (p as usize).wrapping_sub(VADDR).saturating_add(8 * k), 8, what)?;
                Ok(json!({"pitch": u16_at(s, 4, what)?, "volume": u8_at(s, 6, what)?, "channel": u8_at(s, 7, what)?}))
            })
            .collect::<Result<Vec<_>>>()?;
        let voice = g.checked_sub(VOICE_GROUP);
        groups.push(json!({"group": g, "first_id": first, "count": count, "kind": kind, "bank_bytes": size,
                           "bank": group_bank(g), "voice_row": voice, "samples": samples}));
        sizes.push((size as usize, count as usize));
    }
    // a character's voice row: the one whose bank its own voice bank is (the roster's byte 6 = player +0x2C picks it in
    // the game; costumes such as Tiger's have another fighter's)
    let voice_banks: Vec<Value> = (characters.iter())
        .filter_map(|c| c.get("id")?.as_u64())
        .map(|id| {
            let bank = (id as usize).saturating_sub(1);
            let own = banks.iter().find(|b| b.0 == bank).map(|b| (b.1, b.2));
            let row = sizes.iter().skip(VOICE_GROUP).position(|s| Some(*s) == own);
            json!({"model_record": id, "bank": bank, "voice_row": row})
        })
        .collect();
    let what = "the voice rows";
    let mut rows = Vec::with_capacity(GROUPS.1 - VOICE_GROUP);
    for row in 0..GROUPS.1 - VOICE_GROUP {
        let p = (u32_at(elf, VOICES + 16 * row, what)? as usize).wrapping_sub(VADDR);
        let counts = words(elf, p, 4, what)?;
        let n: usize = counts.iter().map(|&c| c as usize).sum();
        ensure!(n <= 256, "voice row {row}: {n} voices");
        rows.push(json!({"row": row, "counts": counts, "codes": words(elf, p.saturating_add(8), n, what)?,
                         "cooldown": u16_at(elf, VOICES + 16 * row + 0xc, what)?}));
    }
    let what = "the hit sounds";
    let arms = |at: usize| (0..4).map(|k| words(elf, at + 8 * k, 4, what)).collect::<Result<Vec<_>>>();
    let floors =
        (0..FLOORS.1).map(|k| words(elf, FLOORS.0 + 6 * k, 3, "the floor sounds")).collect::<Result<Vec<_>>>()?;
    // the timed scripts the moves' type-8 entries name
    let mut last = None;
    for i in 0..MOVE_COUNT {
        let list = crate::u32le(bytes(elf, MOVES + 52 * i, 52, "the moves")?, 0x1c).unwrap_or(!0);
        if list != !0 {
            let entries = until(elf, SOUNDS, list, 2, "sound list", |e| h(e, 0) == 0xffff)?;
            last = entries.iter().map(|e| h(e, 0)).filter(|e| e >> 12 == 8).map(|e| e & 0xfff).chain(last).max();
        }
    }
    let scripts = (0..last.map_or(0, |l| l as u32 + 1))
        .map(|s| {
            let first = u16_at(elf, SCRIPTS.0 + 2 * s as usize, "the sound scripts")?;
            let steps = until(elf, SCRIPTS.1, first as u32, 4, "sound script", |e| e == [0; 4])?;
            Ok(steps
                .iter()
                .map(|e| [e.first().copied().unwrap_or(0) as u16, e.get(1).copied().unwrap_or(0) as u16, h(e, 2)])
                .collect())
        })
        .collect::<Result<Vec<Vec<[u16; 3]>>>>()?;
    Ok(json!({
        "dialect": "ttt",
        "id_format": "an entry is type << 12 | id; a sound id (id & 0xFFF, 0 = none) is sample id - first_id of the group that holds it (0x374B48); ids from 0x321 depend on the arena (arena_sounds)",
        "groups": {"address": "0x42EBD8", "routine": "0x374B48",
            "note": "kind 0 resident, 1 loaded with the arena, 2 a voice bank; sample n of a group is sample n of its bank (audio/sfx/NNN/, NNN = bank); a voice group's bank is its character's (voice_banks); pitch 0x1000 = 48 kHz, channel classes 6 and 8 take one voice per side",
            "entries": groups},
        "voice_banks": {"note": "each character file's voice bank (the file before it in TEKKEN.BIN) and the voice row whose ids play from it", "characters": voice_banks},
        "voice_rows": {"address": "0x501808", "stride": 16, "by": "player +0x2C: the roster's byte 6, not the body",
            "note": "entry types 2-5 are categories 0-3: id codes[sum(counts[:cat]) + index]. Types 2 and 3 play on the move's frame 1, type 2 only when the voice cooldown is out, and sets it to `cooldown`; type 5 at once (0x375B60). codes are sound ids",
            "rows": rows},
        "entry_types": {"routine": "0x375B60 (jump table 0x500CC0), up to 3 entries a frame",
            "0": "nothing", "1": "hit sound (as 7)", "2": "voice category 0, frame 1, cooldown", "3": "voice category 1, frame 1",
            "4": "effect id on the fighter's side, each frame it is reached", "5": "voice category 3 at once",
            "6": "nothing (Tekken 3's swing)", "7": "hit sound: on the frame the move connects, on the victim's side (impact_rules)",
            "8": "timed script (scripts)", "9": "0x376578(self, opponent, id): not decoded"},
        "impact_rules": {"routine": "0x3769D0",
            "format": "entries (7 << 12 | id); an arm: the move's end bone (+0x28) below 0xC",
            "blocked": 0x7010, "ko": [0x700a, 0x700d], "ko_arm_end_bone_below": 0xb,
            "auto": [0x7000, 0x7006, 0x7007, 0x7009], "damage_bands": [13, 23, 33],
            "legs": [0x7009, 0x700c, 0x700b, 0x700d],
            "arms": {"table": "0x500D30, bodies 5 7 10 18 (attacker +0x2A) 0x500D10", "rule": "band 0: column 0 or 1 by rand(); the other bands column 0",
                     "rows": arms(HIT_ARMS.0)?, "bodies": [5, 7, 10, 18], "rows_of_bodies": arms(HIT_ARMS.1)?},
            "special_victims": {"note": "victims whose voice row is 16, 17 (Mokujin, Tetsujin) or 18, 30, 34 (the Jacks) take handlers 0x376818, 0x376790, 0x376728: not decoded", "rows": [16, 17, 18, 30, 34]}},
        "ko_voice": {"routine": "0x375BC0", "rule": "player +0x138 set: a category-2 voice (rand() & 0xFF) * counts[2] >> 8, then the fighter is silent (+0xB6)"},
        "move_sounds": {"format": "entries (type << 12 | id)", "take_off": 0x1012, "take_off_when": "the move's jump start frame (+0x19)",
            "footsteps": [0x1321, 0x1321], "footsteps_gap_frames": 4, "footsteps_when": "player +0x136", "landing": 0x1322, "landing_when": "player +0x137"},
        "arena_sounds": {"routine": "0x3755C8 (jump table 0x500900)", "address": "0x5007F8",
            "note": "id 0x322: the arena's landing (by_arena[arena][0]); 0x321: its footstep, by_arena[arena][1] and [2] in turn; the arena number is gp-0x743A",
            "by_arena": floors,
            "specials": [{"id": 0x323, "arenas": [0], "sounds": [0x63]}, {"id": 0x324, "arenas": [9, 11, 18], "sounds": [0x85, 0x8c, 0xa4]},
                         {"id": 0x325, "arenas": [1, 15], "sounds": [0x67]}, {"id": 0x326, "arenas": [1, 15], "sounds": [0x9a]}]},
        "scripts": {"address": "0x505C10 -> 0x501A78", "routine": "0x37623C",
            "format": "[frame, kind, sound]: kind 0 an effect, 1 an entry of the fighter's own (voice or effect), 2 of the opponent's, 3 0x376578; sound 0x13 starts a footstep pair",
            "entries": scripts},
        "announcer": {"note": "sound entries (1 << 12 | id) the round code plays (0x36EC50)", "round": 0x105a, "final_round": 0x105c, "fight": 0x105b,
            "round_numbers": {"address": "0x500458", "by_round": words(elf, ROUND_NUMBERS, 9, "the round numbers")?, "modes_5_6": 0x1050},
            "call_frames": {"first_round": {"round": 61, "number": 100, "fight": 150}, "other_rounds": {"round": 1, "number": 40, "fight": 90},
                "final_round_when": "round number >= round limit and neither side solo: final_round, no number"},
            "result": {"routine": "0x36EEE0", "order": ["time_up", "perfect", "double_ko", "great", "ko"],
                       "time_up": 0x1054, "perfect": 0x1058, "double_ko": 0, "great": 0x1057, "ko": 0x1059}},
    }))
}

/// music.json: the BGM table (what the sound driver RSPU2DRV.IRX gets at boot, 0x397A00) and each arena's BGM.
/// `files`: the music written, by its file in TEKKEN.BIN.
pub fn music(elf: &[u8], files: &[usize]) -> Result<Value> {
    let what = "the BGM table";
    let mut bgm = Vec::with_capacity(BGM.1);
    for id in 0..BGM.1 {
        let e = bytes(elf, BGM.0 + 10 * id, 10, what)?;
        let s = |o| u16_at(e, o, what).map(|v| v as i16);
        let (kind, file, loop_sector) = (u8_at(e, 0, what)?, u16_at(e, 6, what)? as usize, s(8)?);
        let start = if id == ENDINGS_PIECE.0 { ENDINGS_PIECE.1 } else { 0 };
        // the driver reads a negative loop sector as 0
        let column = json!({"raw": [kind, u8_at(e, 1, what)? as i8, u16_at(e, 2, what)?, s(4)?, file, loop_sector],
            "kind": kind, "source_file": file, "file": files.contains(&file).then(|| format!("{file:02}.flac")),
            "start_seconds": start as f64 * SECTOR_SECONDS, "loop": kind == 1,
            "loop_seconds": (start + loop_sector.max(0) as i64) as f64 * SECTOR_SECONDS});
        bgm.push(json!({"id": id, "columns": [column], "characters": []}));
    }
    let arenas = (0..BGM_BY_ARENA.1)
        .map(|k| Ok(u16_at(elf, BGM_BY_ARENA.0 + 2 * k, "the arenas' BGM")? as i16))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "dialect": "ttt",
        "bgm_table": "0x50CD00 (28 x 10 bytes: u8 kind, s8, u16, s16, u16 file of TEKKEN.BIN, s16 loop sector), sent to RSPU2DRV.IRX by 0x397A00; play: command 0x40 (0x39A0C0)",
        "kinds": "1 a stream (loops from loop_seconds), 2 a sound bank (id 0: file 2, audio/sfx/002/, its left and right halves), 3 a stream played once",
        "names": "0 SELECT, 1 EMBU, 2 RESULT, 4 ENDING REPLAY, 5 NAME ENTRY, 6 STAFF ROLL, 8 CONTINUE?, 9-22 the arenas' themes (0x455D80); 23-27 jingles",
        "files_from": "audio/music/NN.flac: NN is the stream's file in TEKKEN.BIN; null for one that is not a stream",
        "bgm": bgm,
        "by_arena": {"address": "0x4FB9C0", "note": "the fight's BGM id by arena number (stages/stage_NN.glb); settings mode 8 picks 9 + rand() % 14, or 12 (0x3BFEC0)", "bgm": arenas},
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn music_reads_the_bgm_table_and_the_endings_piece() {
        let mut elf = vec![0u8; 0x220000];
        let put = |elf: &mut Vec<u8>, o: usize, v: &[u16]| {
            for (k, v) in v.iter().enumerate() {
                elf[o + 2 * k..][..2].copy_from_slice(&v.to_le_bytes());
            }
        };
        put(&mut elf, BGM.0 + 10 * 9, &[0xea01, 9830, 0xffa3, 22, 93]); // Paul's: file 22, loop at sector 93
        put(&mut elf, BGM.0 + 10 * 7, &[1, 0, 0, 20, 0xac50]); // a negative loop sector
        put(&mut elf, BGM_BY_ARENA.0, &[9]);
        let m = music(&elf, &[20, 22]).unwrap();
        let paul = &m["bgm"][9]["columns"][0];
        assert_eq!(
            (&paul["file"], &paul["loop"], &paul["raw"]),
            (&json!("22.flac"), &json!(true), &json!([1, -22, 9830, -93, 22, 93]))
        );
        assert!((paul["loop_seconds"].as_f64().unwrap() - 93.0 * 1792.0 / 44100.0).abs() < 1e-9);
        let piece = &m["bgm"][7]["columns"][0];
        assert_eq!(piece["start_seconds"], piece["loop_seconds"]);
        assert!(piece["start_seconds"].as_f64().unwrap() > 134.0);
        // a file not written is none; kind 0 does not loop
        assert_eq!(
            (&m["bgm"][0]["columns"][0]["file"], &m["bgm"][0]["columns"][0]["loop"]),
            (&Value::Null, &json!(false))
        );
        assert_eq!(m["by_arena"]["bgm"][0], 9);
        assert!(music(&elf[..BGM.0 + 100], &[]).is_err());
    }
}

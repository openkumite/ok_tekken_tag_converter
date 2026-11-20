//! The fight data in the executable (docs/FIGHT_DATA.md), written as the pack's `gamedata/*.json` (docs/GAMEDATA.md):
//! moves, cancel lists, hit records, reactions, movesets, hurt and push volumes, health. The shapes are the Tekken 3
//! converter's wherever the game kept the concept; the numbers (aliases, conditions, transitions, req codes, flag
//! bits, property ops) are Tekken Tag's own, as stored. Integers only, but two float constants written with their bits.

use crate::moves::{self, ALIAS_COUNT, ALIASES, BODIES, MOVE_COUNT, MOVES, MOVESETS, VADDR};
use crate::{bytes, u8_at, u16_at, u16le, u32_at};
use anyhow::{Context, Result, bail, ensure};
use ok_converter_common::io::Vfs;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

// (address in memory - VADDR = offset in the file; entries)
const CANCELS: (usize, usize) = (0x482248 - VADDR, 0x82cf); // 14 bytes each, the moves' lists one after another
const SHARED: (usize, usize) = (0x47f780 - VADDR, 782); // the lists a CALL enters, each up to a RETURN
const SEQUENCES: usize = 0x4efd44 - VADDR; // the motion input of command c is at + 2 * c
const PROPS: usize = 0x524690 - VADDR; // timed properties: u16 frame, u16 op << 8 | arg; a list ends at frame 0
pub(crate) const SOUNDS: usize = 0x52c448 - VADDR; // sound lists: u16 type << 12 | index; a list ends at 0xffff
const HITS: usize = 0x5e8978 - VADDR; // hit records of 10 bytes; a chain ends at gate 0xffff
const REVERSALS: usize = 0x5e8608 - VADDR; // s16 motion lists ending in -1, up to the hit records
const REACTIONS: (usize, usize) = (0x46be78 - VADDR, 1431); // 56 bytes each, up to the shared cancels
const CURVES: (usize, usize) = (0x523fa0 - VADDR, 74); // 24 bytes each, up to the timed properties
const LIFE: usize = 0x51f5a0 - VADDR; // 8 x u32 health, 20.12 fixed point; then a percent per body
const HURT: (usize, usize) = (0x42f060 - VADDR, 14); // u32 bone slot per sphere, then a pointer per body to s16 radii
const PUSH: (usize, usize) = (0x42efb8 - VADDR, 8);
/// The joints' places (0x3AFAA0): by body (+0x2A) a pointer to 24 x {s16 x, y, z, pad} in the parent joint's axes;
/// the costume file 0x6B (+0x2E, Angel's) has its own at 0x4396C0.
const JOINTS: usize = 0x439a80 - VADDR;
const ANGEL_JOINTS: (usize, usize) = (0x4396c0 - VADDR, 0x6b);
/// Each of the 18 slots' parent slot (the model's node tree through 0x437E70, as 0x3AFAA0 links them).
const SLOT_PARENTS: [i8; 18] = [-1, 0, 1, 1, 3, 4, 5, 1, 7, 8, 9, 0, 11, 12, 13, 11, 15, 16];
const RECOVERABLE: usize = 0x5f5168 - VADDR; // f32 0.6, f32 1.4: the part of a hit's damage the recoverable cap loses
const ANGER: (usize, usize) = (0x5207f0 - VADDR, 76); // 5 x s8: partner body, its variant mask, active body, its mask, effect
// air reactions of 28 bytes (0x374010): entries 0-7 in the first table, entry k >= 8 at the second + 28 * k
const AIR: (usize, usize) = (0x500678 - VADDR, 0x5f2ba8 - VADDR);
// the camera (FIGHT_DATA.md section 18)
const ARENA_PROJECTION: (usize, usize) = (0x507fe0 - VADDR, 25); // {H, H min, H max, dolly flag, variant}, s16
const SITUATIONS: (usize, usize) = (0x508908 - VADDR, 30); // {in frames, hold, pitch[4], yaw offset[4]}, s16
const CAMERA_LISTS: (usize, usize) = (0x434c78 - VADDR, 0x134); // {u32 list, s16 count, total, split, first mirrored}
/// Disc file 12: the camera motions.
pub const CAMERA_FILE: usize = 12;
const CAMERA_MOTIONS: usize = 0x508130 - VADDR; // u32 offsets of the camera motions in disc file 12, rising
const INTRO_OFFSET: usize = 0x509c20 - VADDR; // s16 eye x, y, z
const TEAM_SHOTS: (usize, usize) = (0x521140 - VADDR, 2); // 18 x s16: two turns, then four points (0x3D2330)
/// The per-fighter situation cameras' lists 0x12C-0x177 (0x5095C0: {u32 entries, u32 count}; entries 0x14 bytes
/// {s16, s16 weight, u32 script x 3}; a script 16 x s16), and by camera id the list numbers (12-byte rows, the list at
/// +8): ids 0x1C-0x20 at 0x509700, 0x23-0x66 at 0x5096E8 (0x3943A8).
const FIGHTER_LISTS: (usize, usize) = (0x5095c0 - VADDR, 0x4c);
/// The per-fighter cameras' rows by character and alias (0x509820, 0x4D x 12 bytes {u32 character (+0x28), s16
/// alias (+0x10C), s16, u32 list}), read when the looked-at fighter is down (+0x131).
const FIGHTER_ROWS: (usize, usize) = (0x509820 - VADDR, 0x4d);
const WORLD_TURN: usize = 0x417780 - VADDR;
const FIGHTER_IDS: [(usize, std::ops::RangeInclusive<usize>); 2] =
    [(0x509700 - VADDR, 0x1c..=0x20), (0x5096e8 - VADDR, 0x23..=0x66)];
/// By character (+0x28, the 33 fighters): {s32 count, u32 list of (s16 alias, s16 frame)} (0x3D29E8).
const START_FRAMES: (usize, usize) = (0x447480 - VADDR, 33);
const TEAM_PAIRS: usize = 0x43a180 - VADDR; // 28 bytes: two {character, costume, alias, x, z, facing}, 2 x s16; -1 ends
const REPLAY_SHOTS: (usize, usize) = (0x520c40 - VADDR, 42); // 10 x s16 (0x3D0748)
const REPLAY_SEQUENCES: (usize, usize) = (0x520f88 - VADDR, 14); // 4 x s8 shot numbers, one a pass
/// The life option every match uses (0x3B1488, the only writer of the option, sets 3).
const LIFE_OPTION: usize = 3;

const END: u16 = 0x8000;
const CALL: u16 = 0x8004;
const RETURN: u16 = 0x8005;
/// The alias table's "no move".
const MISSING: u16 = 7350;
/// No list of the game is longer (the longest, a sound list, is under 40 entries): one that does not end is an error.
const MAX_LIST: usize = 256;

/// A u16 / u8 inside an entry already read.
pub(crate) fn h(e: &[u8], o: usize) -> u16 {
    u16le(e, o).unwrap_or_default()
}
fn b(e: &[u8], o: usize) -> u8 {
    e.get(o).copied().unwrap_or_default()
}
fn w(e: &[u8], o: usize) -> u32 {
    crate::u32le(e, o).unwrap_or_default()
}

/// The `size`-byte entries from entry `i` of the table at `base` up to (not with) the one `last` knows.
pub(crate) fn until<'a>(
    elf: &'a [u8],
    base: usize,
    i: u32,
    size: usize,
    what: &str,
    last: impl Fn(&[u8]) -> bool,
) -> Result<Vec<&'a [u8]>> {
    let at = (i as usize).checked_mul(size).and_then(|o| o.checked_add(base)).with_context(|| format!("{what} {i}"))?;
    let mut out = vec![];
    for k in 0..MAX_LIST {
        let e = bytes(elf, at.saturating_add(size * k), size, what)?;
        if last(e) {
            return Ok(out);
        }
        out.push(e);
    }
    bail!("{what} {i} does not end")
}

const LEVELS: [&str; 9] =
    ["none", "low", "mid", "special_mid", "high", "level_5", "unblockable", "throw", "throw_link"];

/// The hit records of the chain starting at `i`, the ungated one (the chain's last) included.
fn chain(elf: &[u8], i: u32) -> Result<Vec<&[u8]>> {
    let mut c = until(elf, HITS, i, 10, "hit record chain", |e| h(e, 0) == 0xffff)?;
    c.push(bytes(elf, HITS + 10 * (i as usize + c.len()), 10, "hit record")?);
    Ok(c)
}

/// moves.json: the 7351 move records. `anims`: each motion's file in the pack; `frames`: each motion's length.
pub fn moves(elf: &[u8], anims: &[Option<String>], frames: &[u8]) -> Result<Value> {
    let mut out = Vec::with_capacity(MOVE_COUNT);
    for i in 0..MOVE_COUNT {
        let r = bytes(elf, MOVES + 52 * i, 52, "the moves")?;
        ensure!((w(r, 0xc) as usize) < CANCELS.1, "move {i}: its cancel list is outside the table");
        let hit = chain(elf, h(r, 0x14) as u32)?.last().map_or(0, |e| h(e, 2));
        let (sound_list, property_list) =
            (Some(w(r, 0x1c)).filter(|&v| v != !0), Some(w(r, 0x20)).filter(|&v| v != !0));
        let sounds = match sound_list {
            Some(s) => Some(
                until(elf, SOUNDS, s, 2, "sound list", |e| h(e, 0) == 0xffff)?
                    .iter()
                    .map(|e| json!({"raw": h(e, 0), "type": h(e, 0) >> 12, "index": h(e, 0) & 0xfff}))
                    .collect::<Vec<_>>(),
            ),
            None => None,
        };
        let props = match property_list {
            Some(p) => Some(until(elf, PROPS, p, 4, "timed property list", |e| h(e, 0) == 0)?),
            None => None,
        };
        // the boot routine adds the arguments of op 0x0C (damage to the opponent) to the damage
        let bonus: u32 = props.iter().flatten().filter(|e| b(e, 3) == 0x0c).map(|e| b(e, 2) as u32).sum();
        let properties = props
            .map(|p| p.iter().map(|e| json!({"frame": h(e, 0), "op": b(e, 3), "arg": b(e, 2)})).collect::<Vec<_>>());
        let (posture, mask, flags, push) = (w(r, 4), w(r, 8), w(r, 0x24), h(r, 0x30));
        let level = mask >> 8 & 0xff;
        let bones = [b(r, 0x28), b(r, 0x29), b(r, 0x2a), b(r, 0x2b)];
        let active = [b(r, 0x2c), b(r, 0x2d)];
        let attack = (bones != [0; 4] || active[0] != 0).then(|| {
            json!({"bones": {"end0": bones[0], "start0": bones[1], "end1": bones[2], "start1": bones[3]},
                   "segment_count": if bones[2] != 0 { 2 } else { 1 }, "object_driven": bones[0] >= 0x18})
        });
        out.push(json!({
            "index": i, "anim": anims.get(i), "motion": w(r, 0), "frames": frames.get(i).copied().unwrap_or(0),
            "posture": {"raw": posture, "base": posture & 7, "state_code": posture >> 11 & 31, "tag_state": posture >> 24 & 1 != 0},
            "attack_mask": {"raw": mask, "mask": mask & 0xffff, "level": level,
                            "level_name": LEVELS.get(level as usize).map_or_else(|| format!("level_{level}"), |n| n.to_string())},
            "cancel_start": w(r, 0xc),
            "next": {"alias": h(r, 0x10), "hi": h(r, 0x12)},
            "hit": h(r, 0x14),
            "damage": hit & 0x3fff, "damage_runtime": (hit & 0x3fff) as u32 + bonus, "damage_type": hit >> 14,
            "air_drift": h(r, 0x16) as i16,
            "bytes_18": {"frame_count_ram": b(r, 0x18), "jump_start": b(r, 0x19), "jump_end": b(r, 0x1a), "hold_frame": b(r, 0x1b)},
            "active": active, "hitstop": b(r, 0x2e) as i8, "air_reaction": b(r, 0x2f),
            "flags24": {"raw": flags, "camera_script": flags >> 23},
            "push_control": {"raw": push, "mode": push & 15, "disable_mask": push >> 4 & 0xff, "mask_only_near_active": push & 0x1000 != 0},
            "word_32": h(r, 0x32),
            "sound_list": sound_list, "sounds": sounds,
            "property_list": property_list, "properties": properties,
            "attack": attack,
            "raw_hex": r.iter().map(|x| format!("{x:02x}")).collect::<String>(),
        }));
    }
    Ok(json!({"address": "0x531EA0 (7351 x 52 bytes)", "moves": out}))
}

const DIRS: [&str; 10] = ["?", "d/b", "d", "d/f", "b", "n", "f", "u/b", "u", "u/f"];

/// Buttons 1-4 of a nibble and the tag button, as numbers and as text.
fn buttons(nibble: u16, tag: bool, sep: &str) -> (Vec<u16>, String) {
    let n: Vec<u16> = (0..4).filter(|k| nibble >> k & 1 != 0).map(|k| k + 1).collect();
    let mut t: Vec<String> = n.iter().map(u16::to_string).collect();
    if tag {
        t.push("tag".into());
    }
    (n, t.join(sep))
}

/// A one-frame command in the Tekken 3 converter's notation: direction bits 5-13 (none or all: any direction),
/// buttons 1-4 in bits 0-3, bit 4 = any of them (without buttons: the direction held), and bit 14, the tag button.
fn command_text(c: u16) -> String {
    let dirs: Vec<&str> = (1..10).filter(|n| c >> (n + 4) & 1 != 0).filter_map(|n| DIRS.get(n).copied()).collect();
    let d = if dirs.len() == 9 { String::new() } else { dirs.join("|") };
    let any = c & 0x10 != 0;
    let mut t = buttons(c & 15, c & 0x4000 != 0, if any { "|" } else { "+" }).1;
    if any && t.contains('|') {
        t = format!("({t})");
    }
    match (d.is_empty(), t.is_empty()) {
        (true, true) if any => "*".into(),
        (false, true) if any => d,
        (true, true) => "* (no button)".into(),
        (false, true) => format!("{d} (no button)"),
        (true, false) => t,
        (false, false) => format!("{d}+{t}"),
    }
}

/// The motion input of command `c` (>= 0x8006): its frame window, and its steps (buttons << 8 | numpad direction).
fn sequence(elf: &[u8], c: u16) -> Result<(i16, Vec<u16>)> {
    let at = SEQUENCES + 2 * c as usize;
    let steps = until(elf, at + 2, 0, 2, "input sequence", |e| h(e, 0) == 0)?;
    Ok((u16_at(elf, at, "input sequence")? as i16, steps.iter().map(|e| h(e, 0)).collect()))
}

fn step_text(v: u16) -> String {
    let d = DIRS.get((v & 15) as usize).copied().unwrap_or("?");
    let t = buttons(v >> 8 & 15, v & 0x4000 != 0, "+").1;
    if t.is_empty() { d.into() } else { format!("{d}+{t}") }
}

/// Entry `index` of a cancel table: u16 cmd, u8 req, u8 cond, u16 param, u16 dest alias, u16 trans, u8 prefilter,
/// u8 window start, u8 window end, u8 exec.
fn cancel(index: usize, e: &[u8], sequences: &BTreeMap<u16, (i16, Vec<u16>)>) -> Value {
    let (cmd, trans) = (h(e, 0), h(e, 8));
    let (kind, input) = match cmd {
        END => ("end", "END".to_string()),
        0x8001 => ("dash", "dash".into()),
        0x8002 => ("backdash", "backdash".into()),
        0x8003 => ("special_8003", String::new()),
        CALL => ("call", "CALL".into()),
        RETURN => ("return", "RETURN".into()),
        c if c < END => ("command", command_text(c)),
        c => (
            "sequence",
            sequences
                .get(&c)
                .map_or(String::new(), |s| s.1.iter().map(|&v| step_text(v)).collect::<Vec<_>>().join(",")),
        ),
    };
    let mut o = json!({"index": index, "cmd": cmd, "kind": kind, "input": input, "req": b(e, 2),
        "cond": b(e, 3) & 0x7f, "cond_tag": b(e, 3) & 0x80 != 0, "param": h(e, 4), "dest": h(e, 6),
        "transition": trans, "transition_type": trans & 0x3fff, "turn_180": trans & 0x8000 != 0,
        "transition_bit14": trans & 0x4000 != 0, "prefilter": b(e, 10),
        "win_start": b(e, 11), "win_end": b(e, 12), "exec": b(e, 13)});
    if let Some(o) = o.as_object_mut().filter(|_| cmd == CALL) {
        o.insert("shared_list_start".into(), json!(h(e, 6)));
    }
    o
}

/// cancels.json: the moves' cancel lists, the shared lists, the input sequences, the reversal motion lists.
pub fn cancels(elf: &[u8]) -> Result<Value> {
    let table = |(base, n): (usize, usize)| -> Result<Vec<&[u8]>> {
        (0..n).map(|i| bytes(elf, base + 14 * i, 14, "the cancel lists")).collect()
    };
    let (own, shared) = (table(CANCELS)?, table(SHARED)?);
    ensure!(own.last().is_some_and(|e| h(e, 0) == END), "the cancel lists do not end in END");
    let mut sequences = BTreeMap::new();
    let mut starts = BTreeSet::new();
    for e in own.iter().chain(&shared) {
        match h(e, 0) {
            CALL => {
                ensure!((h(e, 6) as usize) < SHARED.1, "a cancel calls shared list {}: outside the table", h(e, 6));
                starts.insert(h(e, 6));
            }
            c if c > RETURN && !sequences.contains_key(&c) => {
                sequences.insert(c, sequence(elf, c)?);
            }
            _ => {}
        }
    }
    let entries = |t: &[&[u8]]| t.iter().enumerate().map(|(i, e)| cancel(i, e, &sequences)).collect::<Vec<_>>();
    let seqs: Vec<Value> = (sequences.iter())
        .map(|(&cmd, (window, steps))| {
            let text = steps.iter().map(|&v| step_text(v)).collect::<Vec<_>>().join(",");
            let steps: Vec<Value> = (steps.iter())
                .map(|&v| json!({"raw": v, "dir": v & 15, "buttons": buttons(v >> 8 & 15, false, "").0, "tag": v & 0x4000 != 0}))
                .collect();
            json!({"cmd": cmd, "address": format!("{:#X}", SEQUENCES + VADDR + 2 * cmd as usize), "window": window, "input": text, "steps": steps})
        })
        .collect();
    // the reversal conditions' motion lists, by their place in the table (in s16 units): how a condition's
    // parameter picks one is not decoded
    let (mut families, mut offsets, mut list, mut start) = (vec![], vec![], vec![], 0);
    for k in 0..(HITS - REVERSALS) / 2 {
        match u16_at(elf, REVERSALS + 2 * k, "the reversal motion lists")? as i16 {
            -1 => {
                families.push(std::mem::take(&mut list));
                offsets.push(start);
                start = k + 1;
            }
            v => list.push(v),
        }
    }
    Ok(json!({
        "cancels": {"address": "0x482248 (14-byte entries)", "count": own.len(), "entries": entries(&own)},
        "shared_cancels": {"address": "0x47F780", "count": shared.len(), "list_starts": starts, "entries": entries(&shared)},
        "sequences": {"address": "0x4EFD44 + 2 * cmd: s16 window, u16 steps, 0", "entries": seqs},
        "reversal_families": {"address": "0x5E8608", "families": families, "offsets": offsets},
    }))
}

/// hits.json: every hit record a move's chain reaches.
pub fn hits(elf: &[u8]) -> Result<Value> {
    let mut count = 0;
    for i in 0..MOVE_COUNT {
        let first = u16_at(elf, MOVES + 52 * i + 0x14, "the moves")?;
        count = count.max(first as usize + chain(elf, first as u32)?.len());
    }
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let e = bytes(elf, HITS + 10 * i, 10, "hit record")?;
        let gate = Some(h(e, 0)).filter(|&g| g != 0xffff);
        records.push(json!({"index": i, "gate": gate, "damage": h(e, 2) & 0x3fff, "damage_type": h(e, 2) >> 14,
            "reaction": h(e, 4), "close_reaction": h(e, 6), "close_distance": h(e, 8)}));
    }
    Ok(json!({"address": "0x5E8978 (10-byte records)", "count": count, "reaction_none": 0x592, "records": records}))
}

/// reactions.json: the reactions and the pushback curves they point at.
pub fn reactions(elf: &[u8]) -> Result<Value> {
    let mut entries = Vec::with_capacity(REACTIONS.1);
    for i in 0..REACTIONS.1 {
        let e = bytes(elf, REACTIONS.0 + 56 * i, 56, "the reactions")?;
        let mut params = vec![];
        for k in 0..7 {
            let v = w(e, 4 * k);
            // 1, 3, 5, 6 point at a curve (0: none); the others are angles
            params.push(if k % 2 == 0 && k != 6 {
                json!(v)
            } else if v == 0 {
                Value::Null
            } else {
                let o = (v as usize).wrapping_sub(CURVES.0 + VADDR);
                ensure!(o.is_multiple_of(24) && o / 24 < CURVES.1, "reaction {i}: {v:#x} is not a pushback curve");
                json!(o / 24)
            });
        }
        let victims: Vec<u16> = (0..14).map(|k| h(e, 28 + 2 * k)).collect();
        entries.push(json!({"index": i, "victims": victims, "params": params}));
    }
    let mut curves = Vec::with_capacity(CURVES.1);
    for i in 0..CURVES.1 {
        let e = bytes(elf, CURVES.0 + 24 * i, 24, "the pushback curves")?;
        let impulses: Vec<i16> = (2..12).map(|k| h(e, 2 * k) as i16).collect();
        curves.push(
            json!({"index": i, "constant_frames": h(e, 0) as i16, "velocity": h(e, 2) as i16, "impulses": impulses}),
        );
    }
    // the moves' own air reactions (move +0x2F) are the entries from 8 up
    let mut air_count = 8;
    for i in 0..MOVE_COUNT {
        air_count = air_count.max(b(bytes(elf, MOVES + 52 * i, 52, "the moves")?, 0x2f) as usize + 1);
    }
    let mut air = Vec::with_capacity(air_count);
    for i in 0..air_count {
        let e = bytes(elf, if i < 8 { AIR.0 } else { AIR.1 } + 28 * i, 28, "the air reactions")?;
        let s = |k: usize| h(e, 2 * k) as i16;
        air.push(
            json!({"index": i, "alias": h(e, 0), "target_height": s(1), "rise": s(2), "words": [s(3), s(4), s(5)],
            "pushback": (6..14).map(s).collect::<Vec<_>>()}),
        );
    }
    Ok(json!({
        "reaction_table": "0x46BE78 (1431 x 56 bytes: u32 params[7], u16 victims[14])",
        "slot_names": ["front_standing", "front_crouching", "counter_standing", "counter_crouching", "side_a_standing",
            "side_a_crouching", "side_b_standing", "side_b_crouching", "back_standing", "back_crouching",
            "block_standing", "block_crouching", "grounded_faceup", "grounded_facedown"],
        "param_names": ["pushback_angle", "pushback_curve_hit", "angle_2", "pushback_curve_counter",
            "pushback_angle_grounded", "pushback_curve_grounded", "pushback_curve_block"],
        "none": 0x592,
        "entries": entries,
        "pushback_curves": {"address": "0x523FA0 (74 x 24 bytes)", "count": CURVES.1, "entries": curves},
        "air_reactions": {"address": "0x500678 (0-7), 0x5F2BA8 + 28 x index (8 on): 28 bytes", "entries": air},
    }))
}

fn moveset_file(m: usize) -> String {
    format!("moveset_{m:02}.json")
}

/// movesets/moveset_NN.json of each moveset, then movesets/index.json, into `out`. `models`: the manifest's fighters.
fn movesets(elf: &[u8], models: &[Value], out: &mut Vec<(String, Value)>) -> Result<()> {
    let roster = moves::fighters(elf)?;
    let who = |f: &(String, usize, usize)| json!({"name": f.0, "body": f.1, "moveset_id": f.2, "moveset_file": moveset_file(f.2)});
    for m in 0..MOVESETS {
        let mut aliases = Vec::with_capacity(ALIAS_COUNT);
        for a in 0..ALIAS_COUNT {
            let v = u16_at(elf, ALIASES + 2 * (m * ALIAS_COUNT + a), "the movesets")?;
            ensure!((v as usize) < MOVE_COUNT, "moveset {m}, alias {a}: no move {v}");
            aliases.push(Some(v).filter(|&v| v != MISSING));
        }
        let fighters: Vec<Value> = roster.iter().filter(|f| f.2 == m).map(who).collect();
        let count = aliases.iter().flatten().collect::<BTreeSet<_>>().len();
        out.push((
            format!("gamedata/movesets/{}", moveset_file(m)),
            json!({"moveset_id": m, "fighters": fighters, "move_count": count, "aliases": aliases}),
        ));
    }
    let models: Vec<Value> = (models.iter())
        .filter_map(|m| {
            let set = m.get("moveset")?.as_u64()? as usize;
            Some(json!({"model": m.get("id")?, "model_record": m.get("id")?, "glb": m.get("file")?, "body": m.get("body")?, "moveset_id": set, "moveset_file": moveset_file(set), "hands": m.get("hands")}))
        })
        .collect();
    let index = json!({
        "models": models,
        "fighters": roster.iter().filter(|f| f.2 < MOVESETS).map(who).collect::<Vec<_>>(),
        "files": (0..MOVESETS).map(moveset_file).collect::<Vec<_>>(),
        "aliases": ALIAS_COUNT,
    });
    out.push(("gamedata/movesets/index.json".into(), index));
    Ok(())
}

/// Costume slot `s`'s hands (FIGHT_DATA.md 3.1): `file` its poses, `default` the kind the load eases both hands to
/// and op kind 9 means (0x416BA0), `ops` whether the hand ops move them (True Ogre's move his wings, fighter
/// +0x3130 bit 0: 0x3ADAA0).
pub fn hands(elf: &[u8], s: usize, file: &str) -> Result<Value> {
    let at = |a: usize| u8_at(elf, a - VADDR, "the hand table");
    Ok(json!({"file": file, "default": at(0x416ba0 + s)?, "ops": !(82..=85).contains(&s)}))
}

/// The spheres of the table at `base`: their bone slots, and each body's radii.
fn spheres((base, n): (usize, usize), elf: &[u8], lift: &[(usize, i32)], names: &[(String, usize)]) -> Result<Value> {
    let what = "the hurt and push spheres";
    let mut layout = Vec::with_capacity(n);
    for i in 0..n {
        let y = lift.iter().find(|l| l.0 == i).map_or(0, |l| l.1);
        let slot = u32_at(elf, base + 4 * i, what)?;
        // (OpenKumite finds the joint by its name, the offset in metres with y up)
        let joint = crate::model::RIG.get(slot as usize).map(|r| r.0);
        layout.push(json!({"index": i, "bone_slot": slot, "offset_game": [0, y, 0], "gltf_joint": joint,
                           "offset_gltf_m": [0.0, -y as f64 * 0.001, 0.0]}));
    }
    let mut bodies = Vec::with_capacity(BODIES);
    for (body, name) in names.iter().enumerate() {
        let p = (u32_at(elf, base + 4 * (n + body), what)? as usize).wrapping_sub(VADDR);
        let radii =
            (0..n).map(|k| Ok(u16_at(elf, p.saturating_add(2 * k), what)? as i16)).collect::<Result<Vec<_>>>()?;
        bodies.push(json!({"character": body, "name": name.0, "radii": radii}));
    }
    Ok(
        json!({"table": format!("{:#X} ({n} x u32 bone slot, then {BODIES} x pointer -> {n} x s16 radius)", base + VADDR),
        "layout": layout, "per_character": bodies}),
    )
}

/// The 24 joint places (x, y, z) of the table at file offset `at`.
fn joint_places(elf: &[u8], at: usize) -> Result<Vec<[i16; 3]>> {
    (0..24)
        .map(|k| {
            let w = |c: usize| u16_at(elf, at.saturating_add(8 * k + 2 * c), "the joints' places").map(|v| v as i16);
            Ok([w(0)?, w(1)?, w(2)?])
        })
        .collect()
}

/// rules.json `joints`: the slots' parents and each body's joint places, which 0x3AFAA0 builds the matrix slots with
/// (the strikes, the hurt and push spheres and the drawing all take those slots).
fn joints(elf: &[u8], names: &[(String, usize)]) -> Result<Value> {
    let mut bodies = Vec::with_capacity(BODIES);
    for (body, name) in names.iter().enumerate() {
        let p = (u32_at(elf, JOINTS + 4 * body, "the joints' places")? as usize).wrapping_sub(VADDR);
        bodies.push(json!({"character": body, "name": name.0, "places": joint_places(elf, p)?}));
    }
    let angel = json!({"file_slot": ANGEL_JOINTS.1, "model_record": 345 + 3 * ANGEL_JOINTS.1,
        "places": joint_places(elf, ANGEL_JOINTS.0)?});
    Ok(json!({"table": "0x439A80 (34 x pointer -> 24 x {s16 x, y, z, pad})", "parents": SLOT_PARENTS,
        "per_character": bodies, "per_model": [angel]}))
}

/// rules.json: health, and the hurt and push spheres.
pub fn rules(elf: &[u8]) -> Result<Value> {
    let names = moves::roster(elf)?;
    let life = (0..8).map(|k| u32_at(elf, LIFE + 4 * k, "the life options")).collect::<Result<Vec<_>>>()?;
    let percent =
        (0..BODIES).map(|k| u32_at(elf, LIFE + 32 + 4 * k, "the health percents")).collect::<Result<Vec<_>>>()?;
    let factor = |k: usize| -> Result<Value> {
        let bits = u32_at(elf, RECOVERABLE + 4 * k, "the recoverable health factors")?;
        Ok(json!({"bits": bits, "value": f32::from_bits(bits)}))
    };
    let relations = (0..ANGER.1)
        .map(|k| {
            let b = |i: usize| u8_at(elf, ANGER.0 + 5 * k + i, "the anger relations");
            Ok(json!({"partner_body": b(0)?, "partner_mask": b(1)?, "active_body": b(2)?, "active_mask": b(3)?,
                      "effect": b(4)? as i8}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        // which game's numbers the fight data holds (docs/GAMEDATA.md "Numbering")
        "dialect": "ttt",
        "health": {
            "representation": "20.12 fixed point at player +0x3F4; a body's health = life option x its percent / 100",
            "life_table": "0x51F5A0", "life_options": life.iter().map(|v| v >> 12).collect::<Vec<_>>(), "life_options_raw": life,
            "default_life_option": LIFE_OPTION, "default_max": life.get(LIFE_OPTION).map_or(0, |v| v >> 12),
            "percent_table": "0x51F5C0", "percent_per_character": percent,
            "recoverable": {"address": "0x5F5168", "damage_factor": factor(0)?, "damage_factor_flag_12a": factor(1)?},
        },
        // sphere 8 (the head) is lifted 120, sphere 11 (the root) 60: y is down
        "hurt_spheres": spheres(HURT, elf, &[(8, -120), (11, -60)], &names)?,
        "push_spheres": spheres(PUSH, elf, &[], &names)?,
        "joints": joints(elf, &names)?,
        // constants of the damage code (0x3720C8, docs/FIGHT_DATA.md section 9)
        "counter_hit": {"multiplier": [12, 10], "bonus_tenths": [3],
            "bonus": "3 * |victim move damage| / 10 * f / startup; 3 * damage / 10 when a ki timer caused it"},
        "chip_damage": {"divisor": 10, "add": 0, "enabled_when": "the match flag byte 0x43B804, or the attacker's ki timer"},
        "juggle": {"first_air_hit_percent": 80, "later_air_hits_percent": 50, "counter": "player +0x100"},
        // the tag system (0x3C1900, 0x3C1B38, 0x3CF278; docs/FIGHT_DATA.md section 10)
        "tag": {"regen_divisor": 6000, "anger": {"start": 100, "angry": 120, "hit": 4, "hit_no_damage": 6,
            "damage_tenths": 13, "any_body": 0x22, "relations_table": "0x5207F0", "relations": relations}},
    }))
}

/// Tekken 3's profile fields from its +4 (ok_tekken3_converter's names; "" its unnamed ones), in order: Tekken Tag's
/// profile has them 4 bytes later.
#[rustfmt::skip]
const T3_PROFILE: [&str; 53] = [
    "attack_cooldown_base", "attack_cooldown_0", "attack_cooldown_1", "attack_cooldown_2", "attack_cooldown_3",
    "max_string_length", "throw_attempt_prob", "throw_escape_prob", "string_continue_prob", "throw_select_prob",
    "guard_aware_attack_prob", "fast_poke_prob", "guard_prob", "guard_prob_cap", "counter_attack_prob",
    "counter_attack_cap", "sidestep_prob", "in_string_interrupt_prob", "in_string_interrupt_cap",
    "in_string_guard_prob", "in_string_guard_cap", "followup_count_a", "followup_prob_a",
    "consecutive_attack_limit_a", "followup_prob_b", "consecutive_attack_limit_b", "followup_prob_c",
    "evade_and_handler_prob", "pressure_on_stun_prob", "", "", "", "", "gate_always_zero", "anti_air_prob", "",
    "keep_walk_back_prob", "", "", "", "", "", "", "parry_prob", "", "counter_throw_startup_prob", "",
    "stun_recovery_prob", "", "", "", "", "",
];
/// Tekken Tag's own readers past Tekken 3's (FIGHT_DATA.md section 21.8), by their offset.
#[rustfmt::skip]
const TTT_PROFILE: [(usize, &str); 7] = [
    (0x68, "stun_attack_prob"), (0x6e, "tag_out_prob"), (0x70, "stun_tag_out_prob"), (0x72, "guard_after_reaction_prob"),
    (0x74, "delay_input_prob"), (0x76, "repeat_count"), (0x78, "repeat_guard_prob"),
];
const PROFILES: usize = 0x431918 - VADDR; // 5 difficulties x 0x400, 8 levels x 0x80
const RANGES: usize = 0x432e10 - VADDR; // 6 x u16 per body: four reach boundaries, unused, the body offset
const ZONES: (usize, usize) = (0x4318d8 - VADDR, 0x4318f0 - VADDR); // 6 x u32 zone masks: mine, his
const HANDLERS: (usize, usize) = (0x507a80 - VADDR, 34);
/// The per-moveset table (0x507620): its file offset and rows, one per move set.
const PER_MOVESET: (usize, usize) = (0x507620 - VADDR, 32); // {moveset, vs-this-moveset handler, own handler}
const DIFFICULTIES: [&str; 5] = ["EASY", "NORMAL", "HARD", "VERY HARD", "ULTRA HARD"];

/// ai.json: the CPU AI's tables (docs/FIGHT_DATA.md section 19) in Tekken 3's shape (ok_tekken3_converter's ai.json):
/// the profiles, the reach per body, the zone masks, the per-moveset handlers (their code is not read), the canned
/// inputs, the dash steps, the evade plans, the wake-up categories and the random generators' differences.
pub fn ai(elf: &[u8]) -> Result<Value> {
    let what = "the AI tables";
    let s16 = |o: usize| -> Result<i64> { Ok(i64::from(u16_at(elf, o, what)? as i16)) };
    let mut profiles = vec![];
    for (diff, name) in DIFFICULTIES.iter().enumerate() {
        for level in 0..8 {
            let at = PROFILES + 0x400 * diff + 0x80 * level;
            let mut f = serde_json::Map::new();
            f.insert("wake_up_intervals".into(), bytes(elf, at, 4, what)?.to_vec().into());
            // Tekken Tag's own readers by their name (section 21.8), the others without a reader by their offset
            for k in 0..62usize {
                let o = 4 + 2 * k;
                let t3 = k.checked_sub(2).and_then(|i| T3_PROFILE.get(i)).copied().unwrap_or("");
                let own = TTT_PROFILE.iter().find(|f| f.0 == o).map_or("", |f| f.1);
                let key = if !own.is_empty() {
                    own.into()
                } else if t3.is_empty() {
                    format!("off_{o:02X}")
                } else {
                    t3.into()
                };
                f.insert(key, s16(at + o)?.into());
            }
            profiles.push(json!({"difficulty": diff, "difficulty_name": name, "level": level,
                "address": format!("{:#X}", at + VADDR), "fields": f}));
        }
    }
    let names = moves::roster(elf)?;
    let spacing = (0..BODIES)
        .map(|b| {
            let v = (0..6).map(|k| u16_at(elf, RANGES + 12 * b + 2 * k, what)).collect::<Result<Vec<_>>>()?;
            Ok(json!({"character": b, "name": names.get(b).map_or("", |n| n.0.as_str()), "values": v}))
        })
        .collect::<Result<Vec<_>>>()?;
    let zone = |o: usize| (0..6).map(|k| u32_at(elf, o + 4 * k, what)).collect::<Result<Vec<_>>>();
    let handler = |a: u32| (a != 0).then(|| format!("{a:#X}"));
    let handlers = (0..HANDLERS.1)
        .map(|i| {
            let e = bytes(elf, HANDLERS.0 + 12 * i, 12, what)?;
            Ok(json!({"moveset": w(e, 0), "vs_handler": handler(w(e, 4)), "self_handler": handler(w(e, 8))}))
        })
        .collect::<Result<Vec<_>>>()?;
    // (the first word is played whatever it is, 0x3811E8; the rest to a 0)
    let words = |a: usize| -> Result<Vec<u16>> {
        let rest = until(elf, a - VADDR + 2, 0, 2, what, |e| h(e, 0) == 0)?;
        Ok(std::iter::once(u16_at(elf, a - VADDR, what)?).chain(rest.iter().map(|e| h(e, 0))).collect())
    };
    // the canned inputs: step words (buttons << 8 | numpad direction); King's throws (14 bytes each: standing
    // 0x507A52, crouching 0x507A32, down 0x507A12) and Ganryu's poke (section 21.6)
    let canned = [
        ("sidestep_0", 0x507452),
        ("sidestep_1", 0x50746a),
        ("backdash", 0x5074a2),
        ("okizeme", 0x507482),
        ("throws_stand_0", 0x507a52),
        ("throws_stand_1", 0x507a60),
        ("throws_stand_2", 0x507a6e),
        ("throws_crouch_0", 0x507a32),
        ("throws_crouch_1", 0x507a40),
        ("throws_down_0", 0x507a12),
        ("throws_down_1", 0x507a20),
        ("poke", 0x507618),
    ]
    .iter()
    .map(|&(id, a)| Ok(json!({"id": id, "address": format!("{a:#X}"), "words": words(a)?})))
    .collect::<Result<Vec<_>>>()?;
    // commands 0x8001 / 0x8002 (0x36CF30): the dash and the backdash, a window word then the steps
    let dashes = [(0x8001, 0x4fffd4), (0x8002, 0x4fffe0)]
        .iter()
        .map(|&(cmd, a)| Ok(json!({"cmd": cmd, "address": format!("{a:#X}"), "steps": words(a + 2)?})))
        .collect::<Result<Vec<_>>>()?;
    // per move set (0x507620, 16 bytes: can parry, the high / mid reversal and the low parry inputs, the combo
    // preludes {prob, alias, input} to a negative prob; the neutral offence draws rand() for each prelude, 0x386D60)
    let per_moveset = (0..PER_MOVESET.1)
        .map(|m| {
            let at = PER_MOVESET.0 + 16 * m;
            let seq = |p: u32| -> Result<Value> { Ok(if p == 0 { Value::Null } else { json!({"words": words(p as usize)?}) }) };
            let mut preludes = vec![];
            let mut k = u32_at(elf, at + 12, what)? as usize;
            while k != 0 && preludes.len() < 16 && s16(k - VADDR)? >= 0 {
                let words = words(u32_at(elf, k - VADDR + 4, what)? as usize)?;
                preludes.push(json!({"prob": s16(k - VADDR)?, "alias": u16_at(elf, k - VADDR + 2, what)?, "sequence": {"words": words}}));
                k += 8;
            }
            Ok(json!({"moveset": m, "can_parry": s16(at)?, "high_mid_reversal": seq(u32_at(elf, at + 4, what)?)?,
                "low_parry": seq(u32_at(elf, at + 8, what)?)?, "combo_preludes": preludes}))
        })
        .collect::<Result<Vec<_>>>()?;
    let evade = (0..7).map(|k| u32_at(elf, 0x507990 - VADDR + 4 * k, what)).collect::<Result<Vec<_>>>()?;
    let wake = (0..6).map(|k| u32_at(elf, 0x5078e0 - VADDR + 8 * k, what)).collect::<Result<Vec<_>>>()?;
    let pads = (0..4).map(|k| u16_at(elf, 0x507a08 - VADDR + 2 * k, what)).collect::<Result<Vec<_>>>()?;
    let sine = (0..4096).map(|k| s16(0x4fce88 - VADDR + 2 * k)).collect::<Result<Vec<_>>>()?;
    // the streak mode's profile ladder (0x380120): its rows, as many as gp-0x6FC0 counts
    let rows = u32_at(elf, 0x5fd0f0 - 0x6fc0 - VADDR, what)? as usize;
    let survival = (0..rows.min(64))
        .map(|k| {
            let v = |j: usize| u16_at(elf, 0x432d18 - VADDR + 8 * k + 2 * j, what);
            Ok(json!({"max_wins": v(0)?, "difficulty": v(1)?, "levels": [v(2)?, v(3)?]}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "dialect": "ttt",
        "profiles": {"address": "0x431918", "layout": "difficulty*0x400 + level*0x80", "probability_unit": 4096,
            "note": "Tekken 3's fields 4 bytes later; off_XX by Tekken Tag's offset", "entries": profiles},
        "spacing": {"address": "0x432E10", "format": "6 x u16 per body: [0..3] reach boundaries, [4] unused, [5] body offset",
            "entries": spacing},
        "zone_masks": {"my_zone": zone(ZONES.0)?, "his_zone": zone(ZONES.1)?},
        "per_moveset": {"address": "0x507620", "entries": per_moveset},
        "handlers": {"address": "0x507A80", "entries": handlers, "routines": routines()},
        // Ogre's (0x507A08; Devil's 0x507A00 is the same), drawn by rand() & 3
        "boss_punish_pads": {"address": "0x507A08", "pad_words": pads},
        // Tekken 3's keys, none of their move sets or aliases found in Tekken Tag's code (moveset -1, alias 0xFFFF: none)
        "rules": {"fire": {"moveset": -1, "descriptor": 0}, "low_fire": {"moveset": -1, "descriptor": 0},
            "stance": {"moveset": -1, "alias": 0xffff}, "chain_throws": {"entries": []}, "escaped_throws": {"entries": []},
            "wake_forward": {"movesets": []}, "turn_by_facing": {"movesets": []}, "parries": {"aliases": []},
            "late_parry": {"moveset": -1, "until": [0, 0], "alias": 0xffff}, "counter_alias": {"alias": 0xffff},
            "unthrown": {"moveset": -1}, "evade_closer": {"moveset": -1, "distance": 0},
            "followup_rules": followup_rules(),
            // the wake-up's wait (0x3890D0): his moveset 14's 0xBA9 within 2000
            "wake_wait": {"moveset": 14, "alias": 0xba9, "distance": 2000}},
        // bookkeeping 0x381D30: the distance's swing, sine table 0x4FCE88 (4096 x s16, 4096 = 1) x 192 / 4096
        "distance_jitter": {"amplitude": 192, "sine": sine},
        "canned_sequences": {"entries": canned},
        "motion_records": {"routine": "0x36CF30", "entries": dashes},
        "evade_choices": evade,
        "wakeup_categories": wake,
        // SURVIVAL (settings mode 4): the first row whose max_wins reaches the streak; its difficulty on an odd
        // randint(255), else NORMAL (EASY when the row's is EASY); a level in `levels`, both ends in
        "survival": {"address": "0x432D18", "rows": survival},
        // the AI generator's start (gp-0x64E8), and libc rand() 0x40FBD0 returning bits 0-14 (Tekken 3's: 16-30)
        "rng": {"ai_lcg_seed": u32_at(elf, 0x5fd0f0 - 0x64e8 - VADDR, what)?, "rand_shift": 0},
    }))
}

/// The live per-moveset handlers (section 21.6) as the kinds of Tekken 3's routines they match, with the aliases
/// their code compares (Tekken Tag's).
fn routines() -> Value {
    json!({
        "0x38B698": {"kind": "back_off", "aliases": {"attack": [0x39d]}, "note": "Paul: his 0x39D (0x39C returns at once, a branch bug)"},
        "0x38BC00": {"kind": "canned_throws", "aliases": {"reach": [0x3f]}, "note": "King: canned throws_*, P1A"},
        "0x38BE78": {"kind": "stance_dash", "aliases": {"stance": [0xa8b]}, "note": "Hwoarang: forward out of his stance"},
        "0x38B820": {"kind": "fire_dodge", "aliases": {"breath": [0xba9, 0x1a6]}, "note": "Ogre / True Ogre"},
        "0x38AFC0": {"kind": "fire_dodge", "aliases": {"breath": [0x1459, 0x1458]}, "note": "Devil / Angel: Ogre's"},
        "0x38B718": {"kind": "poke_or_crouch", "aliases": {"attack": [0x1407]},
            "note": "Ganryu: the canned poke at his frames 24-25 within 3000, else a crouch guard within 4000"},
        "0x38B368": {"kind": "special_moves", "aliases": {"rush": [0x1a8], "stance": [0x5c7], "spin": [0x5ca, 0x5cb, 0x5cc],
            "ignore": [0x5ee], "sweep": [0x1a1, 0x5a9], "flight": [0x5c9, 0x623, 0x624, 0x625]},
            "note": "against Yoshimitsu: Tekken 3's 0x80061B88 with Tekken Tag's aliases"},
    })
}

/// The per-alias rule of the follow-up inputs and the chain throws (0x3876A4, 0x38A98C, section 21.2): for my move
/// sets named (all when absent) never, only with rand() & 0x70, only without it, or not on EASY.
fn followup_rules() -> Value {
    json!([
        {"aliases": [0x4ca], "allow": "never"},
        {"aliases": [0x197, 0x181], "allow": "not_easy"},
        {"aliases": [0x3bc], "movesets": [0], "allow": "no_coin"},
        {"aliases": [0x1245, 0x1253], "movesets": [0, 13], "allow": "no_coin"},
        {"aliases": [0x1246, 0x1255], "movesets": [0, 13], "allow": "never"},
        {"aliases": [0x1262], "movesets": [4], "allow": "coin"},
    ])
}

/// rounds.json: the round machine (0x36E198, docs/FIGHT_DATA.md sections 11 and 13), constants of the code written as
/// data. Frames count from 1; aliases are Tekken Tag's.
pub fn rounds() -> Value {
    json!({
        "dialect": "ttt",
        "machine": {"routine": "0x36E198", "state": "gp-0x7268", "states": ["call", "fight", "ko_or_time_up", "result", "pose"]},
        "call": {"note": "state 0: inputs off; the fight (state 1) starts on fight_starts, whether the start motion ended or not",
            "fight_starts": {"first_round": 151, "other_rounds": 91},
            "banner": {"first_round": [66, 104], "other_rounds": [6, 44]}, "ready_from": {"first_round": 106, "other_rounds": 46}},
        "start_motions": {"routine": "0x368398", "first_round": [4, 6], "other_rounds": 5, "unknown_body": 33, "unknown_alias": 9,
            "first_round_pick": "pad bits 0x90 held: alias 4; 0x60: alias 6; else rand() & 1 picks 4 or 6 (the pad: gp-0x74B0 side 0, gp-0x74AC side 1)",
            "first_round_start_frame": "a motion of 151 frames or more starts at frame frames - 149 (it ends with the call), a shorter one at 1",
            "other_rounds_start_frame": 1, "unknown_when": "body 33 in the mode 0x3B6220 tests"},
        "end": {"note": "state 2, counting from the end of the fight (gp-0x71C8); inputs go off except a K.O.'d fight's winner's",
            // (the replay from the ring's last 20 frames (0x364698), at full speed, a new shot each pass; the counter held
            // at 16 / 17 while it runs: 64 frames, three passes of 21-22, seen in PCSX2; then the last shot stays on the
            // live fight to the end of the state, the round's text shown again)
            "ko": {"replay_from": 16, "replay_frames": 20, "replay_passes": 3, "match_over_at": 50, "replay_ends": 110, "done": 135,
                "done_without_replay": 120},
            "time_up": {"match_over_at": 50, "done": 160, "winner_motion": "player +0x420 = 4, +0x430 = 0x1000"}, "other_done": 600},
        "win_poses": {"routine": "0x368528", "aliases": [4770, 4771, 4772, 4773],
            "pick": {"pad_bits": [[0x80, 0], [0x10, 1], [0x40, 2], [0x20, 3]], "else": "the frame counter gp-0x74D8 & 1"},
            "unknown_body": 33, "unknown_pick": "4771 + gp-0x74D8 % 3",
            "partner": "the loser's player becomes the winner's partner, at x 0x400, z -0xC00 from the winner, and poses the same alias; solo: the loser is hidden",
            "frames": 300, "start_skips_from": 2},
        "team_lose": {"routine": "0x36E6C8", "alias": 4780, "frames": 60, "when": "arcade (settings +0x1F), the match over and the human side lost: the loser's partner joins it"},
        "result_flags": {"routine": "0x36F570", "address": "gp-0x7444", "time_up": 0x01, "ko": 0x02, "double_ko": 0x04, "perfect": 0x08, "great": 0x10, "draw": 0x20,
            "perfect_when": "the winning team's two members at their caps (+0x2C == +0x10) and gp-0x6A80[side] == 0",
            "great_score_at_most": 0xcc, "score_unit": "1/4096 of full health (+0x408, 0x36F408)",
            "draw": "both sides win the round; the round limit drops by 1"},
        "round_time": {"routine": "0x36D640", "options_frames": [2160, 2820, 4260, 5640, 6960], "options_seconds": [30, 40, 60, 80, 99],
            "untimed_option": 5, "display": "(frames * 6 / 7 - 1) / 60: a displayed second is 70 frames", "default_option": null},
        "rounds_to_win": {"formula": "min(5, rounds option + 1)", "round_limit": "2 * wins - 1", "default_option": null},
    })
}

/// A camera list's value: (1, situation type) below 30, (2, camera motion) for 30-561, else (0, -1) (0x393A90).
fn camera_value(v: i16) -> (u8, i16) {
    match v {
        0..30 => (1, v + 1),
        30..562 => (2, v - 29),
        _ => (0, -1),
    }
}

/// `N` s16 at `o`.
fn s16s<const N: usize>(elf: &[u8], o: usize, what: &str) -> Result<[i16; N]> {
    let r = bytes(elf, o, 2 * N, what)?;
    Ok(std::array::from_fn(|k| h(r, 2 * k) as i16))
}

/// camera.json: the fight camera in the Tekken 3 converter's shape (OpenKumite's `kumite_sim::camera`): Tekken 3's
/// camera module with Tekken Tag's constants (FIGHT_DATA.md section 18); the code's constants as read there.
pub fn camera(elf: &[u8], file12: &[u8]) -> Result<Value> {
    let mut arenas = Vec::new();
    for i in 0..ARENA_PROJECTION.1 {
        let [h, h_min, h_max, dolly, variant] = s16s(elf, ARENA_PROJECTION.0 + 10 * i, "the arena projections")?;
        arenas.push(json!({"arena": i, "h": h, "h_min": h_min, "h_max": h_max, "dolly": dolly, "variant": variant}));
    }
    let mut situations = Vec::new();
    for i in 0..SITUATIONS.1 {
        let [frames, hold, p0, p1, p2, p3, y0, y1, y2, y3] = s16s(elf, SITUATIONS.0 + 20 * i, "the situation types")?;
        situations.push(json!({"type": i + 1, "in_frames": frames, "field_1": hold, "pitch": [p0, p1, p2, p3],
            "yaw_offset": [y0, y1, y2, y3]}));
    }
    let mut lists = Vec::new();
    for id in 0..CAMERA_LISTS.1 {
        let r = bytes(elf, CAMERA_LISTS.0 + 12 * id, 12, "the camera lists")?;
        let (list, count) = (w(r, 0) as usize, h(r, 4) as i16);
        if list == 0 || count <= 0 {
            continue;
        }
        ensure!(count as usize <= MAX_LIST, "camera list {id:#x}: {count} entries");
        let at = list.checked_sub(VADDR).with_context(|| format!("camera list {id:#x}: pointer {list:#x}"))?;
        let entries = bytes(elf, at, 8 * count as usize, "a camera list")?.as_chunks::<8>().0.iter().map(|e| {
            let ((kind, value), (mirrored_kind, mirrored)) =
                (camera_value(h(e, 4) as i16), camera_value(h(e, 6) as i16));
            json!({"flags": h(e, 0), "weight": h(e, 2) as i16, "type": kind, "value": value,
                   "mirrored_type": mirrored_kind, "mirrored_value": mirrored})
        });
        lists.push(json!({"id": id, "split": h(r, 8) as i16, "first_mirrored": h(r, 10) as i16, "entries": entries.collect::<Vec<_>>()}));
    }
    let (mut starts, mut last) = (Vec::new(), 0);
    while let Some(o) = crate::u32le(elf, CAMERA_MOTIONS + 4 * starts.len()).filter(|&o| starts.is_empty() || o > last)
    {
        last = o;
        starts.push(o as usize);
    }
    let motions = starts
        .iter()
        .enumerate()
        .map(|(k, &at)| {
            camera_motion(file12, at, starts.get(k + 1).copied().unwrap_or(file12.len()))
                .with_context(|| format!("camera motion {}", k + 1))
        })
        .collect::<Result<Vec<_>>>()?;
    // (0x3954E8 subtracts the eye's offset; Tekken 3's adds it, as OpenKumite does: written negated)
    let intro = s16s::<3>(elf, INTRO_OFFSET, "the round-start offset")?.map(|v| -i32::from(v));
    let mut team_shots = Vec::new();
    for i in 0..TEAM_SHOTS.1 {
        let v: [i16; 18] = s16s(elf, TEAM_SHOTS.0 + 36 * i, "the team intro's shots")?;
        let t: Vec<&[i16]> = v.chunks(3).collect();
        team_shots.push(json!({"turn_from": t.first(), "turn_to": t.get(1), "target_from": t.get(2), "target_to": t.get(3), "eye_from": t.get(4), "eye_to": t.get(5)}));
    }
    let mut pairs = Vec::new();
    for i in 0.. {
        let v: [i16; 14] = s16s(elf, TEAM_PAIRS + 28 * i, "the team intro's pairs")?;
        let m = |m: &[i16]| match *m {
            [character, costume, alias, x, z, facing, ..] if character != -1 => Some(
                json!({"character": character, "costume": costume, "alias": alias, "x": x, "z": z, "facing": facing}),
            ),
            _ => None,
        };
        let (Some(a), Some(b)) = (m(&v), m(v.get(6..).unwrap_or_default())) else { break };
        pairs.push(json!([a, b]));
    }
    let start_frames = (0..START_FRAMES.1)
        .map(|c| {
            let r = bytes(elf, START_FRAMES.0 + 8 * c, 8, "the intros' start frames")?;
            let (n, at) = (w(r, 0) as usize, w(r, 4) as usize);
            ensure!(n <= MAX_LIST, "character {c}: {n} intro start frames");
            let at = at.checked_sub(VADDR).with_context(|| format!("character {c}: start frames at {at:#x}"))?;
            Ok(bytes(elf, at, 4 * n, "the intros' start frames")?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|e| [h(e, 0) as i16, h(e, 2) as i16])
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?;
    let mut fighter_ids = Vec::new();
    for (table, ids) in FIGHTER_IDS {
        for id in ids {
            fighter_ids.push(json!([id, w(bytes(elf, table + 12 * id, 12, "the per-fighter camera ids")?, 8)]));
        }
    }
    let fighter_rows = (0..FIGHTER_ROWS.1)
        .map(|k| {
            let r = bytes(elf, FIGHTER_ROWS.0 + 12 * k, 12, "the per-fighter camera rows")?;
            Ok(json!([w(r, 0) as i32, h(r, 4) as i16, w(r, 8)]))
        })
        .collect::<Result<Vec<_>>>()?;
    let fighter_lists = (0..FIGHTER_LISTS.1)
        .map(|k| {
            let r = bytes(elf, FIGHTER_LISTS.0 + 8 * k, 8, "the per-fighter camera lists")?;
            let (at, n) = (w(r, 0) as usize, w(r, 4) as usize);
            ensure!(n <= MAX_LIST, "per-fighter camera list {k}: {n} entries");
            let at = at.checked_sub(VADDR).with_context(|| format!("per-fighter camera list {k} at {at:#x}"))?;
            bytes(elf, at, 0x14 * n, "a per-fighter camera list")?
                .as_chunks::<0x14>()
                .0
                .iter()
                .map(|e| {
                    let scripts = (0..3)
                        .map(|j| {
                            s16s::<16>(
                                elf,
                                (w(e, 4 + 4 * j) as usize).checked_sub(VADDR).context("a per-fighter camera script")?,
                                "a per-fighter camera script",
                            )
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(json!({"weight": h(e, 2) as i16, "scripts": scripts}))
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut replay_shots = Vec::new();
    for i in 0..REPLAY_SHOTS.1 {
        let [target, follow, p, y, r, pr, yr, rr, d, dr] = s16s(elf, REPLAY_SHOTS.0 + 20 * i, "the replay's shots")?;
        replay_shots.push(json!({"target": target, "follow": follow, "turn": [p, y, r], "turn_range": [pr, yr, rr], "distance": d, "distance_range": dr}));
    }
    let sequences = (0..REPLAY_SEQUENCES.1)
        .map(|i| {
            Ok(bytes(elf, REPLAY_SEQUENCES.0 + 4 * i, 4, "the replay's shot sequences")?
                .iter()
                .map(|&b| b as i8)
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?;
    // the win pose's reference (0x3D7DA8): light 0's direction by arena, L = -atan2(-x, -z) put in [0, 2 pi) by
    // 0x3D80E8 / 0x3D8088, * 65535 / 2 pi (gp-0x7B64 / -0x7B60) truncated; written L >> 4, 4096 a turn (0x35BC24).
    // (in a fight 0x353C88 also turns it by the background's turn gp-0x7278: `world_turn` below)
    // the arenas whose far background turns with the camera's parallax (0x417780, u32 by arena; 0x366C18), and light
    // 0 with it in a fight (0x353C88)
    let world_turn = (0..crate::effects::ARENAS)
        .map(|a| crate::u32le(elf, WORLD_TURN + 4 * a).map(|v| v != 0).context("the arenas' background turn"))
        .collect::<Result<Vec<_>>>()?;
    let light_yaw = (0..crate::effects::ARENAS)
        .map(|a| {
            let at = |k: usize| crate::f32le(elf, crate::effects::LIGHTS + 0x54 * a + 4 * k - VADDR);
            let (x, z) = (at(0).context("the arena lights")?, at(2).context("the arena lights")?);
            let tau = std::f32::consts::TAU;
            let l = -(-x).atan2(-z);
            let l = if l < 0.0 { l + tau } else { l };
            Ok((l * 65535.0 / tau) as i64 >> 4)
        })
        .collect::<Result<Vec<_>>>()?;
    let h0 = arenas.first().map_or(json!(0), |a| a["h"].clone());
    Ok(json!({
        "angle_units": "4096 per turn (camera module keeps pitch/yaw << 6)",
        // arena 0's: each arena's below (0x38DBC0 at the round reset)
        "projection_h": h0,
        // the pixels H is measured in: the fight's 512-wide frame (off screen is x >= 533 or < -20, 0x34FE7C; the
        // framing's pixel constants are Tekken 3's x 512 / 368); Tekken 3's is 368
        "screen_width": 512,
        "arena_projection": {"address": "0x507FE0", "note": "by arena (gp-0x743A); variant 1 halves situation pitches, the dolly flag makes a camera motion's zoom a dolly", "rows": arenas},
        "fight": {
            // the depth is H * 1900 / 256 (Tekken 3: 5517 at any H)
            "distance": 615 * 1900 / 256, "distance_per_h": [1900, 256],
            "sample_y_offset": -290, "margin": 150, "fit_px": 392, "dead_zone_px": 68, "lateral_easing": [1, 8],
            "pullback_out": [1, 8], "pullback_in": [1, 32], "vertical_keep_px": 156, "centre_point_world_y": -450,
            "centre_drop": 140, "centre_drop_px": "140 - 450*H/dist",
            // slot 1 (0x38FD90): pitched up its centre drop; pitched down it lifts (y + 1000) * 512 / 640 by the top
            // fighter's height y (when under 1000), eased 28 a frame
            "centre_drop_pitched_up": 180, "slot1_lift": 512,
            "vertical_out": {"rate": [1, 8], "max_step": 800}, "vertical_in": {"rate": [1, 16], "max_step": 1200},
            "yaw": {"start_threshold": 3640, "brake_zone": 14564, "freeze_above": 109226, "acc": 1536, "acc_brake": 768,
                    "damping": [14, 128], "max_velocity": 56784, "settle_frames": 32, "freeze_first_frames": 200,
                    "move_caps": {"0x13": 512, "0x14": 384, "0x15": 128, "0x16": 64}},
            "pitch_lag": {"threshold": 14563, "max_step": 256, "easing": [1, 16]},
            "initial_eye": [0, -1200, -8000],
            "routines": {"sample": "0x38EC08", "yaw": "0x38F7F8", "framing": "0x38FD90", "reset": "0x390E08", "director": "0x38C1E8"},
        },
        "intro": {"address": "0x509C20", "routine": "0x3954E8", "frames_first_round": 120, "frames_other": 90,
                  "ease_frames": 90, "blend_frames": 13, "offsets": [intro],
                  "formula": "eye = base.eye + off*(1 + sin(0xC00 + max(t - T + 90, 1)*1024/90)), t counts T..0 (the move over the first 90 frames); hand-off weight min(t*4096/13, 4096)"},
        // Before round 1 of a match of teams (screen 4 step 7, 0x3B5328: not 1 ON 1 nor the attract demo), each team on
        // the stage for a set-up frame and `frames` + 1 more (START skips), camera mode 2: side 0's first when side 1's
        // pad is in (gp-0x7440), else side 1's; against the CPU (settings mode 8, +0x1A 1) the CPU's alone. The team's member in its slot stays, its partner takes the other slot. Normally (0x3B5150)
        // player 0 at x -500, player 1 at x 500, facing each other, both in their round-1 intro (0x368398); then player 0
        // moved z + `place.z` when side 0's team shows (shot 0 on player 0's chest, +0x708), else z - `place.z` (shot 1
        // on player 1's). A pair in `pairs` (0x3B4F88: by character, and costume unless -1; not in Tekken Bowl) stands
        // at its x, z, facing (65536 a turn) playing its alias, the shot (0 for side 0's team) on the chests' midpoint,
        // `pair_scale` / 4096 on its target's from y and its eyes' z. A shot (0x3D2330, per frame 0x3D2818): each
        // point's x, y, z scaled H / 600, turned by Ry * Rx * Rz of its `turn_*` (4096 a turn; each product's terms
        // truncated) and put on the base; then each frame from + (to - from) * sin(t * 1024 / frames) / 4096, t from 0
        "team_intro": {"routine": "0x3B5328", "frames": 90, "place": {"x": 500, "z": 1500}, "shots": team_shots,
            "pair_scale": 0x1333, "pairs": pairs,
            // by character (0x447480): [alias, frame] its intro motions start at in the team intro (0x368398 while
            // gp-0x6334 is set: that frame, or the motion's frames - 90 when shorter than it + 90; 1 for an alias not
            // listed)
            "start_frames": start_frames},
        // The K.O. replay's shots (0x3D0660 at its start: a sequence by the AI's random of 14; each pass 0x3D0748 takes
        // the sequence's next; the loser: player 1 while player 0's +0xB4 >= 0): `target` 0 the winner's root (+0x51C),
        // 1 the loser's, 2 the loser's point +0x39C as kept at the K.O.; the loser's root when a fighter was in a throw
        // (0x3D0660's tests). The eye: the target + Ry * Rx * Rz of `turn` +- `turn_range` (the yaw plus the winner's
        // +0x3C / 16, the loser's after a throw) on (0, 0, distance +- range) * H / 600, at least 1500 up (`eye_y`, y
        // down); the target at most 1800 up (`target_y`). Each frame (0x3D0A38) the target moves `follow_easing` of
        // the way to `follow` (0 the winner's root, 1 the loser's, else none); the pitch (mod 4096) at most `pitch_max`
        "replay_shots": {"routine": "0x3D0748", "per_frame": "0x3D0A38", "sequences": sequences, "shots": replay_shots,
            "eye_y": -1500, "target_y": -1800, "pitch_max": 568, "follow_easing": [6, 128], "distance_per_h": 600},
        // The win pose (0x35BB58, per frame 0x35BD20; FIGHT_DATA.md 18.3): the eye at the winner's root + R (cos a,
        // sin a), a = `arc_from` + rand(`arc_range`) - the arena's `light_yaw`, R = H * `distance_per_h` + `radius_add`,
        // at `eye_y` (on a variant arena the chest - 290); each frame 8 nearer while farther than H * `distance_per_h`,
        // the view eased >> 5 (Tekken 3's camera A after; its aim without the 5800 minimum, the pitch not up on a
        // variant arena)
        "win_pose": {"routine": "0x35BB58", "arc_from": 0x400 - 113, "arc_range": 227, "distance_per_h": [5200, 955],
            "radius_add": 1440, "eye_y": -850, "light_yaw": light_yaw,
            // light 0 turned in a fight by the background's turn gp-0x7278 (0x366C18 each frame from the view: the
            // angle from the eye to last frame's point 4096 ahead against the yaw, /0x22 at most 0xAA, kept while
            // the yaw moves less than 1/256 turn; reset as a round starts, 0x366DB0)
            "world_turn": world_turn, "turn_step": [0x22, 0xAA]},
        // The per-fighter situation cameras (0x38DEA0, 0x3943A8, 0x394648): `ids` [camera id, list], `rows`
        // [character, alias, list] (when the other fighter is down) and `lists` 0x12C
        // on (weights made cumulative of 4096 at boot by 0x3941F0): entries {`weight`, three `scripts` of 16 s16 (the
        // first for a move's id): kind, frames, blend in, blend out, s32 distance from / to, then by kind four pitches
        // and four yaw offsets, or pitch from / to, the eye's lift, yaw from / to, the delay}; FIGHT_DATA.md 18.2
        "fighter_cameras": {"ids": fighter_ids, "rows": fighter_rows, "lists": fighter_lists},
        "situation_presets": {"address": "0x508908", "entries": situations},
        "move_camera_lists": {"address": "0x434C78", "note": "by camera id (move flags >> 23); weights out of the list's sum; type 1 = situation type, 2 = camera motion, 0 = none; the mirrored value when the orientation is set", "lists": lists},
        // The camera motions (disc file 12 at the offsets 0x508130, by motion - 1; 0x3923F0): `flag` the first byte
        // (0x3928B8: the yaw, by the anchor's facing f (65536 a turn): 0 / 4 0x400 - f/16, 1 0x800 - f/16, 2 -f/16, 3 / 5
        // -(f/16 + 0x400)), then per frame the shot 0x8B92D0 after it: the eye x, y, z (an offset from the target
        // when the arena's dolly flag is set), the target x, y, z, the weight (128 = all the motion's) and the
        // projection H the shot was made for (FIGHT_DATA.md 18.2)
        "camera_motions": {"address": "0x508130", "file": 12, "motions": motions},
    }))
}

/// A camera motion of disc file 12 from `at` to `end` (0x3923F0): its flag byte, then each frame a u16 of eight
/// 2-bit codes (channel 0 first: 0 none, 1 the channel's last delta again, 2 an s8, 3 an s16) and the deltas; each
/// channel's total (s16 for 0-5, s32 for 6-7) goes to the shot's +2, +0xA, +0, +4, +8, +0xC, +0x14, +0x10. Written
/// in the shot's order: [+0, +2, +4, +8, +0xA, +0xC, +0x10, +0x14].
fn camera_motion(d: &[u8], at: usize, end: usize) -> Result<Value> {
    let mut b = d.get(at..end).context("past the file")?.iter().copied();
    let flag = b.next().context("no flag")?;
    let (mut last, mut total, mut frames) = ([0i32; 8], [0i32; 8], Vec::new());
    // (the last motion ends a byte before the file's end: a pad)
    while let (Some(lo), Some(hi)) = (b.next(), b.next()) {
        let codes = u16::from_le_bytes([lo, hi]);
        for (c, (l, t)) in last.iter_mut().zip(total.iter_mut()).enumerate() {
            let delta = match codes >> (2 * c) & 3 {
                0 => 0,
                1 => *l,
                2 => i32::from(b.next().context("a cut frame")? as i8),
                _ => {
                    i32::from(i16::from_le_bytes([b.next().context("a cut frame")?, b.next().context("a cut frame")?]))
                }
            };
            *l = delta;
            *t = if c < 6 { i32::from(t.wrapping_add(delta) as i16) } else { t.wrapping_add(delta) };
        }
        let t = total;
        frames.push([t[2], t[0], t[3], t[4], t[1], t[5], t[7], t[6]]);
    }
    Ok(json!({"flag": flag, "frames": frames}))
}

/// Writes `gamedata/` into `out`; returns its files. `anims`, `frames`: each motion's file in the pack and its
/// length; `models`: the manifest's fighters.
pub fn export(
    out: &dyn Vfs,
    elf: &[u8],
    anims: &[Option<String>],
    frames: &[u8],
    models: &[Value],
    file12: &[u8],
) -> Result<Vec<String>> {
    let mut files = vec![
        ("gamedata/moves.json".to_string(), moves(elf, anims, frames)?),
        ("gamedata/cancels.json".into(), cancels(elf)?),
        ("gamedata/hits.json".into(), hits(elf)?),
        ("gamedata/reactions.json".into(), reactions(elf)?),
        ("gamedata/rules.json".into(), rules(elf)?),
        ("gamedata/camera.json".into(), camera(elf, file12)?),
        ("gamedata/ai.json".into(), ai(elf)?),
    ];
    movesets(elf, models, &mut files)?;
    for (name, v) in &files {
        out.write(name, serde_json::to_string(v)?.as_bytes())?;
    }
    Ok(files.into_iter().map(|f| f.0).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn put(elf: &mut [u8], o: usize, v: &[u16]) {
        for (k, v) in v.iter().enumerate() {
            elf[o + 2 * k..][..2].copy_from_slice(&v.to_le_bytes());
        }
    }

    #[test]
    fn camera_list_values_are_situations_or_motions() {
        let v = [0, 29, 30, 561, 562, -1].map(camera_value);
        assert_eq!(v, [(1, 1), (1, 30), (2, 1), (2, 532), (0, -1), (0, -1)]);
    }

    #[test]
    fn commands_read_as_tekken_notation() {
        assert_eq!(command_text(2), "2");
        assert_eq!(command_text(1 << 9 | 0xe), "n+2+3+4"); // direction 5
        assert_eq!(command_text(1 << 10 | 1 << 7 | 0x13), "d/f|f+(1|2)");
        assert_eq!(command_text(0x4000), "tag");
        assert_eq!(command_text(1 << 6 | 0x10), "d");
        assert_eq!(command_text(1 << 6), "d (no button)");
        assert_eq!(command_text(0x3fe0 | 0x10), "*");
        assert_eq!(step_text(0x306), "f+1+2");
        assert_eq!(step_text(0x4005), "n+tag");
    }

    #[test]
    fn a_move_with_its_lists_and_hit_chain() {
        let mut elf = vec![0u8; HITS + 4000];
        let m = MOVES + 52 * 3;
        // move 3: cancels from 7, next alias 58 turned, hit chain from 5, drift -2, sounds from 4, properties from 9
        // bones 10 0 0 0, active 8-9, hit-stop -3, push control 0x1012
        put(
            &mut elf,
            m,
            &[
                3, 0, 0x0842, 0x0100, 0x0412, 0x80, 7, 0, 58, 0x7fff, 5, 0xfffe, 0, 0, 4, 0, 9, 0, 0x0100, 0x2a00, 10,
                0, 0x0908, 0x00fd, 0x1012, 0,
            ],
        );
        put(&mut elf, HITS + 50, &[0x53, 30, 2, 0, 0, 0xffff, 0x4000 | 20, 46, 7, 900]);
        put(&mut elf, SOUNDS + 8, &[0x6049, 0x7041, 0xffff]);
        put(&mut elf, PROPS + 36, &[4, 0x0c05, 6, 0x0203, 0, 0]);
        for i in (0..MOVE_COUNT).filter(|&i| i != 3) {
            // the other moves: no lists, a chain of the one ungated record
            put(&mut elf, MOVES + 52 * i + 0x14, &[6]);
            elf[MOVES + 52 * i + 0x1c..][..8].fill(0xff);
        }
        let v = moves(&elf, &[None, None, None, Some("anims/0003.glb".into())], &[0, 0, 0, 37]).unwrap();
        let m = &v["moves"][3];
        assert_eq!(
            (m["index"].as_u64(), m["anim"].as_str(), m["frames"].as_u64()),
            (Some(3), Some("anims/0003.glb"), Some(37))
        );
        assert_eq!(m["posture"], json!({"raw": 0x0100_0842, "base": 2, "state_code": 1, "tag_state": true}));
        assert_eq!(m["attack_mask"], json!({"raw": 0x80_0412, "mask": 0x412, "level": 4, "level_name": "high"}));
        assert_eq!((&m["cancel_start"], &m["next"]), (&json!(7), &json!({"alias": 58, "hi": 0x7fff})));
        // the damage is the ungated record's, plus the property's 5
        assert_eq!(
            (m["hit"].as_u64(), m["damage"].as_u64(), m["damage_runtime"].as_u64(), m["damage_type"].as_u64()),
            (Some(5), Some(20), Some(25), Some(1))
        );
        assert_eq!(
            (m["air_drift"].as_i64(), m["hitstop"].as_i64(), &m["active"]),
            (Some(-2), Some(-3), &json!([8, 9]))
        );
        assert_eq!(m["flags24"], json!({"raw": 0x2a00_0100, "camera_script": 0x54}));
        assert_eq!(
            m["push_control"],
            json!({"raw": 0x1012, "mode": 2, "disable_mask": 1, "mask_only_near_active": true})
        );
        assert_eq!(
            m["sounds"],
            json!([{"raw": 0x6049, "type": 6, "index": 0x49}, {"raw": 0x7041, "type": 7, "index": 0x41}])
        );
        assert_eq!(m["properties"], json!([{"frame": 4, "op": 12, "arg": 5}, {"frame": 6, "op": 2, "arg": 3}]));
        assert_eq!(
            m["attack"],
            json!({"bones": {"end0": 10, "start0": 0, "end1": 0, "start1": 0}, "segment_count": 1, "object_driven": false})
        );
        assert_eq!(m["raw_hex"].as_str().unwrap().len(), 104);
        // a move without lists or attack
        let m = &v["moves"][4];
        assert!(m["sounds"].is_null() && m["properties"].is_null() && m["attack"].is_null() && m["anim"].is_null());
        // the hit records: up to the last chain's end
        let v = hits(&elf).unwrap();
        assert_eq!(v["count"], 7);
        assert_eq!(
            v["records"][5],
            json!({"index": 5, "gate": 0x53, "damage": 30, "damage_type": 0, "reaction": 2, "close_reaction": 0, "close_distance": 0})
        );
        assert_eq!(
            v["records"][6],
            json!({"index": 6, "gate": null, "damage": 20, "damage_type": 1, "reaction": 46, "close_reaction": 7, "close_distance": 900})
        );
        // lists that do not end, a cancel list outside the table, a cut executable: errors
        let mut bad = elf.clone();
        bad[SOUNDS + 8..SOUNDS + 8 + 2 * MAX_LIST].fill(1);
        assert!(format!("{:#}", moves(&bad, &[], &[]).unwrap_err()).contains("sound list 4 does not end"));
        let mut bad = elf.clone();
        put(&mut bad, MOVES + 0xc, &[0x82cf]);
        assert!(moves(&bad, &[], &[]).is_err());
        let mut bad = elf.clone();
        bad[HITS..].fill(0);
        assert!(hits(&bad).is_err() && moves(&bad, &[], &[]).is_err());
        assert!(moves(&elf[..MOVES + 100], &[], &[]).is_err() && hits(&[]).is_err());
    }

    #[test]
    fn cancel_entries_sequences_and_shared_lists() {
        let mut elf = vec![0u8; HITS];
        let entry = |elf: &mut [u8], (base, _): (usize, usize), i: usize, words: [u16; 5], bytes: [u8; 4]| {
            put(elf, base + 14 * i, &[words[0], 0, words[1], words[2], words[3]]);
            elf[base + 14 * i + 2] = (words[4] >> 8) as u8;
            elf[base + 14 * i + 3] = words[4] as u8;
            elf[base + 14 * i + 10..][..4].copy_from_slice(&bytes);
        };
        // Paul's jab: 2 on frames 1-12 commits at 8; a tag-gated reversal turned around; a sequence; a call; END
        entry(&mut elf, CANCELS, 0, [2, 0, 300, 6, 0], [0x20, 1, 12, 8]);
        entry(&mut elf, CANCELS, 1, [0x4000, 7, 301, 0x8022, 0x01e5], [0, 1, 255, 255]);
        entry(&mut elf, CANCELS, 2, [0x8010, 0, 302, 0x4003, 0], [0, 1, 20, 20]);
        entry(&mut elf, CANCELS, 3, [CALL, 0, 2, 0, 0], [0; 4]);
        entry(&mut elf, CANCELS, 4, [END, 0, 58, 6, 0], [63, 25, 255, 25]);
        entry(&mut elf, CANCELS, CANCELS.1 - 1, [END, 0, 0, 0, 0], [0; 4]);
        entry(&mut elf, SHARED, 2, [0x8001, 0, 9, 0, 0], [0; 4]);
        entry(&mut elf, SHARED, 3, [RETURN, 0, 0, 0, 0], [0; 4]);
        put(&mut elf, SEQUENCES + 2 * 0x8010, &[25, 5, 2, 3, 0x306, 0]);
        put(&mut elf, REVERSALS, &[0xffff, 7, 8, 0xffff, 9]);
        let v = cancels(&elf).unwrap();
        let e = &v["cancels"]["entries"];
        assert_eq!(v["cancels"]["count"], 0x82cf);
        assert_eq!(
            e[0],
            json!({"index": 0, "cmd": 2, "kind": "command", "input": "2", "req": 0, "cond": 0, "cond_tag": false,
            "param": 0, "dest": 300, "transition": 6, "transition_type": 6, "turn_180": false, "transition_bit14": false,
            "prefilter": 0x20, "win_start": 1, "win_end": 12, "exec": 8})
        );
        assert_eq!(
            (e[1]["input"].as_str(), e[1]["req"].as_u64(), e[1]["cond"].as_u64(), &e[1]["cond_tag"]),
            (Some("tag"), Some(1), Some(0x65), &json!(true))
        );
        assert_eq!(
            (e[1]["param"].as_u64(), e[1]["transition_type"].as_u64(), &e[1]["turn_180"]),
            (Some(7), Some(0x22), &json!(true))
        );
        assert_eq!(
            (e[2]["kind"].as_str(), e[2]["input"].as_str(), &e[2]["transition_bit14"]),
            (Some("sequence"), Some("n,d,d/f,f+1+2"), &json!(true))
        );
        assert_eq!((e[3]["kind"].as_str(), e[3]["shared_list_start"].as_u64()), (Some("call"), Some(2)));
        assert_eq!(
            (e[4]["kind"].as_str(), e[4]["win_start"].as_u64(), e[4]["dest"].as_u64()),
            (Some("end"), Some(25), Some(58))
        );
        assert_eq!(v["shared_cancels"]["list_starts"], json!([2]));
        assert_eq!(
            (v["shared_cancels"]["entries"][2]["kind"].as_str(), v["shared_cancels"]["entries"][3]["kind"].as_str()),
            (Some("dash"), Some("return"))
        );
        let s = &v["sequences"]["entries"];
        assert_eq!(s.as_array().unwrap().len(), 1);
        assert_eq!(
            (s[0]["cmd"].as_u64(), s[0]["window"].as_i64(), s[0]["input"].as_str()),
            (Some(0x8010), Some(25), Some("n,d,d/f,f+1+2"))
        );
        assert_eq!(s[0]["steps"][3], json!({"raw": 0x306, "dir": 6, "buttons": [1, 2], "tag": false}));
        assert_eq!(v["reversal_families"]["families"].as_array().unwrap()[..2], [json!([]), json!([7, 8])]);
        assert_eq!(v["reversal_families"]["offsets"].as_array().unwrap()[..2], [json!(0), json!(1)]);
        // a call outside the shared lists, a sequence that does not end, a table without its END, a cut executable
        let mut bad = elf.clone();
        entry(&mut bad, CANCELS, 3, [CALL, 0, 782, 0, 0], [0; 4]);
        assert!(cancels(&bad).is_err());
        let mut bad = elf.clone();
        bad[SEQUENCES + 2 * 0x8010..][..2 * MAX_LIST + 4].fill(1);
        assert!(cancels(&bad).is_err());
        let mut bad = elf.clone();
        entry(&mut bad, CANCELS, CANCELS.1 - 1, [2, 0, 0, 0, 0], [0; 4]);
        assert!(cancels(&bad).is_err());
        assert!(cancels(&elf[..CANCELS.0 + 100]).is_err());
    }

    #[test]
    fn reactions_curves_spheres_and_health() {
        let mut elf = vec![0u8; RECOVERABLE + 8];
        let curve = |k: usize| ((CURVES.0 + VADDR + 24 * k) as u32).to_le_bytes();
        let r = REACTIONS.0 + 56 * 2;
        elf[r..r + 4].copy_from_slice(&57345u32.to_le_bytes());
        elf[r + 4..r + 8].copy_from_slice(&curve(73));
        elf[r + 24..r + 28].copy_from_slice(&curve(1));
        put(&mut elf, r + 28, &[3011, 3011, 3012]);
        put(&mut elf, CURVES.0 + 24, &[3, 0xffce, 200, 100, 50, 40, 20, 0, 0, 0, 0, 0xffff]);
        // air reactions: 1 in the first table, a move's own 9 in the second
        put(&mut elf, AIR.0 + 28, &[4762, 550, 400, 6, 31, 40]);
        put(&mut elf, AIR.1 + 28 * 9, &[3469, 0, 0, 0, 0xffec, 0xffec, 0xffec]);
        elf[MOVES + 52 * 5 + 0x2f] = 9;
        let v = reactions(&elf).unwrap();
        let air = v["air_reactions"]["entries"].as_array().unwrap();
        assert_eq!(air.len(), 10);
        assert_eq!(
            air[1],
            json!({"index": 1, "alias": 4762, "target_height": 550, "rise": 400, "words": [6, 31, 40], "pushback": [0, 0, 0, 0, 0, 0, 0, 0]})
        );
        assert_eq!(
            (&air[9]["alias"], &air[9]["words"], &air[9]["pushback"][0]),
            (&json!(3469), &json!([0, -20, -20]), &json!(-20))
        );
        assert_eq!(
            (v["entries"].as_array().unwrap().len(), v["pushback_curves"]["entries"].as_array().unwrap().len()),
            (1431, 74)
        );
        assert_eq!(v["entries"][2]["params"], json!([57345, 73, 0, null, 0, null, 1]));
        assert_eq!(
            v["entries"][2]["victims"].as_array().unwrap()[..4],
            [json!(3011), json!(3011), json!(3012), json!(0)]
        );
        assert_eq!(
            v["pushback_curves"]["entries"][1],
            json!({"index": 1, "constant_frames": 3, "velocity": -50, "impulses": [200, 100, 50, 40, 20, 0, 0, 0, 0, -1]})
        );
        // a curve pointer between two curves, or past the last
        for bad in [(CURVES.0 + VADDR + 25) as u32, (CURVES.0 + VADDR + 24 * 74) as u32, 5] {
            let mut e = elf.clone();
            e[r + 4..r + 8].copy_from_slice(&bad.to_le_bytes());
            assert!(reactions(&e).is_err(), "{bad:#x}");
        }
        assert!(reactions(&elf[..REACTIONS.0 + 56]).is_err());

        // rules: two radius sets, body 11 on the second
        let (normal, large) = (0x1c7180, 0x1c71a0);
        put(&mut elf, normal, &[200, 200, 120]);
        put(&mut elf, large, &[300, 300, 200]);
        for body in 0..BODIES {
            let p = (if body == 11 { large } else { normal } + VADDR) as u32;
            elf[HURT.0 + 4 * (14 + body)..][..4].copy_from_slice(&p.to_le_bytes());
            elf[PUSH.0 + 4 * (8 + body)..][..4].copy_from_slice(&p.to_le_bytes());
        }
        // joint places: every body's at one table, slot 13 (shin A) at (390, 0, -1)
        let places = 0x1c7200;
        put(&mut elf, places + 8 * 13, &[390, 0, 0xffff]);
        for body in 0..BODIES {
            elf[JOINTS + 4 * body..][..4].copy_from_slice(&((places + VADDR) as u32).to_le_bytes());
        }
        elf[HURT.0 + 4 * 8] = 2;
        elf[LIFE..LIFE + 4].copy_from_slice(&(125u32 << 12).to_le_bytes());
        elf[LIFE + 32 + 4 * 11] = 120;
        elf[RECOVERABLE..RECOVERABLE + 4].copy_from_slice(&0.6f32.to_le_bytes());
        let v = rules(&elf).unwrap();
        let hurt = &v["hurt_spheres"];
        assert_eq!(
            hurt["layout"][8],
            json!({"index": 8, "bone_slot": 2, "offset_game": [0, -120, 0], "gltf_joint": "head", "offset_gltf_m": [0.0, 120.0 * 0.001, 0.0]})
        );
        assert_eq!(hurt["layout"][11]["offset_game"], json!([0, -60, 0]));
        assert_eq!(hurt["per_character"].as_array().unwrap().len(), 34);
        assert_eq!(hurt["per_character"][0]["radii"].as_array().unwrap()[..3], [json!(200), json!(200), json!(120)]);
        assert_eq!(hurt["per_character"][11]["radii"][2], 200);
        assert_eq!(
            (
                v["push_spheres"]["layout"].as_array().unwrap().len(),
                v["push_spheres"]["per_character"][11]["radii"].as_array().unwrap().len()
            ),
            (8, 8)
        );
        assert_eq!(
            (v["health"]["life_options"][0].as_u64(), v["health"]["percent_per_character"][11].as_u64()),
            (Some(125), Some(120))
        );
        assert_eq!(v["joints"]["per_character"][21]["places"][13], json!([390, 0, -1]));
        assert_eq!(
            (v["joints"]["parents"][14].as_i64(), v["joints"]["per_model"][0]["model_record"].as_u64()),
            (Some(13), Some(666))
        );
        assert_eq!(v["health"]["recoverable"]["damage_factor"]["bits"], 0x3f19_999a);
        assert_eq!(v["tag"]["anger"]["relations"].as_array().unwrap().len(), 76);
        // a radius pointer outside the executable
        elf[HURT.0 + 4 * 14..][..4].copy_from_slice(&0xffff_fff0u32.to_le_bytes());
        assert!(rules(&elf).is_err());
    }

    #[test]
    fn movesets_and_their_index() {
        let mut elf = vec![0u8; ALIASES + 2 * MOVESETS * ALIAS_COUNT];
        put(&mut elf, ALIASES + 2 * ALIAS_COUNT, &[7350, 806, 806, 2]);
        // the roster: "LAW" on body 1 with moveset 1
        let name = 0x1000;
        elf[name..name + 4].copy_from_slice(b"LAW\0");
        elf[0x1bcc20..][..4].copy_from_slice(&((name + VADDR) as u32).to_le_bytes());
        elf[0x1bcc24] = 1;
        elf[0x1bcc27] = 1;
        let models = [
            json!({"id": 348, "file": "models/348.glb", "body": 1, "moveset": 1}),
            json!({"id": 9, "file": "models/009.glb", "body": null, "moveset": null}),
        ];
        let mut out = vec![];
        movesets(&elf, &models, &mut out).unwrap();
        assert_eq!(out.len(), 34);
        let (name, set) = &out[1];
        assert_eq!(name, "gamedata/movesets/moveset_01.json");
        assert_eq!((set["aliases"].as_array().unwrap().len(), &set["move_count"]), (5532, &json!(3)));
        assert_eq!(set["aliases"].as_array().unwrap()[..4], [json!(null), json!(806), json!(806), json!(2)]);
        assert_eq!(
            set["fighters"],
            json!([{"name": "Law", "body": 1, "moveset_id": 1, "moveset_file": "moveset_01.json"}])
        );
        let index = &out[33].1;
        assert_eq!(
            index["models"],
            json!([{"model": 348, "model_record": 348, "glb": "models/348.glb", "body": 1, "moveset_id": 1, "moveset_file": "moveset_01.json", "hands": null}])
        );
        assert_eq!((index["files"].as_array().unwrap().len(), index["fighters"].as_array().unwrap().len()), (33, 1));
        // an alias naming a move past the last
        put(&mut elf, ALIASES, &[7351]);
        assert!(movesets(&elf, &models, &mut vec![]).is_err());
    }
}

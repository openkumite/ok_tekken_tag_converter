//! The fight data in the executable (docs/FIGHT_DATA.md), written as the pack's `gamedata/*.json` (docs/GAMEDATA.md):
//! moves, cancel lists, hit records, reactions, movesets, hurt and push volumes, health. The shapes are the Tekken 3
//! converter's wherever the game kept the concept; the numbers (aliases, conditions, transitions, req codes, flag
//! bits, property ops) are Tekken Tag's own, as stored. Integers only, but two float constants written with their bits.

use crate::moves::{self, ALIAS_COUNT, ALIASES, BODIES, MOVE_COUNT, MOVES, MOVESETS, VADDR};
use crate::{bytes, u16_at, u16le, u32_at};
use anyhow::{Context, Result, bail, ensure};
use ok_converter_common::io::Vfs;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

// (address in memory - VADDR = offset in the file; entries)
const CANCELS: (usize, usize) = (0x482248 - VADDR, 0x82cf); // 14 bytes each, the moves' lists one after another
const SHARED: (usize, usize) = (0x47f780 - VADDR, 782); // the lists a CALL enters, each up to a RETURN
const SEQUENCES: usize = 0x4efd44 - VADDR; // the motion input of command c is at + 2 * c
const PROPS: usize = 0x524690 - VADDR; // timed properties: u16 frame, u16 op << 8 | arg; a list ends at frame 0
const SOUNDS: usize = 0x52c448 - VADDR; // sound lists: u16 type << 12 | index; a list ends at 0xffff
const HITS: usize = 0x5e8978 - VADDR; // hit records of 10 bytes; a chain ends at gate 0xffff
const REVERSALS: usize = 0x5e8608 - VADDR; // s16 motion lists ending in -1, up to the hit records
const REACTIONS: (usize, usize) = (0x46be78 - VADDR, 1431); // 56 bytes each, up to the shared cancels
const CURVES: (usize, usize) = (0x523fa0 - VADDR, 74); // 24 bytes each, up to the timed properties
const LIFE: usize = 0x51f5a0 - VADDR; // 8 x u32 health, 20.12 fixed point; then a percent per body
const HURT: (usize, usize) = (0x42f060 - VADDR, 14); // u32 bone slot per sphere, then a pointer per body to s16 radii
const PUSH: (usize, usize) = (0x42efb8 - VADDR, 8);
const RECOVERABLE: usize = 0x5f5168 - VADDR; // f32 0.6, f32 1.4: the part of a hit's damage the recoverable cap loses

const END: u16 = 0x8000;
const CALL: u16 = 0x8004;
const RETURN: u16 = 0x8005;
/// The alias table's "no move".
const MISSING: u16 = 7350;
/// No list of the game is longer (the longest, a sound list, is under 40 entries): one that does not end is an error.
const MAX_LIST: usize = 256;

/// A u16 / u8 inside an entry already read.
fn h(e: &[u8], o: usize) -> u16 {
    u16le(e, o).unwrap_or_default()
}
fn b(e: &[u8], o: usize) -> u8 {
    e.get(o).copied().unwrap_or_default()
}
fn w(e: &[u8], o: usize) -> u32 {
    crate::u32le(e, o).unwrap_or_default()
}

/// The `size`-byte entries from entry `i` of the table at `base` up to (not with) the one `last` knows.
fn until<'a>(
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
            "active": active, "hitstop": b(r, 0x2e) as i8, "byte_2F": b(r, 0x2f),
            "flags24": {"raw": flags, "camera_script": flags >> 24},
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
            Some(json!({"model": m.get("id")?, "model_record": m.get("id")?, "glb": m.get("file")?, "body": m.get("body")?, "moveset_id": set, "moveset_file": moveset_file(set)}))
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
    Ok(json!({
        // which game's numbers the fight data holds (docs/GAMEDATA.md "Numbering")
        "dialect": "ttt",
        "health": {
            "representation": "20.12 fixed point at player +0x3F4; a body's health = life option x its percent / 100",
            "life_table": "0x51F5A0", "life_options": life.iter().map(|v| v >> 12).collect::<Vec<_>>(), "life_options_raw": life,
            "percent_table": "0x51F5C0", "percent_per_character": percent,
            "recoverable": {"address": "0x5F5168", "damage_factor": factor(0)?, "damage_factor_flag_12a": factor(1)?},
        },
        // sphere 8 (the head) is lifted 120, sphere 11 (the root) 60: y is down
        "hurt_spheres": spheres(HURT, elf, &[(8, -120), (11, -60)], &names)?,
        "push_spheres": spheres(PUSH, elf, &[], &names)?,
    }))
}

/// Writes `gamedata/` into `out`; returns its files. `anims`, `frames`: each motion's file in the pack and its
/// length; `models`: the manifest's fighters.
pub fn export(
    out: &dyn Vfs,
    elf: &[u8],
    anims: &[Option<String>],
    frames: &[u8],
    models: &[Value],
) -> Result<Vec<String>> {
    let mut files = vec![
        ("gamedata/moves.json".to_string(), moves(elf, anims, frames)?),
        ("gamedata/cancels.json".into(), cancels(elf)?),
        ("gamedata/hits.json".into(), hits(elf)?),
        ("gamedata/reactions.json".into(), reactions(elf)?),
        ("gamedata/rules.json".into(), rules(elf)?),
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
        assert_eq!(m["flags24"], json!({"raw": 0x2a00_0100, "camera_script": 0x2a}));
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
        let v = reactions(&elf).unwrap();
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
        assert_eq!(v["health"]["recoverable"]["damage_factor"]["bits"], 0x3f19_999a);
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
            json!([{"model": 348, "model_record": 348, "glb": "models/348.glb", "body": 1, "moveset_id": 1, "moveset_file": "moveset_01.json"}])
        );
        assert_eq!((index["files"].as_array().unwrap().len(), index["fighters"].as_array().unwrap().len()), (33, 1));
        // an alias naming a move past the last
        put(&mut elf, ALIASES, &[7351]);
        assert!(movesets(&elf, &models, &mut vec![]).is_err());
    }
}

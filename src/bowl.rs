//! Tekken Bowl (docs/BOWL_ENDINGS.md sections 1-6): `gamedata/bowl.json`, the tables and constants OpenKumite's bowl
//! reads (docs/GAMEDATA.md "bowl.json"), and the bowling hall's objects the bowl draws: the ball, the pin, the pin
//! deck's machinery.

use crate::moves::{self, VADDR};
use crate::stage::Batch;
use crate::{f32le, u8_at, u16_at, u32_at};
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// The bowling hall's arena number (geometry file 189 + 7 x 20).
pub const ARENA: usize = 20;
/// Scene 42's motion pack (0x363FB8 -> 0x35CFD0): its part 0 is the bowler's motions.
pub const MOTIONS: usize = 128;
/// The executable's gp.
const GP: usize = 0x5F_D0F0;
/// Per character (FE ids 0-33): `{s16 S, u8 left-handed, u8 female style}`, the four strike reactions, the three
/// voices (`u16 x 3`, then 2 bytes).
const STATS: usize = 0x52_3618;
const REACTIONS: usize = 0x52_3590;
const VOICES: usize = 0x52_36A0;
const CHARACTERS: usize = 34;
/// The strike pose by moveset (33 x s16).
const POSES: usize = 0x52_37D8;
/// The callouts' string pointers: the strikes' six, MISS / FOUL, SPARE / NICE COVER.
const STRIKES: usize = 0x52_3A18;
const MISS: usize = 0x5F_69D8;
const SPARE: usize = 0x5F_6A00;

/// A string of the executable at a pointer.
fn text(elf: &[u8], pointer: u32) -> String {
    let at = (pointer as usize).wrapping_sub(VADDR);
    let raw: Vec<u8> = elf.get(at..).unwrap_or_default().iter().take(32).take_while(|&&c| c != 0).copied().collect();
    String::from_utf8_lossy(&raw).to_string()
}

/// `gamedata/bowl.json` from the executable; `motions`: the bowler's motions' frame counts (file 128 part 0).
pub fn bowl(elf: &[u8], motions: &[usize]) -> Result<Value> {
    let what = "Tekken Bowl's tables";
    let a = |addr: usize| addr - VADDR;
    let gp = |off: usize| -> Result<f64> { Ok(f32le(elf, a(GP - off)).context(what)? as f64) };
    let strings = |addr: usize, n: usize| -> Result<Vec<String>> {
        (0..n).map(|k| Ok(text(elf, u32_at(elf, a(addr) + 4 * k, what)?))).collect()
    };
    let mut characters = vec![];
    for c in 0..CHARACTERS {
        let s = u16_at(elf, a(STATS) + 4 * c, what)? as i16;
        let reactions: Vec<u8> = (0..4).map(|k| u8_at(elf, a(REACTIONS) + 4 * c + k, what)).collect::<Result<_>>()?;
        let voices: Vec<u16> = (0..3).map(|k| u16_at(elf, a(VOICES) + 8 * c + 2 * k, what)).collect::<Result<_>>()?;
        characters.push(json!({"id": c, "s": s, "left": u8_at(elf, a(STATS) + 4 * c + 2, what)? != 0,
            "female": u8_at(elf, a(STATS) + 4 * c + 3, what)? != 0, "reactions": reactions, "voices": voices}));
    }
    let poses: Vec<i16> = (0..33).map(|k| Ok(u16_at(elf, a(POSES) + 2 * k, what)? as i16)).collect::<Result<_>>()?;
    // the rack (0x3EA050): pin k's x and z offsets, z + 8000
    let rack = [
        (0, 0x7754),
        (0x7750, 0x774C),
        (0x7748, 0x7744),
        (0x7740, 0),
        (0, 0),
        (0x773C, 0),
        (0x7738, 0x7728),
        (0x7734, 0x7728),
        (0x7730, 0x7728),
        (0x772C, 0x7728),
    ];
    let at = |off: usize| if off == 0 { Ok(0.0) } else { gp(off) };
    let pins: Vec<[f64; 2]> = rack.iter().map(|&(x, z)| Ok([at(x)?, at(z)? + 8000.0])).collect::<Result<_>>()?;
    // the 25 beside the lane (z 250-7500), which the ball can reach; the 9 behind the bowler are not tested
    let spectators: Vec<Value> = (moves::cast(elf, ARENA)?.iter())
        .filter(|p| p.pos[2] > 0.0)
        .map(|p| json!({"model": p.model, "x": p.pos[0], "y": p.pos[1], "z": p.pos[2], "heading": p.heading}))
        .collect();
    let [strike, double, turkey, great, punch_out, perfect]: [String; 6] =
        strings(STRIKES, 6)?.try_into().unwrap_or_default();
    let (miss, spare) = (strings(MISS, 2)?, strings(SPARE, 2)?);
    Ok(json!({
        "routine": "0x3F0008", "arena": ARENA,
        "motions": {"file": MOTIONS, "clip": "bowl_NNN", "frames": motions,
            "hand": {"note": "slot + 4 for a left-hander", "stand": 0, "step_right": 1, "step_left": 2, "throw": 3},
            "charge": {"run": 0x6A, "after": 0x6E, "female": 2, "left": 1}},
        "characters": characters,
        "copied": {"note": "Mokujin (15) and Unknown (33) bowl with the stats of the character they copy; Unknown is left-handed unless she copies Yoshimitsu", "mokujin": 15, "unknown": 33},
        "strike_poses": poses,
        "throw": {"routine": "0x3EAF78", "bowler_z": -11000, "start_x": [-64, 64], "step": 8, "x_limit": 1000,
            "turn": 0x10, "turn_limit": 0x800, "angle_units": 65536,
            "spin_bounds": {"right": [-2.0, 1.0], "left": [-1.0, 2.0], "jacks": [-gp(0x7714)?, gp(0x7714)?], "jack_movesets": [0x10, 0x13]},
            "stat_knee": 400, "stat_above": gp(0x776C)?, "spin_speed": [gp(0x7764)?, gp(0x7768)?, gp(0x7760)?],
            "power_speed": "2 (S' + 180) / 340", "power_max": 100, "over_charged": "S / 2000",
            "throw_frames": 150, "release_frame": 106, "steer": 1280,
            "release": "speed = S (power + 100) / 200; vel = speed (sin a, 0, cos a); spin = m (cos a, 0, -sin a); m and power each + their meter speed x U(-1, 1)",
            "held_offset": 150, "message_frames": 60},
        "charge": {"frames": 175, "voice_frame": 25, "distance": "256 f to 25, 256 f - (f - 25)^2 to 50, an ease to 75, then 14825", "end": 14825},
        "ball": {"routine": "0x3EC768", "start_z": gp(0x775C)?, "radius": 150, "lane_y": -150, "fall_y": gp(0x7680)?,
            "gravity": gp(0x767C)?, "bounce": gp(0x7674)?, "hook_start": gp(0x7670)?, "hook_length": gp(0x766C)?,
            "hook_keeps_speed_from": 20, "roll_ends": gp(0x7548)?, "charge_ends": 8000, "frames": [150, 1000]},
        "pins": {"y": gp(0x7758)?, "rack": pins, "side": 455, "gravity": gp(0x7638)?, "contacts": [80, 60, 100, 75],
            "down_radius2": gp(0x7620)?, "bounce": [0.4, 0.8],
            "walls": {"from_z": gp(0x75D4)?, "x": gp(0x75D0)?.abs(), "chamfer": 106, "back_z": 12691, "restitution": 0.4}},
        "spectators": {"routine": "0x3EFDB8", "radius2": gp(0x7544)?, "ko_frames": 180, "ko_at": 90, "voice": 0x1059, "people": spectators},
        "speed": {"routine": "0x3EAC88", "factor": gp(0x771C)?, "fast": 20, "minus": 6, "slow": 0.7, "mph": 0.625},
        "sounds": {"touch": [0x10AA, 0x10AB], "touch_fast_from": 270, "pins": [0x10AE, 0x10AD, 0x10AC], "pin_levels": [15, 30],
            "pin_quiet": [8, 14, 30], "strike": {"1": 0x10B2, "3": 0x1046, "12": 0x1058, "else": 0x1057, "then": 0x10B5}},
        "callouts": {"strikes": [strike, double, turkey, great], "punch_out": punch_out, "perfect": perfect,
            "miss": miss.first(), "foul": miss.get(1), "spare": spare.first(), "nice_cover": spare.get(1),
            "split": "SPLIT", "ko": "K.O.", "over_charged": "OVER CHARGED"},
        "split_gap": 500,
        "camera": {"routine": "0x3EBC98", "fixed": [0, -2300, -12000], "bowler_y": -2350, "ball_to": [0, -1500, 4000], "ball_until_z": 4000},
        "models": {"ball": "stages/bowl_ball.glb", "pin": "stages/bowl_pin.glb", "deck": "stages/bowl_deck.glb", "pin_base": 250.66},
    }))
}

/// The arenas' object tables (0x3D0E18), by arena: `{u32 links, u32 *links, u32 objects, u32 *objects}`.
const OBJECTS: usize = 0x52_0FC0;
/// The geometry pack's part (of the non-empty ones) whose blocks are the arena's object models.
pub const OBJECT_PART: usize = 3;

/// The bowling hall's objects the bowl draws, from arena 20's object table: an object (64 bytes) is `s16 model` (a
/// block of the object part, [`crate::stage::blocks`]) and, at +0x20, its place (x, y, z); a link (s16 parent, s16
/// child) places the child from its parent. The roll's draw (0x3EEF00 -> 0x3D10D0) draws object 0 at the ball and
/// object 1 at each pin (2 and 3 are their shadows, not kept); the linked ones are the pin deck's machinery (masking
/// unit, its sign, the back), standing where the table puts them. Returns the ball, the pin and the deck, placed.
pub fn models(elf: &[u8], blocks: &[Vec<Batch>]) -> Result<[Vec<Batch>; 3]> {
    let what = "the arena's object table";
    let at = |p: u32| (p as usize).checked_sub(VADDR).context(what);
    let table = at(u32_at(elf, OBJECTS - VADDR + 16 * ARENA, what)?)?;
    let word = |k: usize| u32_at(elf, table + 4 * k, what);
    let (links, link_list, count, list) = (word(0)? as usize, at(word(1)?)?, word(2)? as usize, at(word(3)?)?);
    let s16 = |o: usize| -> Result<i16> { Ok(u16_at(elf, o, what)? as i16) };
    let pairs: Vec<(usize, usize)> = (0..links.min(64))
        .map(|k| Ok((s16(link_list + 4 * k)? as usize, s16(link_list + 4 * k + 2)? as usize)))
        .collect::<Result<_>>()?;
    let place = |k: usize| -> Result<[f32; 3]> {
        let o = list + 64 * k + 0x20;
        Ok([0, 4, 8].map(|d| f32le(elf, o + d).unwrap_or(0.0)))
    };
    // object k's model where the table puts it (its parent's place and its own)
    let placed = |k: usize| -> Result<Vec<Batch>> {
        if k >= count.min(64) {
            return Ok(vec![]);
        }
        let mut p = place(k)?;
        if let Some(&(parent, _)) = pairs.iter().find(|l| l.1 == k) {
            let q = place(parent)?;
            p = [p[0] + q[0], p[1] + q[1], p[2] + q[2]];
        }
        let model = usize::try_from(s16(list + 64 * k)?).unwrap_or(usize::MAX);
        let mut m = blocks.get(model).cloned().unwrap_or_default();
        for v in m.iter_mut().flat_map(|b| b.pos.iter_mut()) {
            *v = [v[0] + p[0], v[1] + p[1], v[2] + p[2]];
        }
        Ok(m)
    };
    let mut deck: Vec<usize> = pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
    deck.sort_unstable();
    deck.dedup();
    let deck = deck.into_iter().map(placed).collect::<Result<Vec<_>>>()?.concat();
    Ok([placed(0)?, placed(1)?, deck])
}

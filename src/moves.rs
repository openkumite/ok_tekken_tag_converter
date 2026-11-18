//! Tables in the executable: which fighter file is which body and costume, and each moveset's moves.
use crate::{bytes, u8_at, u16_at, u32_at};
use anyhow::Result;
use std::collections::HashSet;

const SLOTS: usize = 0x1c0690; // 34 bodies x 8 costumes: i16 slot, fighter file = 345 + 3 * slot
const MOVES: usize = 0x1f2ea0; // 7351 records of 52 bytes; record i plays motion i
const MOVE_COUNT: usize = 7351;
const ALIASES: usize = MOVES + 52 * MOVE_COUNT + 4; // 33 movesets x 5532 u16 move numbers
const ALIAS_COUNT: usize = 5532;
pub const MOVESETS: usize = 33;

const ROSTER: usize = 0x1bcc20; // 50 rows of 16 bytes: name pointer, u8 body, u8, u8, u8 moveset, u8; a long name fills the next row
const VADDR: usize = 0x33f000; // address in memory - offset in the file
const BODIES: usize = 34;

const STAGES: usize = 0x101770; // 20 rows: u32 name pointer, u32 arena number (its geometry is file 189 + 7 * number)

/// The capitalised text a pointer in the executable leads to ("ARMOR KING" -> "Armor King").
fn text(elf: &[u8], pointer: u32) -> Option<String> {
    let mut word = false;
    let name = elf.get((pointer as usize).wrapping_sub(VADDR)..)?.iter().take_while(|&&c| c != 0);
    Some(
        name.map(|&c| {
            let c = c as char;
            let up = !word;
            word = c.is_ascii_alphabetic();
            if up { c } else { c.to_ascii_lowercase() }
        })
        .collect(),
    )
}

/// The stage-select name of arena `number`; the disc also holds a few arenas that menu does not list.
pub fn arena(elf: &[u8], number: usize) -> Result<Option<String>> {
    let what = "the stage table";
    for r in (0..20).map(|i| STAGES + 8 * i) {
        if u32_at(elf, r + 4, what)? as usize == number {
            return Ok(text(elf, u32_at(elf, r, what)?));
        }
    }
    Ok(None)
}

const CAST: usize = 0x114fc8; // per arena: u32 pointer to a list of people, a second list, a table of motion data

/// A background character of an arena: which of its models, where, facing (degrees about the
/// vertical axis, 0 = +z), the motions it goes through in turn, and the frame it starts at.
pub struct Person {
    pub model: usize,
    pub pos: [f32; 3],
    pub heading: f32,
    pub motions: Vec<usize>,
    pub start: usize,
}

/// The people of arena `number`. A list is 32-byte records up to one at the origin:
///   u16 ?, u16 start frame, u16 ?, u16 ?, f32 x, y, z, heading, u8 ?, u8 model, u8 motion, u8 ?, u32 script
/// A script (used when the motion is 255) is 16-byte steps up to a null pointer: u32 pointer to a
/// flag, u16 1, u8 motion (255 = none), u8 flag. A few steps name a motion one past the arena's last
/// (King 8, Hwoarang 6); what that means is not known and those steps are skipped.
// shortcut: a script's conditions and its 255 steps are ignored, its motions simply follow each other
pub fn cast(elf: &[u8], number: usize) -> Result<Vec<Person>> {
    let what = "an arena's people";
    let file = |p: u32| Some((p as usize).wrapping_sub(VADDR)).filter(|&o| p != 0 && o.saturating_add(32) <= elf.len());
    let mut out = vec![];
    for k in 0..2 {
        let Some(list) = file(u32_at(elf, CAST + 12 * number + 4 * k, what)?) else { continue };
        for r in (0..256).map(|i| list + 32 * i) {
            let Ok(r) = bytes(elf, r, 32, what) else { break };
            let f = |k: usize| u32_at(r, 8 + 4 * k, what).map(f32::from_bits);
            let pos = [f(0)?, f(1)?, f(2)?];
            if pos == [0.; 3] {
                break;
            }
            let mut motions: Vec<usize> =
                Some(u8_at(r, 26, what)? as usize).filter(|&m| m != 255).into_iter().collect();
            if let Some(script) = file(u32_at(r, 28, what)?).filter(|_| motions.is_empty()) {
                let steps = (0..64)
                    .map_while(|i| bytes(elf, script + 16 * i, 16, what).ok())
                    .take_while(|s| !s.starts_with(&[0; 4]));
                motions = steps.filter_map(|s| s.get(6).map(|&m| m as usize)).filter(|&m| m != 255).collect();
            }
            out.push(Person {
                model: u8_at(r, 25, what)? as usize,
                pos,
                heading: f(3)?,
                motions,
                start: u16_at(r, 2, what)? as usize,
            });
        }
    }
    Ok(out)
}

/// (name, moveset) of each body, from the game's roster. A fighter that is a costume of another's
/// body (Tiger, Panda) has a body number past the last one: its name joins the body with its moveset.
pub fn roster(elf: &[u8]) -> Result<Vec<(String, usize)>> {
    let what = "the roster";
    let mut out = vec![(String::new(), 0); BODIES];
    for r in (0..50).map(|i| ROSTER + 16 * i) {
        let (pointer, body, set) =
            (u32_at(elf, r, what)?, u8_at(elf, r + 4, what)? as usize, u8_at(elf, r + 7, what)? as usize);
        let Some(name) = text(elf, pointer) else { continue }; // text, not a pointer: the previous row's name
        if let Some(o) = out.get_mut(body) {
            *o = (name, set);
        } else if let Some(o) = out.iter_mut().find(|o| !o.0.is_empty() && o.1 == set) {
            o.0 = format!("{} / {name}", o.0);
        }
    }
    Ok(out)
}

/// (body, costume) of the fighter in costume slot `slot`.
pub fn fighter(elf: &[u8], slot: usize) -> Result<Option<(usize, usize)>> {
    for i in 0..BODIES * 8 {
        if u16_at(elf, SLOTS + 2 * i, "the costume table")? as usize == slot {
            return Ok(Some((i / 8, i % 8)));
        }
    }
    Ok(None)
}

/// The moves (= motion numbers) of moveset `m`, in alias order without repeats.
pub fn moveset(elf: &[u8], m: usize) -> Result<Vec<usize>> {
    let (mut seen, mut out) = (HashSet::new(), Vec::new());
    for i in 0..ALIAS_COUNT {
        let v = u16_at(elf, ALIASES + 2 * (m * ALIAS_COUNT + i), "the movesets")? as usize;
        // 7350 marks a missing alias
        if v < MOVE_COUNT - 1 && seen.insert(v) {
            out.push(v);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn tables_outside_the_executable_are_errors() {
        // a cut or empty executable: every table read is an error, not a panic
        for elf in [&[][..], &[0u8; 0x101780][..]] {
            assert!(arena(elf, 3).is_err());
            assert!(cast(elf, 3).is_err());
            assert!(roster(elf).is_err());
            assert!(fighter(elf, 1000).is_err());
            assert!(moveset(elf, 32).is_err());
        }
        // zeros where the tables are: nobody, no names
        let elf = vec![0u8; ALIASES + 2 * MOVESETS * ALIAS_COUNT];
        assert!(arena(&elf, 3).unwrap().is_none());
        assert!(cast(&elf, 3).unwrap().is_empty());
        assert!(roster(&elf).unwrap().iter().all(|r| r.0.is_empty()));
        assert_eq!(fighter(&elf, 0).unwrap(), Some((0, 0)));
        assert_eq!(moveset(&elf, 32).unwrap(), [0]);
    }

    #[test]
    fn text_is_capitalised_by_word() {
        let mut elf = vec![0u8; 32];
        elf[4..15].copy_from_slice(b"ARMOR KING\0");
        assert_eq!(text(&elf, (VADDR + 4) as u32).unwrap(), "Armor King");
        assert!(text(&elf, 0x20).is_none()); // text, not a pointer
    }
}

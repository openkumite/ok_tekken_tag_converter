//! Tekken Tag Tournament (PlayStation 2, USA v2.00, SLUS-20001) converted into a pack: the disc's one big file read
//! through its table (`archive`), its fighters (`model`), arenas (`stage`), textures (`tex`), motions (`anim`),
//! sounds and streams (`audio`), the executable's tables that name and join them (`moves`) and its fight data (`fight`), the front end's tables and pictures (`front`), the endings (`ending`), sound and music tables (`sound`), shadows and effects (`effects`), written as glb (`glb`),
//! PNG and WAV into a pack (`pack`), with the part OpenKumite reads under `assets/` (`assets`). Files go through `ok_converter_common::io::Vfs`; `main` is the command line.
//! The formats: docs/FORMATS.md.
//!
//! Everything on the disc is little-endian and untrusted: a read outside the data is an error or "not this format",
//! never a panic.

// (roster.json `front_end` is one json! literal, deeper than the default limit)
#![recursion_limit = "256"]

pub mod anim;
pub mod archive;
pub mod assets;
pub mod audio;
pub mod bowl;
pub mod effects;
pub mod ending;
pub mod fight;
pub mod front;
pub mod glb;
pub mod model;
pub mod moves;
pub mod objects;
pub mod pack;
pub mod sound;
pub mod stage;
pub mod tex;

use anyhow::{Context, Result};

pub(crate) fn u16le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(*b.get(o..)?.first_chunk()?))
}

pub(crate) fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(*b.get(o..)?.first_chunk()?))
}

pub(crate) fn f32le(b: &[u8], o: usize) -> Option<f32> {
    u32le(b, o).map(f32::from_bits)
}

/// `n` bytes at `o`; `what` names them in the error.
pub(crate) fn bytes<'a>(b: &'a [u8], o: usize, n: usize, what: &str) -> Result<&'a [u8]> {
    o.checked_add(n)
        .and_then(|end| b.get(o..end))
        .with_context(|| format!("{what}: {n} bytes at {o:#x} are outside the data ({:#x} bytes)", b.len()))
}

pub(crate) fn u8_at(b: &[u8], o: usize, what: &str) -> Result<u8> {
    Ok(bytes(b, o, 1, what)?.first().copied().unwrap_or_default())
}

pub(crate) fn u16_at(b: &[u8], o: usize, what: &str) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes(b, o, 2, what)?.try_into()?))
}

pub(crate) fn u32_at(b: &[u8], o: usize, what: &str) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(b, o, 4, what)?.try_into()?))
}

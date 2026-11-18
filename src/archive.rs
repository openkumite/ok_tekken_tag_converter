//! TEKKEN.BIN: the file table (in the executable), first-sector scrambling, and nested packs.
use crate::{u32_at, u32le};
use anyhow::{Context, Result, bail, ensure};
use ok_converter_common::disc::Iso;
use ok_converter_common::io::DiscFile;

const ELF: &str = "SLUS_200.01";
const ELF_SIZE: u32 = 2873728;
const BIN: &str = "TEKKEN.BIN";
const TOC: usize = 0x1cae40; // file offset in SLUS_200.01 (vaddr 0x509e40)
pub const FILES: usize = 748;
/// What the table adds to a file's sector (counted from TEKKEN.BIN's start).
const SECTOR_BIAS: u32 = 0x4820;
/// No file is larger (the longest movie is 74 MB): a broken table cannot ask for more memory.
const MAX_SIZE: u32 = 256 << 20;

#[derive(Debug, PartialEq)]
pub struct Entry {
    /// Its first sector in TEKKEN.BIN.
    pub sector: u32,
    pub size: u32,
    /// Not scrambled: a stream (music, a movie).
    pub plain: bool,
}

/// Entry `i` of the file table: u32 sector + 0x4820, u32 size + 0x350a (size -1 marks an absent file), u32 check,
/// u32 0xffff on a stream.
pub fn entry(elf: &[u8], i: usize) -> Result<Entry> {
    let what = "the file table";
    let e = TOC + 16 * i;
    let (sector, a, b, c) =
        (u32_at(elf, e, what)?, u32_at(elf, e + 4, what)?, u32_at(elf, e + 8, what)?, u32_at(elf, e + 12, what)?);
    let size = (a as i32).saturating_sub(0x350a).max(0) as u32;
    // the third word repeats index and sector count: validates the table position
    ensure!(
        (b >> 18) as usize == i && (b & 0x3ffff).wrapping_sub(0x11f3c) == size.div_ceil(2048),
        "file table entry {i} is not Tekken Tag Tournament USA v2.00's"
    );
    ensure!(size <= MAX_SIZE, "file table entry {i}: {size} bytes");
    let sector = sector.checked_sub(SECTOR_BIAS).with_context(|| format!("file table entry {i}: sector {sector}"))?;
    Ok(Entry { sector, size, plain: c == 0xffff || i == 0x2c })
}

/// Unscrambles the first sector of file `i`: xor with a 64-bit LCG seeded by the index.
pub fn unscramble(d: &mut [u8], i: usize) {
    let i = i as u64;
    let mut k = 218319u64.wrapping_mul(i + 0xf3c).wrapping_add(i.wrapping_sub(0x348f).wrapping_mul(0x1234567));
    for w in d.as_chunks_mut::<4>().0.iter_mut().take(0x200) {
        *w = (u32::from_le_bytes(*w) ^ k as u32).to_le_bytes();
        k = k.wrapping_mul(0x71c3ab9).wrapping_add(0x3941);
    }
}

/// The disc, opened: its executable, the file table, and TEKKEN.BIN's files read one at a time.
pub struct Archive {
    iso: Iso,
    /// TEKKEN.BIN's first sector and its size in sectors.
    bin: (u32, u32),
    pub elf: Vec<u8>,
    pub toc: Vec<Entry>,
}

impl Archive {
    /// `src`: the disc's data track.
    pub fn open(src: &dyn DiscFile) -> Result<Archive> {
        let wrong = "not Tekken Tag Tournament USA v2.00 (SLUS-20001)";
        let mut iso = Iso::open(src)?;
        let (lba, size) = iso.find(ELF).with_context(|| format!("{}: {wrong}: no {ELF} on the disc", src.name()))?;
        if size != ELF_SIZE {
            bail!("{}: {wrong}: another version's {ELF} ({size} bytes)", src.name());
        }
        let elf = iso.read_file(lba, size)?;
        let (lba, size) = iso.find(BIN).with_context(|| format!("{}: {wrong}: no {BIN} on the disc", src.name()))?;
        if !iso.sector(lba as u64)?.starts_with(b"NAMCO TEKKEN TAG TOURNAMENT") {
            bail!("{}: {wrong}: {BIN} is another game's, or the image is damaged", src.name());
        }
        let sectors = size.div_ceil(2048);
        let mut toc = Vec::with_capacity(FILES);
        for i in 0..FILES {
            let e = entry(&elf, i).with_context(|| format!("{}: {wrong}", src.name()))?;
            ensure!(e.sector as u64 + e.size.div_ceil(2048) as u64 <= sectors as u64, "file {i} is outside {BIN}");
            toc.push(e);
        }
        Ok(Archive { iso, bin: (lba, sectors), elf, toc })
    }

    /// File `i`, unscrambled.
    pub fn read(&mut self, i: usize) -> Result<Vec<u8>> {
        let e = self.toc.get(i).with_context(|| format!("no file {i} in {BIN}"))?;
        let mut d = self
            .iso
            .read_file(self.bin.0.saturating_add(e.sector), e.size)
            .with_context(|| format!("file {i} of {BIN}"))?;
        ensure!(d.len() == e.size as usize, "file {i} of {BIN} is cut short: the image is incomplete");
        if !e.plain {
            unscramble(&mut d, i);
        }
        Ok(d)
    }
}

/// A pack: u32 count, u32 first offset, then (size, next offset) pairs. None when `d` is not one.
pub fn pack(d: &[u8]) -> Option<Vec<&[u8]>> {
    let (n, mut o) = (u32le(d, 0)? as usize, u32le(d, 4)? as usize);
    if d.len() < 16 || n == 0 || n > 64 || o != (8 + 8 * n).next_multiple_of(16) {
        return None;
    }
    let mut out = Vec::new();
    for k in 0..n {
        let (size, next) = (u32le(d, 8 + 8 * k)? as usize, u32le(d, 12 + 8 * k)?);
        let end = o.checked_add(size)?;
        out.push(d.get(o..end)?);
        if k + 1 == n {
            if next != 0xffff_ffff {
                return None;
            }
        } else if (next as usize) < end {
            return None;
        } else {
            o = next as usize;
        }
    }
    Some(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn pack_splits_and_rejects() {
        // two parts: 4 bytes at 0x20, 2 bytes at 0x30
        let mut d = vec![0u8; 0x32];
        for (o, v) in [(0, 2u32), (4, 0x20), (8, 4), (12, 0x30), (16, 2), (20, 0xffff_ffff)] {
            d[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        d[0x20..0x24].copy_from_slice(b"abcd");
        d[0x30..0x32].copy_from_slice(b"xy");
        assert_eq!(pack(&d), Some(vec![&b"abcd"[..], &b"xy"[..]]));
        // cut anywhere, it is not a pack (and not a panic)
        for n in 0..d.len() {
            assert_eq!(pack(&d[..n]), None, "{n}");
        }
        d[20] = 0; // no end marker
        assert_eq!(pack(&d), None);
        d[20] = 0xff;
        d[12..16].copy_from_slice(&0x22u32.to_le_bytes()); // the second part starts inside the first
        assert_eq!(pack(&d), None);
        d[8..12].copy_from_slice(&u32::MAX.to_le_bytes()); // a size past the end
        assert_eq!(pack(&d), None);
        assert_eq!(pack(b"VAGp"), None);
    }

    /// A table with entry `i` made of `words`.
    fn table(i: usize, words: [u32; 4]) -> Vec<u8> {
        let mut elf = vec![0u8; TOC + 16 * (i + 1)];
        for (k, w) in words.iter().enumerate() {
            elf[TOC + 16 * i + 4 * k..][..4].copy_from_slice(&w.to_le_bytes());
        }
        elf
    }

    #[test]
    fn entry_decodes_and_rejects() {
        // file 5: 3 sectors in, 5000 bytes (3 sectors), scrambled
        let check = 5 << 18 | (0x11f3c + 3);
        let good = [SECTOR_BIAS + 3, 5000 + 0x350a, check, 0];
        assert_eq!(entry(&table(5, good), 5).unwrap(), Entry { sector: 3, size: 5000, plain: false });
        // a stream; file 0x2c is one without the mark
        assert!(entry(&table(5, [good[0], good[1], check, 0xffff]), 5).unwrap().plain);
        assert!(entry(&table(0x2c, [SECTOR_BIAS, 0x350a, 0x2c << 18 | 0x11f3c, 0]), 0x2c).unwrap().plain);
        // an absent file: size -1
        let absent = entry(&table(5, [SECTOR_BIAS, 0x3509, 5 << 18 | 0x11f3c, 0]), 5).unwrap();
        assert_eq!(absent.size, 0);
        // broken: the wrong index, the wrong sector count, a sector before TEKKEN.BIN's start, too large, a cut table, zeros
        assert!(entry(&table(5, [good[0], good[1], 6 << 18 | (0x11f3c + 3), 0]), 5).is_err());
        assert!(entry(&table(5, [good[0], good[1], check + 1, 0]), 5).is_err());
        assert!(entry(&table(5, [SECTOR_BIAS - 1, good[1], check, 0]), 5).is_err());
        let huge = MAX_SIZE + 1;
        assert!(entry(&table(5, [good[0], huge + 0x350a, 5 << 18 | (0x11f3c + huge.div_ceil(2048)), 0]), 5).is_err());
        assert!(entry(&table(5, good)[..TOC + 16 * 5 + 10], 5).is_err());
        assert!(entry(&vec![0; TOC + 16 * FILES], 5).is_err());
        assert!(entry(&[], 0).is_err());
    }

    #[test]
    fn unscramble_is_its_own_inverse_over_the_first_sector() {
        let plain: Vec<u8> = (0..3000u32).map(|v| (v * 7) as u8).collect();
        let mut d = plain.clone();
        unscramble(&mut d, 345);
        assert_ne!(d[..2048], plain[..2048]);
        assert_eq!(d[2048..], plain[2048..]); // the first sector only
        // the key stream's first word for file 345: 218319 * (345 + 0xf3c) + (345 - 0x348f) * 0x1234567, low 32 bits
        let k = 218319u64.wrapping_mul(345 + 0xf3c).wrapping_add(345u64.wrapping_sub(0x348f).wrapping_mul(0x1234567));
        assert_eq!(
            u32::from_le_bytes(d[..4].try_into().unwrap()),
            u32::from_le_bytes(plain[..4].try_into().unwrap()) ^ k as u32
        );
        unscramble(&mut d, 345);
        assert_eq!(d, plain);
        // another file's key does not undo it; a file shorter than a word is left alone
        unscramble(&mut d, 345);
        unscramble(&mut d, 346);
        assert_ne!(d, plain);
        let mut short = vec![1, 2, 3];
        unscramble(&mut short, 1);
        assert_eq!(short, [1, 2, 3]);
    }
}

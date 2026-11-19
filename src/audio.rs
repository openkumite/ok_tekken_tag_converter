//! PS-ADPCM (VAG banks, interleaved stereo streams), PSS movie demux, WAV out.
use crate::u32le;

/// The predictor's two coefficients, by filter.
const FILTERS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];

/// Decodes 16-byte PS-ADPCM frames (28 samples each); stops at an end-marker frame (flag 7):
/// what follows one in a movie's sound is filler (checked against the pictures' length).
pub fn adpcm(d: &[u8]) -> Vec<i16> {
    let (mut s1, mut s2) = (0i32, 0i32);
    let mut out = Vec::with_capacity(d.len() / 16 * 28);
    for f in d.as_chunks::<16>().0 {
        if f[1] == 7 {
            break;
        }
        let shift = (f[0] & 15).min(12) as u32;
        let (f0, f1) = FILTERS.get((f[0] >> 4) as usize).copied().unwrap_or((122, -60));
        for nib in f[2..].iter().flat_map(|b| [b & 15, b >> 4]) {
            let v = (((nib as i32) << 28 >> 28) << 12 >> shift) + ((s1 * f0 + s2 * f1 + 32) >> 6);
            let v = v.clamp(-32768, 32767);
            out.push(v as i16);
            s2 = s1;
            s1 = v;
        }
    }
    out
}

pub fn wav(rate: u32, channels: u16, pcm: &[i16]) -> Vec<u8> {
    let n = pcm.len() as u32 * 2;
    let mut w = Vec::with_capacity(44 + n as usize);
    w.extend(b"RIFF");
    w.extend((36 + n).to_le_bytes());
    w.extend(b"WAVEfmt ");
    w.extend(16u32.to_le_bytes());
    w.extend(1u16.to_le_bytes());
    w.extend(channels.to_le_bytes());
    w.extend(rate.to_le_bytes());
    w.extend(rate.wrapping_mul(channels as u32 * 2).to_le_bytes()); // (a bank's rate is the disc's word)
    w.extend((channels * 2).to_le_bytes());
    w.extend(16u16.to_le_bytes());
    w.extend(b"data");
    w.extend(n.to_le_bytes());
    for s in pcm {
        w.extend(s.to_le_bytes());
    }
    w
}

/// A sound bank: "VAGp" files back to back. Returns (name, rate, samples, the sample its loop starts at: the SPU
/// goes back there from the frame flagged loop end, flag bit 4 marks the start).
pub fn vag_bank(d: &[u8]) -> Vec<(String, u32, Vec<i16>, Option<usize>)> {
    let (mut o, mut out) = (0, Vec::new());
    while let Some((head, body)) = d.get(o..).and_then(|d| d.split_at_checked(0x30)) {
        if !head.starts_with(b"VAGp") {
            o += 16;
            continue;
        }
        let be = |o: usize| head.get(o..).and_then(|b| b.first_chunk()).map_or(0, |b| u32::from_be_bytes(*b));
        let (size, rate) = ((be(12) as usize).min(body.len()), be(16));
        let name = head.get(0x20..).unwrap_or_default();
        let name: String = name.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
        let body = body.get(..size).unwrap_or(body);
        let start = body.as_chunks::<16>().0.iter().position(|f| f[1] & 4 != 0).map(|k| k * 28);
        out.push((name, rate, adpcm(body), start));
        o += 0x30 + size;
    }
    out
}

/// A movie's sound sectors and packs: 7 sound sectors, then 16 MPEG-2 program stream packs of 16 KiB, over and over.
const MOVIE_SOUND: usize = 7;
const MOVIE_PERIOD: usize = MOVIE_SOUND + 16 * 8;
const PACK: u32 = 0xba01_0000;

/// A stream file: 2048-byte audio sectors (left 1024 bytes, right 1024 bytes), and in movies 16 KiB MPEG-2 program
/// stream packs between them. A movie's layout is taken by place, not by content: once its sound has ended (movie
/// 15's 3 s early) its sound sectors hold leftovers, some a copy of a pack. Returns (interleaved stereo pcm, pss).
pub fn stream(d: &[u8]) -> (Vec<i16>, Vec<u8>) {
    let (mut l, mut r, mut pss) = (Vec::new(), Vec::new(), Vec::new());
    let movie = u32le(d, MOVIE_SOUND * 2048) == Some(PACK);
    for (k, sector) in d.chunks(2048).enumerate() {
        let video = movie && k % MOVIE_PERIOD >= MOVIE_SOUND;
        if video && (k % MOVIE_PERIOD - MOVIE_SOUND).is_multiple_of(8) && u32le(sector, 0) == Some(PACK) {
            // (the last pack may be cut short)
            pss.extend(d.get(k * 2048..).and_then(|p| p.get(..p.len().min(0x4000))).unwrap_or_default());
        } else if let (false, Some((left, right))) = (video, sector.split_at_checked(1024))
            && right.len() == 1024
        {
            l.extend(left);
            r.extend(right);
        }
    }
    let (l, r) = (adpcm(&l), adpcm(&r));
    (l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect(), pss)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn adpcm_decodes_frames_up_to_the_end_marker() {
        // shift 12, filter 0: a nibble is its own value; low nibble first
        let mut d = vec![12, 0, 0x21, 0xf7];
        d.resize(16, 0);
        d.extend([0, 7]); // an end marker, and whatever follows it
        d.resize(48, 0x11);
        let pcm = adpcm(&d);
        assert_eq!(pcm.len(), 28);
        assert_eq!(pcm[..5], [1, 2, 7, -1, 0]);
        // a cut frame is left out; any header byte decodes
        assert!(adpcm(&d[..15]).is_empty());
        assert_eq!(adpcm(&[0xff; 32]).len(), 56);
    }

    #[test]
    fn vag_bank_and_stream_split_what_is_there() {
        let mut vag = b"VAGp".to_vec();
        vag.resize(0x30, 0);
        vag[12..16].copy_from_slice(&32u32.to_be_bytes());
        vag[16..20].copy_from_slice(&22050u32.to_be_bytes());
        vag[0x20..0x24].copy_from_slice(b"kick");
        vag.resize(0x30 + 32, 0);
        vag[0x30 + 17] = 6; // the second frame starts the loop
        let mut bank = vag.clone();
        bank.extend(&vag);
        let sounds = vag_bank(&bank);
        assert_eq!(sounds.len(), 2);
        assert_eq!((sounds[1].0.as_str(), sounds[1].1, sounds[1].2.len(), sounds[1].3), ("kick", 22050, 56, Some(28)));
        // a size past the end takes what there is; a cut header is no sound
        vag[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(vag_bank(&vag)[0].2.len(), 56);
        assert!(vag_bank(&vag[..0x2f]).is_empty());
        assert!(vag_bank(&[]).is_empty());

        // a stream: audio sectors; a movie: its 7 sound sectors, packs (the last cut short) and, where its sound has
        // ended, a sound sector that copies a pack, taken as sound; nothing of a cut sector
        let mut s = vec![0u8; 7 * 2048];
        let pack = |s: &mut Vec<u8>, n: usize| s.extend(0xba01_0000u32.to_le_bytes().iter().chain(&vec![1; n - 4]));
        pack(&mut s, 0x4000);
        s.resize(135 * 2048, 0);
        pack(&mut s, 0x4000);
        s.resize(150 * 2048, 0);
        pack(&mut s, 3000);
        let (pcm, pss) = stream(&s);
        assert_eq!((pcm.len(), pss.len()), (14 * 1024 / 16 * 28 * 2, 0x4000 + 3000));
        assert_eq!(stream(&s[..2047]), (vec![], vec![]));
        assert_eq!(stream(&s[..7 * 2048]).0.len(), 7 * 1024 / 16 * 28 * 2);
        let wav = wav(44100, 2, &pcm);
        assert_eq!(
            (wav.len(), &wav[..4], &wav[40..44]),
            (44 + pcm.len() * 2, &b"RIFF"[..], &(pcm.len() as u32 * 2).to_le_bytes()[..])
        );
    }
}

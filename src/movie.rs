//! The movies (files 14-16): their MPEG-2 program stream (`audio::stream` takes it out of the file) decoded, each
//! picture a JPEG, as the Motion-JPEG AVI OpenKumite plays (the Tekken 3 converter's movie format), at the movie's
//! own size and rate (640 x 448, 30 a second).

use anyhow::{Result, bail};
use ok_converter_common::psx;

/// The JPEGs' quality (the Tekken 3 converter's movies').
const QUALITY: u8 = 85;

/// The video elementary stream (stream 0xE0) of a program stream: each PES packet's payload in order.
fn video(pss: &[u8]) -> Vec<u8> {
    let (mut o, mut es) = (0, Vec::new());
    while let Some(&[a, b, c, id, l0, l1]) = pss.get(o..o + 6) {
        if [a, b, c] != [0, 0, 1] {
            // not a start code: on to the next pack (a cut or damaged one)
            match pss.get(o + 1..).and_then(|r| r.windows(4).position(|w| w == [0, 0, 1, 0xba])) {
                Some(k) => o += 1 + k,
                None => break,
            }
            continue;
        }
        let len = usize::from(u16::from_be_bytes([l0, l1]));
        o += match id {
            // a pack header: 14 bytes and its stuffing
            0xba => 14 + usize::from(pss.get(o + 13).map_or(0, |b| b & 7)),
            0xb9 => 4,
            _ => {
                if id == 0xe0 {
                    // the MPEG-2 PES header's own length at +8
                    let start = o + 9 + usize::from(pss.get(o + 8).copied().unwrap_or(0));
                    es.extend(pss.get(start..(o + 6 + len).min(pss.len())).unwrap_or_default());
                }
                6 + len
            }
        };
    }
    es
}

/// BT.601 limited-range YCbCr to RGB, as the PS2's IPU converts it, chroma 4:2:0 (each sample over 2 x 2 pixels).
fn rgb(p: &filmcraft_mpeg2v::Picture) -> Vec<u8> {
    let (w, h, cw) = (p.width as usize, p.height as usize, p.chroma_width() as usize);
    let mut out = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let c = y / 2 * cw + x / 2;
            let (l, cb, cr) = (
                298 * (i32::from(p.y.get(y * w + x).copied().unwrap_or(16)) - 16),
                i32::from(p.cb.get(c).copied().unwrap_or(128)) - 128,
                i32::from(p.cr.get(c).copied().unwrap_or(128)) - 128,
            );
            for v in [l + 409 * cr, l - 100 * cb - 208 * cr, l + 516 * cb] {
                out.push(((v + 128) >> 8).clamp(0, 255) as u8);
            }
        }
    }
    out
}

/// A movie's pictures (its program stream `pss`) as a Motion-JPEG AVI: the AVI, its pictures, its frame rate and the
/// decoder's errors (a damaged picture is concealed: 0 on a good copy of the disc).
pub fn avi(pss: &[u8]) -> Result<(Vec<u8>, usize, u32, usize)> {
    let es = video(pss);
    let mut dec = filmcraft_mpeg2v::Decoder::new();
    let mut pictures = Vec::new();
    let (mut jpegs, mut size, mut fps) = (Vec::new(), (0, 0), 30);
    let mut encode = |pictures: &mut Vec<filmcraft_mpeg2v::Picture>| -> Result<()> {
        for p in pictures.drain(..) {
            if p.chroma != filmcraft_mpeg2v::ChromaFormat::Yuv420 {
                bail!("a movie picture is not 4:2:0");
            }
            size = (p.width, p.height);
            fps = p.info.frame_rate.map_or(30, |(n, d)| (n + d / 2) / d.max(1)).max(1);
            let mut j = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut j, QUALITY).encode(
                &rgb(&p),
                p.width,
                p.height,
                image::ExtendedColorType::Rgb8,
            )?;
            jpegs.push(j);
        }
        Ok(())
    };
    for (k, au) in filmcraft_mpeg2v::access_units(&es).into_iter().enumerate() {
        pictures.extend(dec.decode(es.get(au).unwrap_or_default(), k as i64).unwrap_or_default());
        encode(&mut pictures)?;
    }
    pictures.extend(dec.flush());
    encode(&mut pictures)?;
    if jpegs.is_empty() {
        bail!("the movie has no pictures");
    }
    Ok((psx::avi(&jpegs, size.0, size.1, fps), jpegs.len(), fps, dec.errors()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn the_video_stream_comes_out_of_its_packs() {
        // a pack header (no stuffing), a system header, two video PES with 3-byte headers, an audio one, a cut one
        let mut pss = vec![0, 0, 1, 0xba, 0x44, 0, 4, 0, 4, 1, 1, 0x89, 0xc3, 0xf8];
        pss.extend([0, 0, 1, 0xbb, 0, 2, 9, 9]);
        let pes = |id: u8, body: &[u8]| [&[0, 0, 1, id, 0, (body.len() + 3) as u8, 0x81, 0, 0][..], body].concat();
        pss.extend(pes(0xe0, b"abc"));
        pss.extend(pes(0xc0, b"xx"));
        pss.extend(pes(0xe0, b"de"));
        pss.extend([7, 7]); // junk: skipped to the next pack
        pss.extend([0, 0, 1, 0xba, 0x44, 0, 4, 0, 4, 1, 1, 0x89, 0xc3, 0xf8]);
        pss.extend(&pes(0xe0, b"fgh")[..10]);
        assert_eq!(video(&pss), b"abcdef");
        assert!(video(&[0, 0, 1]).is_empty());
        assert!(avi(&pss).is_err());
    }
}

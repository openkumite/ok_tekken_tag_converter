//! GS texture uploads (GIF packets with BITBLTBUF/TRXPOS/TRXREG + IMAGE data) and TEX0 lookups.
use crate::{u16le, u32le};

#[derive(Clone)]
pub struct Upload {
    /// which load it came from: later loads reuse addresses, so an image and its palette must share one
    pub set: u64,
    pub dbp: u32,
    /// buffer width in pixels
    pub dbw: usize,
    pub psm: u32,
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
}

fn u64le(d: &[u8], o: usize) -> Option<u64> {
    Some(u32le(d, o)? as u64 | (u32le(d, o + 4)? as u64) << 32)
}

/// Every image transfer in a GIF stream; stops at the first thing that is not one.
pub fn uploads(d: &[u8]) -> Vec<Upload> {
    let (mut o, mut out, mut reg) = (0, Vec::new(), [0u64; 4]);
    let (mut set, mut last) = (0, 0); // an indexed image at or below the previous one's address starts a new load
    while let (Some(lo), Some(hi)) = (u64le(d, o), u64le(d, o + 8)) {
        o += 16;
        let (nloop, flg, nreg) = ((lo & 0x7fff) as usize, (lo >> 58) & 3, ((lo >> 60) & 15) as usize);
        if lo == 0 || nloop == 0 {
            break;
        }
        if flg == 0 {
            for i in 0..nloop * nreg {
                let (Some(value), Some(addr)) = (u64le(d, o), u64le(d, o + 8)) else { return out };
                if (hi >> (4 * (i % nreg))) & 15 == 0xe
                    && let Some(r) = reg.get_mut(((addr & 0xff) as usize).wrapping_sub(0x50))
                {
                    *r = value;
                }
                o += 16;
            }
        } else if flg == 2 {
            let data = d.get(o..).unwrap_or_default();
            let data = data.get(..nloop * 16).unwrap_or(data);
            let (dbp, psm) = ((reg[0] >> 32) as u32 & 0x3fff, (reg[0] >> 56) as u32 & 63);
            if psm == 0x13 || psm == 0x14 {
                if dbp <= last && out.iter().any(|u: &Upload| u.psm == 0x13 || u.psm == 0x14) {
                    set += 1;
                }
                last = dbp;
            }
            out.push(Upload {
                dbw: (reg[0] >> 48 & 63) as usize * 64,
                set,
                dbp,
                psm,
                w: (reg[2] & 0xfff) as usize,
                h: (reg[2] >> 32 & 0xfff) as usize,
                data: data.to_vec(),
            });
            o += data.len();
        } else {
            break;
        }
    }
    out
}

/// Texture channel `k` showing its frame `f` (0x3A82C8 sets a fighter's channels up, 0x3A80E8 uploads the frame shown):
/// `model` is the fighter's model part, whose channel table follows its nodes (a u32 count, then at +0x14 16 bytes
/// `{s16 channel, u16 dbp, x, y, w, h, dbw, psm}` each: where the frame goes, from the fighter's first texture block);
/// `frames` its pack part 1 (channel k's frame f at [0][k][f], an image and its palette). Only Jin has one: his
/// face with and without the mark (`k` 0, frames 0 and 1).
pub fn channel_frame(ups: &mut [Upload], model: &[u8], frames: &[u8], k: usize, f: usize) -> Option<()> {
    let t = u32le(model, 0x14)? as usize;
    let e = (0..u32le(model, t.checked_sub(4)?)?.min(8) as usize)
        .map(|e| t + 16 * e)
        .find(|&o| u16le(model, o) == Some(k as u16))?;
    let [dbp, x, y, w, h, dbw, psm] = [2, 4, 6, 8, 10, 12, 14].map(|i| u16le(model, e + i).unwrap_or(0) as usize);
    fn pack(d: &[u8], i: usize) -> Option<&[u8]> {
        crate::archive::pack(d)?.get(i).copied()
    }
    let src =
        uploads(pack(pack(pack(frames, 0)?, k)?, f)?).into_iter().find(|u| u.psm == 0x13 && u.w >= w && u.h >= h)?;
    // shortcut: PSMT8 only (Jin's), in a sheet its own width from a page boundary; others would need the GS's swizzle
    let at = |u: &&mut Upload| {
        u.psm == 0x13
            && psm == 0x13
            && u.dbw == dbw * 64
            && u.dbp as usize <= dbp
            && (dbp - u.dbp as usize).is_multiple_of(32)
    };
    let dst = ups.iter_mut().find(at)?;
    let p = (dbp - dst.dbp as usize) / 32; // pages of 128 x 64 texels, across the buffer then down
    let across = (dst.dbw / 128).max(1);
    let (px, py) = (p % across * 128 + x, p / across * 64 + y);
    for r in 0..h {
        let row = src.data.get(r * src.w..r * src.w + w)?;
        dst.data.get_mut((py + r) * dst.w + px..(py + r) * dst.w + px + w)?.copy_from_slice(row);
    }
    Some(())
}

pub struct Tex0 {
    pub tbp: u32,
    /// buffer width in pixels
    pub tbw: usize,
    pub psm: u32,
    pub tw: usize,
    pub th: usize,
    pub cbp: u32,
    /// TCC: the texture's alpha is used
    pub alpha: bool,
}
pub fn tex0(t: u64) -> Tex0 {
    Tex0 {
        tbp: t as u32 & 0x3fff,
        tbw: (t >> 14 & 63) as usize * 64,
        psm: (t >> 20) as u32 & 63,
        tw: 1 << (t >> 26 & 15),
        th: 1 << (t >> 30 & 15),
        cbp: (t >> 37) as u32 & 0x3fff,
        alpha: t >> 34 & 1 != 0,
    }
}

/// Colour `i` of a palette (or of a direct-colour image): PSMCT32 (alpha 0x80 = opaque) or PSMCT16.
fn colour(c: &Upload, i: usize) -> [u8; 4] {
    if c.psm == 2 {
        return u16le(&c.data, i * 2).map_or([0; 4], rgba16);
    }
    let rgba = c.data.get(i * 4..).and_then(|b| b.first_chunk::<4>());
    rgba.map_or([0; 4], |&[r, g, b, a]| [r, g, b, (a as u32 * 2).min(255) as u8])
}

/// The RGBA image a TEX0 selects from the uploads: (width, height, pixels). Indexed formats only
/// (PSMT8 0x13, PSMT4 0x14), textures starting on a page boundary of an upload.
pub fn image(ups: &[Upload], t: &Tex0, alpha: bool) -> Option<(usize, usize, Vec<u8>)> {
    // (the GS draws no texture larger: a register of a broken file cannot ask for more)
    if t.tw > 1024 || t.th > 1024 {
        return None;
    }
    // The base pointer counts blocks (16x16 pixels in PSMT8, 32x16 in PSMT4), 32 to a page (128x64,
    // 128x128). Pages follow each other across the buffer width, then down: a texture narrower than
    // the sheet it was uploaded in has its pages spread over the sheet's rows.
    let four = t.psm == 0x14;
    let (pw, ph) = (128, if four { 128 } else { 64 });
    let pages = |u: &Upload| (u.dbw / pw, u.h.div_ceil(ph)); // across, down
    let blocks = |u: &Upload| t.tbp.saturating_sub(u.dbp) as usize;
    let first = |u: &Upload| blocks(u) / 32;
    let paged = |u: &Upload| pages(u).0 >= 2 && blocks(u).is_multiple_of(32);
    // shortcut: in a sheet one page wide the blocks are taken as rows, true only from a page boundary
    let row = |u: &Upload| (blocks(u) / (u.w / if four { 32 } else { 16 }).max(1)) * 16;
    let holds = |u: &&Upload| {
        u.psm == t.psm
            && u.dbp <= t.tbp
            && u.w >= 16
            && if paged(u) { first(u) < pages(u).0 * pages(u).1 } else { row(u) < u.h }
    };
    let is_clut = |u: &&Upload| u.dbp == t.cbp && (u.psm == 0 || u.psm == 2);
    let clut = ups
        .iter()
        .filter(is_clut)
        .find(|c| ups.iter().any(|u| u.set == c.set && holds(&u)))
        .or_else(|| ups.iter().find(is_clut))?;
    let src = ups.iter().filter(holds).max_by_key(|u| (u.set == clut.set, u.dbp))?;
    let across = (t.tbw / pw).max(1); // of the texture
    let (w, h) = if paged(src) {
        (
            t.tw.min(across * pw),
            t.th.min((pages(src).0 * pages(src).1).saturating_sub(first(src)).div_ceil(across) * ph),
        )
    } else {
        (t.tw.min(src.w), t.th.min(src.h.saturating_sub(row(src))))
    };
    let mut px = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let i = if paged(src) {
                let p = first(src) + y / ph * across + x / pw;
                let (sx, sy) = (p % pages(src).0 * pw + x % pw, p / pages(src).0 * ph + y % ph);
                if sx < src.w { sy * src.w + sx } else { usize::MAX / 2 }
            } else {
                (row(src) + y) * src.w + x
            };
            let mut idx = if four {
                (src.data.get(i / 2).copied().unwrap_or(0) >> (4 * (i & 1))) as usize & 15
            } else {
                src.data.get(i).copied().unwrap_or(0) as usize
            };
            if !four {
                idx = (idx & 0xe7) | (idx & 8) << 1 | (idx & 16) >> 1; // CSM1 palette order
            }
            let [r, g, b, a] = colour(clut, idx);
            px.extend([r, g, b, if alpha { a } else { 255 }]);
        }
    }
    Some((w, h, px))
}

/// The texels `x` x `y` of a `w` x `h` RGBA picture, which repeats past its edges (as the GS repeats a texture).
pub fn cut(w: usize, h: usize, px: &[u8], x: std::ops::Range<usize>, y: std::ops::Range<usize>) -> Vec<u8> {
    let mut out = Vec::with_capacity(x.len() * y.len() * 4);
    for y in y {
        let row = px.get(y % h.max(1) * w * 4..).unwrap_or_default();
        for x in x.clone() {
            out.extend(row.get(x % w.max(1) * 4..).and_then(|p| p.first_chunk::<4>()).unwrap_or(&[0; 4]));
        }
    }
    out
}

/// Pictures packed into one: its size and pixels, and where each picture's top left is (in the order given).
pub struct Atlas {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
    pub place: Vec<(usize, usize)>,
}

/// RGBA pictures (width, height, pixels) in an atlas at least 1024 wide: shelves, tallest first, each picture with
/// a border of its own edge texels (filtering at its edge takes no neighbour's colour); the height a power of two.
/// None for a picture of no size or fewer pixels than its size, or more than 8192 rows.
pub fn atlas(items: &[(usize, usize, Vec<u8>)]) -> Option<Atlas> {
    const PAD: usize = 1;
    let width = items.iter().map(|p| p.0 + 2 * PAD).max().unwrap_or(1).next_power_of_two().max(1024);
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(items.get(i).map_or(0, |p| p.1)));
    let (mut x, mut y, mut shelf) = (0, 0, 0);
    let mut place = vec![(0, 0); items.len()];
    for i in order {
        let &(w, h, _) = items.get(i)?;
        if w == 0 || h == 0 {
            return None;
        }
        if x + w + 2 * PAD > width {
            (x, y, shelf) = (0, y + shelf, 0);
        }
        *place.get_mut(i)? = (x + PAD, y + PAD);
        x += w + 2 * PAD;
        shelf = shelf.max(h + 2 * PAD);
        if y + shelf > 8192 {
            return None;
        }
    }
    let h = (y + shelf).next_power_of_two();
    let mut rgba = vec![0; width * h * 4];
    for ((w, ph, px), &(x, y)) in items.iter().zip(&place) {
        let rows: Vec<&[u8]> = px.chunks_exact(w * 4).take(*ph).collect();
        for r in 0..ph + 2 * PAD {
            let src = rows.get(r.saturating_sub(PAD).min(ph - 1))?;
            let (first, last) = (src.first_chunk::<4>()?, src.last_chunk::<4>()?);
            let at = ((y - PAD + r) * width + x - PAD) * 4;
            let line = rgba.get_mut(at..at + (w + 2 * PAD) * 4)?;
            for (d, s) in line.iter_mut().zip(first.iter().chain(*src).chain(last)) {
                *d = *s;
            }
        }
    }
    Some(Atlas { w: width, h, rgba, place })
}

pub fn png(w: usize, h: usize, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, w as u32, h as u32);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    // writing to a Vec cannot fail
    if let Ok(mut wr) = e.write_header() {
        let _ = wr.write_image_data(rgba);
    }
    out
}

fn rgba16(v: u16) -> [u8; 4] {
    let c = |s: u16| ((v >> s & 31) * 255 / 31) as u8;
    [c(0), c(5), c(10), if v & 0x8000 != 0 || v & 0x7fff != 0 { 255 } else { 0 }]
}

/// A palette-sized upload (16 or 256 colours, at most 16 rows): not a picture where a stream has indexed images.
pub fn is_palette(u: &Upload) -> bool {
    (u.psm == 0 || u.psm == 2) && matches!(u.w * u.h, 16 | 256) && u.h <= 16
}

/// Every upload as a picture: (width, height, RGBA). An indexed image takes the palette upload `palette` names for
/// its upload number, else the nearest palette-sized upload after it (else before it); without one it comes out in
/// greys.
// shortcut: the pairing is a guess where one stream holds several palettes for one image and `palette` does not
// name it; read the TEX0 registers of whatever draws the image (as models do) to get the exact pairs.
pub fn pictures(ups: &[Upload], palette: &dyn Fn(usize) -> Option<usize>) -> Vec<(usize, usize, Vec<u8>)> {
    let entries = |u: &Upload| if u.psm == 0 || u.psm == 2 { u.w * u.h } else { 0 };
    let is_clut = is_palette;
    let has_indexed = ups.iter().any(|u| u.psm == 0x13 || u.psm == 0x14);
    let mut out = Vec::new();
    for (k, u) in ups.iter().enumerate() {
        if u.w == 0 || u.h == 0 || u.w > 2048 || u.h > 2048 || (has_indexed && is_clut(u)) {
            continue;
        }
        let n = u.w * u.h;
        let px: Vec<u8> = match u.psm {
            0 if u.data.len() >= n * 4 => (0..n).flat_map(|i| colour(u, i)).collect(),
            1 if u.data.len() >= n * 3 => {
                u.data.as_chunks::<3>().0.iter().take(n).flat_map(|&[r, g, b]| [r, g, b, 255]).collect()
            }
            2 if u.data.len() >= n * 2 => (0..n).flat_map(|i| colour(u, i)).collect(),
            0x13 | 0x14 if u.data.len() >= if u.psm == 0x13 { n } else { n / 2 } => {
                let want = if u.psm == 0x13 { 256 } else { 16 };
                let (before, after) = ups.split_at(k);
                let fits = |c: &&Upload| is_clut(c) && entries(c) >= want;
                let named = palette(k).and_then(|p| ups.get(p)).filter(|c| fits(c));
                let clut = named.or_else(|| after.iter().find(fits)).or_else(|| before.iter().rev().find(fits));
                let byte = |i: usize| u.data.get(i).copied().unwrap_or(0);
                (0..n)
                    .flat_map(|i| {
                        let mut idx =
                            if u.psm == 0x13 { byte(i) as usize } else { (byte(i / 2) >> (4 * (i & 1))) as usize & 15 };
                        match clut {
                            Some(c) => {
                                if u.psm == 0x13 {
                                    idx = (idx & 0xe7) | (idx & 8) << 1 | (idx & 16) >> 1;
                                }
                                colour(c, idx)
                            }
                            None => {
                                let g = if u.psm == 0x13 { idx as u8 } else { idx as u8 * 17 };
                                [g, g, g, 255]
                            }
                        }
                    })
                    .collect()
            }
            _ => continue,
        };
        out.push((u.w, u.h, px));
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A GIF stream that uploads `data` as a `w` x `h` image of format `psm` at block `dbp`.
    fn gif(dbp: u64, psm: u64, w: u64, h: u64, data: &[u8]) -> Vec<u8> {
        let q = |lo: u64, hi: u64| [lo.to_le_bytes(), hi.to_le_bytes()].concat();
        // PACKED, 1 register (A+D), 3 loops: BITBLTBUF, TRXPOS, TRXREG; then IMAGE
        let mut d = q(3 | 1 << 60, 0xe);
        d.extend(q(dbp << 32 | 2 << 48 | psm << 56, 0x50));
        d.extend(q(0, 0x51));
        d.extend(q(w | h << 32, 0x52));
        d.extend(q(data.len().div_ceil(16) as u64 | 2 << 58, 0));
        d.extend(data);
        d.resize(d.len().next_multiple_of(16), 0);
        d
    }

    #[test]
    fn uploads_image_and_pictures() {
        // a 16 x 16 PSMT4 image of colour 1, then its 16-colour palette (8 x 2, PSMCT32) whose colour 1 is red
        let mut d = gif(0x100, 0x14, 16, 16, &[0x11; 128]);
        let mut clut = vec![0u8; 64];
        clut[4..8].copy_from_slice(&[255, 0, 0, 0x80]);
        d.extend(gif(0x200, 0, 8, 2, &clut));
        let ups = uploads(&d);
        assert_eq!(ups.len(), 2);
        assert_eq!(
            (ups[0].dbp, ups[0].psm, ups[0].w, ups[0].h, ups[0].dbw, ups[0].data.len()),
            (0x100, 0x14, 16, 16, 128, 128)
        );
        let pics = pictures(&ups, &|_| None);
        assert_eq!(pics.len(), 1); // the palette is not a picture
        assert_eq!((pics[0].0, pics[0].1, &pics[0].2[..4]), (16, 16, &[255, 0, 0, 255][..]));
        // TEX0: the image at 0x100, 16 x 16 (2^4), PSMT4, palette at 0x200
        let t = tex0(0x100 | 2 << 14 | 0x14 << 20 | 4 << 26 | 4 << 30 | 0x200 << 37);
        let (w, h, px) = image(&ups, &t, false).unwrap();
        assert_eq!((w, h, &px[..4], px.len()), (16, 16, &[255, 0, 0, 255][..], 1024));
        assert!(std::str::from_utf8(&png(w, h, &px)[1..4]).is_ok_and(|s| s == "PNG"));
        // no such texture; a size no texture has
        assert!(image(&ups, &tex0(0x300 | 0x14 << 20 | 4 << 26 | 4 << 30 | 0x200 << 37), false).is_none());
        assert!(image(&ups, &tex0(0x100 | 0x14 << 20 | 15 << 26 | 15 << 30 | 0x200 << 37), false).is_none());
        // cut anywhere, or noise: what is whole comes out, and nothing panics
        for n in 0..d.len() {
            let ups = uploads(&d[..n]);
            let _ = (pictures(&ups, &|_| None), image(&ups, &t, true));
        }
        let noise: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        for o in 0..64 {
            let ups = uploads(&noise[o..]);
            let _ = (pictures(&ups, &|_| None), image(&ups, &t, true));
        }
    }

    #[test]
    fn a_channel_frame_goes_where_its_table_says() {
        // a pack (`archive::pack`): count, first offset, (size, next) pairs, the parts 16-aligned
        fn pack(parts: &[Vec<u8>]) -> Vec<u8> {
            let mut o = (8 + 8 * parts.len()).next_multiple_of(16);
            let mut d = [(parts.len() as u32).to_le_bytes(), (o as u32).to_le_bytes()].concat();
            for (k, p) in parts.iter().enumerate() {
                o += p.len().next_multiple_of(16);
                let next = if k + 1 == parts.len() { u32::MAX } else { o as u32 };
                d.extend([(p.len() as u32).to_le_bytes(), next.to_le_bytes()].concat());
            }
            d.resize(d.len().next_multiple_of(16), 0);
            parts.iter().for_each(|p| {
                d.extend(p.iter().copied().chain(std::iter::repeat_n(0, p.len().next_multiple_of(16) - p.len())))
            });
            d
        }
        // a 128 x 192 PSMT8 sheet of 0s; channel 0's frames 0 and 1 (16 x 8 of 1s, of 2s) go to page 2 (block
        // 0x40), at (16, 4)
        let mut ups = uploads(&gif(0, 0x13, 128, 192, &[0; 128 * 192]));
        let frames = pack(&[pack(&[pack(&[gif(0, 0x13, 16, 8, &[1; 128]), gif(0, 0x13, 16, 8, &[2; 128])])])]);
        let mut model = vec![0u8; 0x14];
        model.extend(0x1Cu32.to_le_bytes());
        model.extend(1u32.to_le_bytes());
        model.extend([0u16, 0x40, 16, 4, 16, 8, 2, 0x13].iter().flat_map(|v| v.to_le_bytes()));
        assert_eq!(channel_frame(&mut ups, &model, &frames, 0, 1), Some(()));
        let at = |x: usize, y: usize| ups[0].data[y * 128 + x];
        assert_eq!((at(16, 132), at(31, 139), at(15, 132), at(16, 140), at(16, 4)), (2, 2, 0, 0, 0));
        // no such channel or frame: nothing changes
        assert_eq!(
            (channel_frame(&mut ups, &model, &frames, 1, 0), channel_frame(&mut ups, &model, &frames, 0, 2)),
            (None, None)
        );
        assert_eq!(ups[0].data.iter().filter(|&&p| p != 0).count(), 128);
    }

    #[test]
    fn a_named_palette_wins() {
        // an image of colour 1, then two palettes (red, then green): the nearest after it, or the one named
        let mut d = gif(0x100, 0x14, 16, 16, &[0x11; 128]);
        for c in [[255, 0, 0, 0x80], [0, 255, 0, 0x80]] {
            let mut clut = vec![0u8; 64];
            clut[4..8].copy_from_slice(&c);
            d.extend(gif(0x200, 0, 8, 2, &clut));
        }
        let ups = uploads(&d);
        assert_eq!(&pictures(&ups, &|_| None)[0].2[..4], &[255, 0, 0, 255]);
        assert_eq!(&pictures(&ups, &|_| Some(2))[0].2[..4], &[0, 255, 0, 255]);
        // a name past the uploads, or of an image: the nearest
        assert_eq!(&pictures(&ups, &|_| Some(9))[0].2[..4], &[255, 0, 0, 255]);
        assert_eq!(&pictures(&ups, &|_| Some(0))[0].2[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn palettes_baked_into_an_atlas() {
        // a 16 x 16 PSMT8 image of index 8 in a 256-wide sheet, and a 256-colour palette whose entry 16 is green
        // and entry 8 red: CSM1 order swaps bits 3 and 4, so the image is green; half-transparent where kept
        let mut clut = vec![0u8; 1024];
        clut[64..68].copy_from_slice(&[0, 255, 0, 0x40]);
        clut[32..36].copy_from_slice(&[255, 0, 0, 0x80]);
        let mut d = gif(0, 0x13, 256, 16, &[8; 256 * 16]);
        d.extend(gif(0x300, 0, 16, 16, &clut));
        let ups = uploads(&d);
        let t = tex0(4 << 14 | 0x13 << 20 | 4 << 26 | 4 << 30 | 0x300 << 37);
        let (w, h, px) = image(&ups, &t, true).unwrap();
        assert_eq!((w, h, &px[..4]), (16, 16, &[0, 255, 0, 0x80][..]));
        assert_eq!(&image(&ups, &t, false).unwrap().2[..4], &[0, 255, 0, 255]);
        // in an atlas: the tall one first, each with a border of its edge texels
        let a = atlas(&[(w, h, px), (2, 40, vec![9; 2 * 40 * 4])]).unwrap();
        assert_eq!((a.w, a.h, &a.place), (1024, 64, &vec![(5, 1), (1, 1)]));
        let at = |x: usize, y: usize| &a.rgba[(y * a.w + x) * 4..][..4];
        assert_eq!(
            (at(5, 1), at(4, 0), at(21, 17), at(22, 1)),
            (&[0, 255, 0, 0x80][..], &[0, 255, 0, 0x80][..], &[0, 255, 0, 0x80][..], &[0; 4][..])
        );
        assert_eq!((at(0, 0), at(3, 41)), (&[9; 4][..], &[9; 4][..]));
        // a picture of no size, or of fewer pixels than its size, or taller than an atlas: none
        assert!(atlas(&[(0, 4, vec![])]).is_none());
        assert!(atlas(&[(4, 4, vec![0; 60])]).is_none());
        assert!(atlas(&[(1, 9000, vec![0; 36000])]).is_none());
        assert_eq!(atlas(&[]).unwrap().h, 1);
    }
}

//! Motions: the arcade (System 12) bit-packed format, 57 channels a frame at 60 fps:
//! polar root (heading, height, distance), then (x, y, z) angles of the 18 joints of `RIG` in order
//! (the arcade stores them in another order), 65536 = 360 degrees.
use crate::glb::{self, Glb};
use crate::model::{RIG, SCALE};
use crate::u16le;
use serde_json::json;

pub const CHANNELS: usize = 57;
pub type Frame = [u16; CHANNELS];

struct Bits<'a> {
    d: &'a [u8],
    o: usize,
    acc: u32,
    n: u32,
}
impl Bits<'_> {
    /// The next `n` bits (12 at most).
    fn take(&mut self, n: u32) -> Option<u32> {
        while self.n < n {
            self.acc |= (u16le(self.d, self.o)? as u32) << self.n;
            self.o += 2;
            self.n += 16;
        }
        let v = self.acc & ((1 << n) - 1);
        self.acc >>= n;
        self.n -= n;
        Some(v)
    }
}

/// Every frame of the motion at `base`. None when the data is not a motion.
/// `arcade`: the original packing (16-frame blocks, 4-bit fields), which the arenas' background
/// characters keep; the fighters' motions are repacked with 32-frame blocks and 5-bit fields.
pub fn decode(d: &[u8], base: usize, arcade: bool) -> Option<Vec<Frame>> {
    let d = d.get(base..)?;
    if d.len() < 0x130 {
        return None;
    }
    let count = if arcade { u16le(d, 0)? as usize } else { *d.first()? as usize }; // the repacked ones are at most 255 frames
    if count == 0 {
        return None;
    }
    let (block_len, bits) = if arcade { (16, 4) } else { (32, 5) };
    let row = |o: usize| -> Option<Frame> {
        let mut r = [0; CHANNELS];
        for (c, v) in r.iter_mut().enumerate() {
            *v = u16le(d, o + 2 * c)?;
        }
        Some(r)
    };
    let first = row(2)?;
    let mut frames = vec![first];
    for f in 1..count.saturating_sub(1) {
        let (block, within) = ((f - 1) / block_len, (f - 1) % block_len);
        // every block restarts from frame 0's values
        let mut cursor = u16le(d, 0xe6 + block * 2)? as usize * 2;
        let mut out = first;
        for (c, out) in out.iter_mut().enumerate() {
            let mut b = Bits { d, o: cursor, acc: 0, n: 0 };
            let span = b.take(bits)? as usize;
            if span == 0 {
                return None;
            }
            let shift = if c == 1 || c == 2 { 2 } else { 4 };
            let (mut v, mut repeat, mut code) = (*out as i32, 0, 0);
            for _ in 0..=within {
                if repeat > 0 {
                    repeat -= 1;
                } else {
                    code = b.take(4)?;
                    if code >= 8 {
                        repeat = (b.take(bits)? + 1) & ((1 << bits) - 1);
                        code -= 8;
                    }
                }
                let (w, bias) =
                    *[(0, 0), (6, -84), (4, -20), (2, -4), (2, 1), (4, 5), (6, 21), (12, 0)].get(code as usize)?;
                if code == 7 {
                    v = (b.take(12)? as i32) << shift;
                } else if code > 0 {
                    v = v.wrapping_add((b.take(w)? as i32 + bias) << shift);
                }
            }
            *out = v as u16;
            cursor += span * 2;
        }
        frames.push(out);
    }
    if count > 1 {
        frames.push(row(0x74)?);
    }
    Some(frames)
}

/// An unpacked motion, as some arenas store their background characters': u16 frames, u16 19
/// (three-channel groups), then the frames.
pub fn raw(d: &[u8]) -> Option<Vec<Frame>> {
    if u16le(d, 2)? != 19 {
        return None;
    }
    let frames = d.get(4..4 + u16le(d, 0)? as usize * CHANNELS * 2)?;
    let frames: Vec<Frame> = frames
        .as_chunks::<{ CHANNELS * 2 }>()
        .0
        .iter()
        .map(|f| {
            let mut r = [0; CHANNELS];
            for (v, b) in r.iter_mut().zip(f.as_chunks::<2>().0) {
                *v = u16::from_le_bytes(*b);
            }
            r
        })
        .collect();
    Some(frames).filter(|f| !f.is_empty())
}

/// A motion as a skeleton-only GLB: one animation whose channels target the joints by name, so
/// it plays on any fighter's GLB (three.js binds tracks by node name).
// (a frame's channels are indexed below CHANNELS: 3 + 3 * 17 + 2 = 56)
#[allow(clippy::indexing_slicing)]
pub fn glb(frames: &[Frame]) -> Vec<u8> {
    let mut g = Glb::default();
    let ang = |v: u16| v as f64 * std::f64::consts::TAU / 65536.0;
    let times: Vec<f32> = (0..frames.len()).map(|i| i as f32 / 60.0).collect();
    let input = g.ranged(&times, 1, "SCALAR");
    let (mut nodes, mut samplers, mut channels) = (vec![], vec![], vec![]);
    for (p, (joint, twins)) in RIG.iter().enumerate() {
        let mut q: Vec<f32> = Vec::with_capacity(frames.len() * 4);
        for f in frames {
            // the data is for the arcade's mirrored space: x and y angles change sign
            let mut x = glb::quat(&glb::joint([-ang(f[3 + 3 * p]), -ang(f[4 + 3 * p]), ang(f[5 + 3 * p])], [0.; 3]));
            // keep the same hemisphere as the previous key so interpolation takes the short way
            if q.last_chunk::<4>().is_some_and(|l| l.iter().zip(&x).map(|(a, b)| a * b).sum::<f32>() < 0.0) {
                x = x.map(|v| -v);
            }
            q.extend(x);
        }
        let output = g.accessor(&glb::floats(&q), glb::F32, "VEC4", frames.len());
        for k in 0..twins.len() {
            let name = if k == 0 { joint.to_string() } else { format!("{joint}.{k}") };
            nodes.push(json!({"name": name}));
            samplers.push(json!({"input": input, "output": output}));
            channels
                .push(json!({"sampler": samplers.len() - 1, "target": {"node": nodes.len() - 1, "path": "rotation"}}));
        }
    }
    let t: Vec<f32> = frames
        .iter()
        .flat_map(|f| {
            let (h, d) = (ang(f[0]), f[2] as f64 * SCALE);
            // the same mirror as the angles: z runs the other way (a punch must carry its fighter forward)
            [(h.sin() * d) as f32, (f[1] as f64 * SCALE) as f32, (-h.cos() * d) as f32]
        })
        .collect();
    let output = g.accessor(&glb::floats(&t), glb::F32, "VEC3", frames.len());
    samplers.push(json!({"input": input, "output": output}));
    channels.push(json!({"sampler": samplers.len() - 1, "target": {"node": 0, "path": "translation"}}));
    let all: Vec<usize> = (0..nodes.len()).collect();
    g.finish(json!({"scene": 0, "scenes": [{"nodes": all}], "nodes": nodes, "animations": [{"name": "motion", "samplers": samplers, "channels": channels}]}))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn raw_reads_whole_frames_only() {
        let mut d = vec![2, 0, 19, 0];
        d.extend((0..CHANNELS as u16 * 2).flat_map(|v| v.to_le_bytes()));
        let f = raw(&d).unwrap();
        assert_eq!((f.len(), f[1][0], f[1][56]), (2, 57, 113));
        assert!(raw(&d[..d.len() - 2]).is_none()); // a cut frame
        assert!(raw(&[2, 0, 18, 0]).is_none()); // not this format
        assert!(raw(&[]).is_none());
    }

    /// Bits, lowest first, as the stream's little-endian u16 words.
    struct W(Vec<u8>, u32, u32);
    impl W {
        fn put(&mut self, v: u32, n: u32) {
            self.1 |= v << self.2;
            self.2 += n;
            while self.2 >= 16 {
                self.0.extend((self.1 as u16).to_le_bytes());
                self.1 >>= 16;
                self.2 -= 16;
            }
        }
        fn end(mut self) -> Vec<u8> {
            self.put(0, 15);
            self.0
        }
    }

    /// A fighter's motion of `count` frames in one block: each channel `c` starts at `100 * c`, and its stream is
    /// `stream(c)`'s bits after its 5-bit length.
    fn motion(count: u8, stream: impl Fn(usize, &mut W)) -> Vec<u8> {
        let mut d = vec![0u8; 0x130];
        d[0] = count;
        for c in 0..CHANNELS {
            d[2 + 2 * c..][..2].copy_from_slice(&(100 * c as u16).to_le_bytes());
            d[0x74 + 2 * c..][..2].copy_from_slice(&(7 + c as u16).to_le_bytes()); // the last frame, stored whole
        }
        d[0xe6..0xe8].copy_from_slice(&(0x130u16 / 2).to_le_bytes()); // block 0 starts after the header
        for c in 0..CHANNELS {
            let mut w = W(vec![], 0, 0);
            w.put(0, 5);
            stream(c, &mut w);
            let mut body = w.end();
            body[0] |= (body.len() / 2) as u8; // the channel's length in words
            d.extend(body);
        }
        d
    }

    #[test]
    fn decode_follows_the_bit_stream() {
        // four frames: the first and the last stored whole, two decoded.
        // channel 0: code 4 (2 bits + 1) with 3: +4 << 4; then code 7: 0x123 << 4, absolute
        // channel 1 (height: shift 2): code 1 (6 bits - 84) with 0: -84 << 2, repeated by code 8 + 1 with count 1...
        // the others: code 0 (no change), run of 2 (code 8, repeat 1)
        let d = motion(4, |c, w| match c {
            0 => {
                w.put(4, 4);
                w.put(3, 2);
                w.put(7, 4);
                w.put(0x123, 12);
            }
            1 => {
                w.put(9, 4); // code 1, repeated once more
                w.put(1, 5);
                w.put(0, 6);
                w.put(0, 6);
            }
            _ => {
                w.put(8, 4);
                w.put(1, 5);
            }
        });
        let f = decode(&d, 0, false).unwrap();
        assert_eq!(f.len(), 4);
        assert_eq!(f[0][5], 500);
        assert_eq!((f[1][0], f[2][0]), (4 << 4, 0x123 << 4));
        assert_eq!((f[1][1], f[2][1]), ((100 - (84 << 2)) as u16, (100 - 2 * (84 << 2)) as u16));
        assert_eq!((f[1][5], f[2][5], f[2][56]), (500, 500, 5600));
        assert_eq!((f[3][0], f[3][56]), (7, 63));
        // at an offset in a bank
        let mut bank = vec![0xee; 10];
        bank.extend(&d);
        assert_eq!(decode(&bank, 10, false).unwrap(), f);
    }

    #[test]
    fn decode_rejects_broken_motions() {
        let d = motion(4, |_, w| {
            w.put(8, 4);
            w.put(1, 5);
        });
        assert!(decode(&d, 0, false).is_some());
        // cut anywhere (the stream, the header), past the end, empty: not a motion, and not a panic
        for n in 0..d.len() {
            assert!(decode(&d[..n], 0, false).is_none(), "{n}");
        }
        assert!(decode(&d, d.len() + 1, false).is_none());
        assert!(decode(&d, usize::MAX, false).is_none());
        // no frames; a channel of no length
        let mut none = d.clone();
        none[0] = 0;
        assert!(decode(&none, 0, false).is_none());
        let mut empty = d.clone();
        empty[0x130] = 0;
        assert!(decode(&empty, 0, false).is_none());
        // a block that starts far outside
        let mut far = d.clone();
        far[0xe6..0xe8].copy_from_slice(&[0xff, 0xff]);
        assert!(decode(&far, 0, false).is_none());
        // any bytes: never a panic
        let mut x = 1u32;
        let noise: Vec<u8> = (0..4000)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8
            })
            .collect();
        for base in 0..64 {
            let _ = decode(&noise, base, false);
            let _ = decode(&noise, base, true);
        }
    }
}

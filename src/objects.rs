//! An arena's objects (docs/EFFECTS.md 7): the models of its geometry pack's part 3, placed by the object table
//! 0x520FC0 and moved by the arena's code, which the converter runs here frame by frame to record their motion.
//!
//! The table's row of arena n (16 bytes): `{*table, u32, *update, *init}`; the table `{u32 links, *links, u32 objects,
//! *objects}`; a link `{s16 parent, s16 child}`; an object (64 bytes, 0x3D0E18 makes the runtime object of 0x70):
//! `s16 model` (a block of the object part, [`crate::stage::blocks`]), +2 the view (0 the arena's, 1 the floor's),
//! +4, +6 the draw bucket, +8 the most children, +0x10 the turn (x, y, z, radians), +0x20 the place (mm), +0x30 the
//! routine run as it is drawn (0x3D1468), +0x34 the one run once made. An object is drawn as `parent · T · R` with R
//! = Rx·Ry·Rz (0x34AD60: z turned first), or Rz·Ry·Rx when its +8 is set (the arena's code sets it), and only when
//! the arena's update routine draws it (0x3D10D0, 0x3D1160): which ones each arena draws, and how its code moves
//! them, is below.

use crate::moves::VADDR;
use crate::stage::Batch;
use crate::{f32le, u16_at, u32_at};
use anyhow::{Context, Result};

/// The arenas' rows (0x3D0E18).
const ROWS: usize = 0x52_0FC0;
const GP: usize = 0x5F_D0F0;
const PI: f64 = std::f64::consts::PI;

/// An object's place in a frame: its parent's axes, then `at` (mm), turned by `turn` (radians: Rx·Ry·Rz, or Rz·Ry·Rx
/// when `xyz`), and `scale`d.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub at: [f64; 3],
    pub turn: [f64; 3],
    pub xyz: bool,
    pub scale: [f64; 3],
}

/// An object as the arena draws it: its model (none: a frame for its children), its parent (an index into the list),
/// and its pose in each frame from the arena's start (one: it stands still); the last pose is where the first one's
/// motion goes on (the loop's end).
pub struct Object {
    pub model: Vec<Batch>,
    pub parent: Option<usize>,
    pub poses: Vec<Pose>,
    /// Turned about y to face the camera each frame (Xiaoyu's lamps, 0x3D5640: the camera's turn less its parent's).
    pub face: bool,
}

/// The pose as a rotation quaternion (x, y, z, w).
pub fn quat(p: &Pose) -> [f64; 4] {
    let axis = |k: usize, a: f64| {
        let (s, c) = (libm::sin(a / 2.0), libm::cos(a / 2.0));
        let mut q = [0.0, 0.0, 0.0, c];
        if let Some(v) = q.get_mut(k) {
            *v = s;
        }
        q
    };
    let mul = |a: [f64; 4], b: [f64; 4]| {
        let ([ax, ay, az, aw], [bx, by, bz, bw]) = (a, b);
        [
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz,
        ]
    };
    let [x, y, z] = [0, 1, 2].map(|k| axis(k, p.turn.get(k).copied().unwrap_or(0.0)));
    if p.xyz { mul(mul(z, y), x) } else { mul(mul(x, y), z) }
}

/// The angle kept within ±π as the arena's code keeps it (0x3D80E8).
fn wrap(a: f64) -> f64 {
    if a > PI {
        a - 2.0 * PI
    } else if a < -PI {
        a + 2.0 * PI
    } else {
        a
    }
}

/// So many frames of a turn of `rate` radians a frame, at most `most`, as ends nearest a whole number of turns.
fn loop_frames(rate: f64, most: usize) -> usize {
    let off = |n: usize| {
        let a = (n as f64 * rate).rem_euclid(2.0 * PI);
        a.min(2.0 * PI - a)
    };
    (1..=most).min_by(|&a, &b| off(a).total_cmp(&off(b))).unwrap_or(1)
}

/// Arena `arena`'s objects as its code draws them in a fight, from the object table and the models of its object
/// part (`blocks`, each one's batches); none for an arena that draws none (or the bowl's, which OpenKumite's bowl
/// draws itself).
// (the objects indexed below are fewer than the table's, checked first)
#[allow(clippy::too_many_lines, clippy::indexing_slicing)]
pub fn objects(elf: &[u8], arena: usize, blocks: &[Vec<Batch>]) -> Result<Vec<Object>> {
    let what = "the arena's object table";
    let at = |p: u32| (p as usize).checked_sub(VADDR).context(what);
    let row = ROWS - VADDR + 16 * arena;
    let table = at(u32_at(elf, row, what)?)?;
    let word = |k: usize| u32_at(elf, table + 4 * k, what);
    let (links, link_list, count, list) = (word(0)? as usize, word(1)?, word(2)? as usize, word(3)?);
    let s16 = |o: usize| -> Result<i16> { Ok(u16_at(elf, o, what)? as i16) };
    let f = |o: usize| f32le(elf, o).map(f64::from).context(what);
    // a constant of the arena's code (gp - off)
    let gp = |off: usize| f(GP - off - VADDR);
    let (count, links) = (count.min(64), links.min(64));
    let list = if count > 0 { at(list)? } else { 0 };
    let link_list = if links > 0 { at(link_list)? } else { 0 };
    let mut parent = vec![None; count];
    for k in 0..links {
        let (p, c) = (s16(link_list + 4 * k)?, s16(link_list + 4 * k + 2)?);
        if let (Ok(p), Some(slot)) = (usize::try_from(p), usize::try_from(c).ok().and_then(|c| parent.get_mut(c))) {
            *slot = Some(p).filter(|&p| p < count);
        }
    }
    let rest = |k: usize| -> Result<Pose> {
        let o = list + 64 * k;
        Ok(Pose {
            at: [f(o + 0x20)?, f(o + 0x24)?, f(o + 0x28)?],
            turn: [f(o + 0x10)?, f(o + 0x14)?, f(o + 0x18)?],
            xyz: false,
            scale: [1.0; 3],
        })
    };
    let model = |k: usize| -> Result<Vec<Batch>> {
        let m = usize::try_from(s16(list + 64 * k)?).unwrap_or(usize::MAX);
        Ok(blocks.get(m).cloned().unwrap_or_default())
    };
    // the objects drawn (each with its subtree), by the arena's update routine; then their motion
    let drawn: Vec<usize> = match arena {
        // Paul (0x3D5798): the train, 0 and its three cars
        0 => vec![0],
        // Law-A (0x3D6FD8): the lights 4-13, each in its own scale, and 14-18
        1 => (4..19).collect(),
        // Xiaoyu (0x3D52B0): the carousel 0 (the horses 1-8, the lamps 9-12), 13, 14
        3 => vec![0, 13, 14],
        // Nina (0x3D5E80): the helicopter 1 and its rotors
        9 | 18 => vec![1],
        // King (0x3D4B18): the helicopters' formation 11 (11 itself not drawn: 0x3D4AD0)
        11 => vec![11],
        // Unknown (0x3D2168)
        14 => vec![0, 1],
        _ => vec![],
    };
    // the objects each arena's code names
    let named = match arena {
        0 => 4,
        1 => 19,
        3 => 15,
        9 | 18 => 4,
        11 => 21,
        14 => 2,
        _ => 0,
    };
    if drawn.is_empty() || count < named {
        return Ok(vec![]);
    }
    // drawn: under a drawn object (a link list may loop: a chain no longer than the objects)
    let keep: Vec<bool> = (0..count)
        .map(|k| {
            let mut p = Some(k);
            for _ in 0..=count {
                match p {
                    Some(q) if drawn.contains(&q) => return true,
                    Some(q) => p = parent.get(q).copied().flatten(),
                    None => break,
                }
            }
            false
        })
        .collect();
    let mut out: Vec<Object> = (0..count)
        .map(|k| {
            let hidden = arena == 11 && k == 11;
            Ok(Object {
                model: if keep[k] && !hidden { model(k)? } else { vec![] },
                parent: parent[k],
                poses: vec![rest(k)?],
                // Xiaoyu's lamps 9-12 (their routine 0x3D5640)
                face: arena == 3 && (9..13).contains(&k),
            })
        })
        .collect::<Result<_>>()?;
    // runs `step(frame, poses)` for `frames` frames from the rest poses, recording the objects in `which`
    let run = |out: &mut Vec<Object>, which: &[usize], frames: usize, step: &mut dyn FnMut(usize, &mut Vec<Pose>)| {
        let mut now: Vec<Pose> = out.iter().map(|o| o.poses[0]).collect();
        let mut tracks: Vec<Vec<Pose>> = vec![vec![]; which.len()];
        for t in 0..=frames {
            step(t, &mut now);
            for (track, &k) in tracks.iter_mut().zip(which) {
                track.push(now[k]);
            }
        }
        for (track, &k) in tracks.into_iter().zip(which) {
            out[k].poses = track;
        }
    };
    match arena {
        0 => {
            // 0x3D56F0: the train starts at (0, 14000, 200000) (0x521B90) turned 5 to 10 degrees about y, z negated;
            // 0x3D5798 each frame heads it along its circle (-atan2(z, x) + pi/2) and moves it 278 mm (0x521BA0)
            // that way; after 1800 to 2400 frames it starts again at another angle (here: always the middle ones)
            let (start, speed) = (f(0x52_1B90 - VADDR + 8)?, f(0x52_1BA0 - VADDR)?);
            let a = 1.5 * gp(0x7CB0)?;
            let (s, c) = (libm::sin(a), libm::cos(a));
            let origin = [start * s, f(0x52_1B94 - VADDR)?, -start * c];
            let quarter = gp(0x7CAC)?;
            run(&mut out, &[0], 2100, &mut |t, now| {
                let p = &mut now[0];
                if t == 0 || t == 2100 {
                    p.at = origin;
                }
                p.turn[1] = wrap(-libm::atan2(p.at[2], p.at[0]) + quarter);
                if t < 2100 {
                    p.at[0] += speed * libm::cos(p.turn[1]);
                    p.at[2] -= speed * libm::sin(p.turn[1]);
                }
            });
        }
        1 => {
            // 0x3D6FD8 draws lights 4-13 in a frame of its own scale (x and z; y 1.1)
            let scales = [0x7BC4, 0x7BBC, 0x7BB8, 0x7BB4, 0x7BB4, 0x7BC4, 0x7BBC, 0x7BB8, 0x7BB4, 0x7BB4];
            let y = gp(0x7BC0)?;
            for (k, off) in (4..14).zip(scales) {
                let s = gp(off)?;
                out.push(Object {
                    model: vec![],
                    parent: None,
                    poses: vec![Pose { at: [0.0; 3], turn: [0.0; 3], xyz: false, scale: [s, y, s] }],
                    face: false,
                });
                out[k].parent = Some(out.len() - 1);
            }
        }
        3 => {
            // the carousel turns gp-0x7CC0 a frame (0x3D54F8); each horse rises and falls 300 mm on a sine whose
            // phase goes gp-0x7CBC a frame (0x3D5538), the second four half a turn behind (0x3D5630)
            let (turn, bob) = (gp(0x7CC0)?, gp(0x7CBC)?);
            run(&mut out, &[0], loop_frames(turn, 2000), &mut |_, now| now[0].turn[1] = wrap(now[0].turn[1] + turn));
            let horses: Vec<usize> = (1..9).collect();
            let mut phase = [0.0f64; 8];
            for p in phase.iter_mut().skip(4) {
                *p = gp(0x7CB8)?;
            }
            run(&mut out, &horses, loop_frames(bob, 420), &mut |_, now| {
                for (k, p) in phase.iter_mut().enumerate() {
                    *p = wrap(*p + bob);
                    now[k + 1].at[1] = 300.0 * libm::sin(*p);
                }
            });
        }
        9 | 18 => {
            // 0x3D6208: the helicopter turned about z, y, x (its +8 set), half a turn about y (pi/6 - rand x pi/3 +
            // pi: the middle one); 0x3D62A0 while it waits on its pad: 100 mm up and down on a sine of gp-0x7C48 a
            // frame. Its rotors turn gp-0x7BE4 (the main one, about y: 0x3D6B50) and gp-0x7BD8 (the tail's, about x:
            // 0x3D6B88) a frame. Not done: its take-off and return (states 2 and 3, after 600 frames out of sight).
            let (base, bob) = (out[1].poses[0].at[1], gp(0x7C48)?);
            out[1].poses[0].xyz = true;
            out[1].poses[0].turn[1] = gp(0x7C50)? - 0.5 * gp(0x7C54)? + gp(0x7C4C)?;
            let mut phase = 0.0;
            run(&mut out, &[1], loop_frames(bob, 420), &mut |_, now| {
                phase = wrap(phase + bob);
                now[1].at[1] = base + 100.0 * libm::sin(phase);
            });
            let (main, tail) = (gp(0x7BE4)?, gp(0x7BD8)?);
            run(&mut out, &[2, 3], loop_frames(main, 120), &mut |_, now| {
                now[2].turn[1] = wrap(now[2].turn[1] + main);
                now[3].turn[0] = wrap(now[3].turn[0] + tail);
            });
        }
        11 => {
            // 0x3D4B18: the formation's frame 11 goes round an ellipse (x 100 m: gp-0x7D00, z 80 m: gp-0x7CFC) by
            // gp-0x7D04 a frame from pi - rand x 2pi (here: 0), turned -atan2(z, x); the two wingmen 15 and 18 turn
            // to -atan2 of their own place, less 11's turn (0x3D5190). The rotors: 13, 16, 19 about y by gp-0x7CF0
            // a frame (0x3D4ED8), 14, 17, 20 about x by gp-0x7CEC (0x3D4F18).
            let (step, rx, rz) = (gp(0x7D04)?, gp(0x7D00)?, gp(0x7CFC)?);
            let mut a = 0.0f64;
            run(&mut out, &[11, 15, 18], loop_frames(step, 4000), &mut |t, now| {
                if t > 0 {
                    a = wrap(a + step);
                }
                now[11].at = [libm::cos(a) * rx, 0.0, libm::sin(a) * rz];
                now[11].turn[1] = wrap(-libm::atan2(now[11].at[2], now[11].at[0]));
                for k in [15, 18] {
                    let (x, z) = (now[11].at[0] + now[k].at[0], now[11].at[2] + now[k].at[2]);
                    now[k].turn[1] = wrap(-libm::atan2(z, x) - now[11].turn[1]);
                }
            });
            let (main, tail) = (gp(0x7CF0)?, gp(0x7CEC)?);
            run(&mut out, &[13, 14, 16, 17, 19, 20], loop_frames(main, 120), &mut |_, now| {
                for k in [13, 16, 19] {
                    now[k].turn[1] = wrap(now[k].turn[1] + main);
                }
                for k in [14, 17, 20] {
                    now[k].turn[0] = wrap(now[k].turn[0] + tail);
                }
            });
        }
        _ => {}
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn turns_compose_as_the_ps2_does() {
        // Rx·Ry·Rz: z first; a quarter turn about y takes x to -z
        let p = Pose { at: [0.0; 3], turn: [0.0, PI / 2.0, 0.0], xyz: false, scale: [1.0; 3] };
        let [x, y, z, w] = quat(&p);
        assert!((y - libm::sqrt(0.5)).abs() < 1e-9 && x == 0.0 && z == 0.0 && (w - libm::sqrt(0.5)).abs() < 1e-9);
        assert_eq!(loop_frames(1.029744267463684, 120), 61);
        assert_eq!(wrap(3.5), 3.5 - 2.0 * PI);
    }
}

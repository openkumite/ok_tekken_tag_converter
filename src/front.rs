//! The front end (docs/FRONTEND.md): the 2D files' palettes, and the tables OpenKumite's menus, select screen, HUD,
//! arcade ladder and unlocks read, as roster.json `front_end` (docs/GAMEDATA.md "front_end"). The pictures are the
//! 2D files' images as `assets/pictures/FFF_NNN.png`, the names `textures/` has; `front_end` names them.

use crate::moves::{self, VADDR};
use crate::tex::{self, Upload};
use crate::{u8_at, u16_at, u32_at};
use anyhow::Result;
use serde_json::{Value, json};

/// A TEKKEN.BIN file of 2D pictures the front end draws (FRONTEND.md 1: the sprite groups' files but Tekken Bowl's).
pub fn is_2d(file: usize) -> bool {
    matches!(file, 3 | 130..=132 | 134 | 136 | 141..=182)
}
/// The portraits' files 143-181, one per body, in name order: their bodies.
const PORTRAITS: [usize; 39] = [
    26, 37, 38, 18, 32, 31, 12, 29, 8, 27, 13, 6, 21, 16, 9, 10, 30, 19, 3, 11, 28, 1, 25, 2, 7, 23, 15, 5, 20, 14, 35,
    0, 22, 17, 36, 34, 33, 24, 4,
];
/// Bodies: the 34 characters and the costume bodies Tiger, Panda, Tetsujin, Alex, Angel.
const BODIES: usize = 39;

/// The palette upload of image upload `k` of 2D file `file`, where the nearest one after it is not the one drawn
/// with (FRONTEND.md 1, 3, 4): file 3's faces share upload 132 (palette id 0x84) but RANDOM's and the locked face,
/// which take upload 163 (0x3CBFB8), its name plates upload 76 (0x4C), the menu's arrows upload 40 (0x3CCF40: palette
/// table 0x520638's fourth); file 141's pictures take its upload 0.
pub fn palette(file: usize, k: usize) -> Option<usize> {
    match (file, k) {
        (3, 162 | 165) => Some(163),
        (3, 129 | 130) => Some(40),
        (3, 131..=173) => Some(132),
        (3, 75..=114) => Some(76),
        (141, 1..) => Some(0),
        _ => None,
    }
}

/// The health-bar strips drawn angry (FRONTEND.md 4): uploads 45-55 of file 3 with palettes 0x3D-0x42, as
/// `003_NNN_angry.png` (NNN: the strip's picture number): each its file name and PNG.
pub fn angry_strips(ups: &[Upload]) -> Vec<(String, Vec<u8>)> {
    (0..6)
        .filter_map(|i| {
            let pair = [ups.get(45 + 2 * i)?.clone(), ups.get(61 + i)?.clone()];
            let (w, h, px) = tex::pictures(&pair, &|_| Some(1)).into_iter().next()?;
            Some((format!("003_{:03}_angry.png", 6 + i), tex::png(w, h, &px)))
        })
        .collect()
}

/// The select cursor (upload 115 of file 3) in 2P's palette, upload 117 (0x3CC9E0: 1P's is 116, the one after it):
/// `003_057_p2.png` and its PNG.
pub fn cursor_2p(ups: &[Upload]) -> Option<(String, Vec<u8>)> {
    let pair = [ups.get(115)?.clone(), ups.get(117)?.clone()];
    let (w, h, px) = tex::pictures(&pair, &|_| Some(1)).into_iter().next()?;
    Some(("003_057_p2.png".to_string(), tex::png(w, h, &px)))
}

/// The text colours (the printf `%c`: palette upload 9 + colour, 36 of them) of the chrome font (upload 2 of file 3) and
/// the small one (upload 4): `003_001_cNN.png`, `003_002_cNN.png`, NN the colour, and their PNGs.
pub fn font_colours(ups: &[Upload]) -> Vec<(String, Vec<u8>)> {
    [(2, 1), (4, 2)]
        .into_iter()
        .flat_map(|(up, picture)| (0..FONT_COLOURS).map(move |c| (up, picture, c)))
        .filter_map(|(up, picture, c)| {
            let pair = [ups.get(up)?.clone(), ups.get(9 + c)?.clone()];
            let (w, h, px) = tex::pictures(&pair, &|_| Some(1)).into_iter().next()?;
            Some((format!("003_{picture:03}_c{c:02}.png"), tex::png(w, h, &px)))
        })
        .collect()
}
/// The chrome font's colours (palettes 9-44 of file 3).
const FONT_COLOURS: usize = 36;

/// A picture of file 3 by its upload number among `ups` (its uploads): `003_NNN.png`, NNN its place among the
/// images, palettes not counted (sprites.py).
fn sprite(ups: &[Upload], k: usize) -> String {
    format!("003_{:03}.png", ups.iter().take(k).filter(|u| !tex::is_palette(u)).count())
}

/// A string of the executable as it is (codes like `%H%V` left out).
fn string(elf: &[u8], pointer: u32) -> String {
    let at = (pointer as usize).wrapping_sub(VADDR);
    let raw: Vec<u8> = elf.get(at..).unwrap_or_default().iter().take(96).take_while(|&&c| c != 0).copied().collect();
    let s = String::from_utf8_lossy(&raw).to_string();
    // the codes: % and one letter, %P%F%c... (FRONTEND.md 2)
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// The select's 3D characters (FRONTEND.md 9): per character id the clips of its moveset's aliases 0 (standing, looped)
/// and 3 (the pick's pose), None where its set has none (Mokujin's and Unknown's stub). `clips`: each move's clip.
fn select_motions(elf: &[u8], clips: &[Option<String>]) -> Result<Vec<Option<[String; 2]>>> {
    let sets = moves::roster(elf)?;
    let clip = |set: usize, alias: usize| -> Result<Option<String>> {
        let v = u16_at(elf, moves::ALIASES + 2 * (set * moves::ALIAS_COUNT + alias), "the movesets")? as usize;
        Ok(clips.get(v).cloned().flatten())
    };
    (0..moves::BODIES)
        .map(|c| match sets.get(c).map(|s| s.1).filter(|&set| set < moves::MOVESETS) {
            Some(set) => Ok(clip(set, 0)?.zip(clip(set, 3)?).map(|(a, b)| [a, b])),
            None => Ok(None),
        })
        .collect()
}

/// roster.json `front_end` (docs/GAMEDATA.md "front_end"), from the executable's tables (FRONTEND.md 2-8); `ups`:
/// file 3's uploads (the HUD and select atlas), which name its pictures; `clips`: each move's clip (the select's
/// characters' motions).
pub fn front_end(elf: &[u8], ups: &[Upload], clips: &[Option<String>]) -> Result<Value> {
    let sprite = |k: usize| sprite(ups, k);
    let a = |addr: usize| addr - VADDR;
    let what = "the front end's tables";
    let u8s = |addr: usize, n: usize| -> Result<Vec<u8>> { (0..n).map(|i| u8_at(elf, a(addr) + i, what)).collect() };
    let i32_at = |addr: usize| -> Result<i32> { Ok(u32_at(elf, a(addr), what)? as i32) };
    let s16 = |addr: usize| -> Result<i16> { Ok(u16_at(elf, a(addr), what)? as i16) };
    // the main menu 0x51EFE0: {label*, id, locked, help, 0}
    let mut menu = vec![];
    for i in 0..12 {
        let r = 0x51EFE0 + 8 * i;
        menu.push(json!({"mode": u8_at(elf, a(r) + 4, what)?, "label": string(elf, u32_at(elf, a(r), what)?),
            "locked": u8_at(elf, a(r) + 5, what)? != 0, "colour": u8_at(elf, a(r) + 6, what)?}));
    }
    let titles: Vec<String> =
        (0..9).map(|m| u32_at(elf, a(0x51EC18 + 12 * m), what).map(|p| string(elf, p))).collect::<Result<_>>()?;
    // the select grid 0x440A68 {slot, up, down, left, right, visible, body, x, y} and slot -> character 0x520460
    // (55: RANDOM)
    let mut slots = vec![];
    for i in 0..36 {
        let r = 0x440A68 + 12 * i;
        let b = u8s(r, 5)?;
        let character = u8_at(elf, a(0x520460) + i, what)? as i8;
        let n = |k: usize| b.get(k).copied().filter(|&v| v < 36);
        slots.push(json!({"slot": b.first(), "up": n(1), "down": n(2), "left": n(3), "right": n(4),
            "x": s16(r + 8)?, "y": s16(r + 10)?, "character": (0..34).contains(&character).then_some(character)}));
    }
    let set = |addr: usize| -> Result<Vec<usize>> {
        Ok(u8s(addr, 34)?.iter().enumerate().filter(|e| *e.1 != 0).map(|e| e.0).collect())
    };
    // {x, y, z, yaw} rows
    let places = |addr: usize, n: usize| -> Result<Vec<Vec<i32>>> {
        (0..n).map(|k| (0..4).map(|o| i32_at(addr + 16 * k + 4 * o)).collect()).collect()
    };
    let faces: Vec<String> =
        (0..BODIES).map(|b| i32_at(0x520508 + 4 * b).map(|id| sprite(id as usize))).collect::<Result<_>>()?;
    let mut portraits = vec![String::new(); BODIES];
    for (k, &b) in PORTRAITS.iter().enumerate() {
        if let Some(p) = portraits.get_mut(b) {
            *p = format!("{:03}_000.png", 143 + k);
        }
    }
    // the cursor tags' x by side (0x5205D8: {x on 1P's side, x on 2P's, sprite} by tag)
    let tag_dx = [i32_at(0x5205D8)?, i32_at(0x5205D8 + 4)?];
    let streaks = u8s(0x51F678, BODIES)?;
    let mut plates = vec![];
    for b in 0..BODIES {
        let (id, x) = (i32_at(0x5206B8 + 8 * b)?, i32_at(0x5206B8 + 8 * b + 4)?);
        plates.push(json!({"picture": sprite(id as usize), "x": x, "streak": streaks.get(b)}));
    }
    // the arcade: rival teams per character 0x51F3C8 (34: any), the fallback 0x51F560, the CPU pairs 0x5207F0
    let rivals: Vec<Vec<Option<i32>>> = (0..34)
        .map(|c| (0..3).map(|k| i32_at(0x51F3C8 + 12 * c + 4 * k).map(|v| (0..34).contains(&v).then_some(v))).collect())
        .collect::<Result<_>>()?;
    let fallback: Vec<i32> = (0..4).map(|k| i32_at(0x51F560 + 4 * k)).collect::<Result<_>>()?;
    let mut pairs = vec![];
    for i in 0..76 {
        let p = u8s(0x5207F0 + 5 * i, 5)?;
        if let (Some(&x), Some(&y)) = (p.first(), p.get(2)) {
            pairs.push(json!([x, y]));
        }
    }
    // the unlock schedule 0x440818 {u16 stage, u16 kind, u32 character} to stage 0xFFFF
    let mut schedule = vec![];
    for i in 0..64 {
        let r = 0x440818 + 8 * i;
        let stage = u16_at(elf, a(r), what)?;
        if stage == 0xFFFF {
            break;
        }
        schedule.push(
            json!({"stage": stage, "kind": u16_at(elf, a(r) + 2, what)?, "character": u32_at(elf, a(r) + 4, what)?}),
        );
    }
    // the options 0x439CE0 (0x40 bytes: {config byte*, default, label*, 0, value names*[6]}), the game options' first 12
    let mut options = vec![];
    for i in 0..12 {
        let r = 0x439CE0 + 0x40 * i;
        let values: Vec<String> = (0..6)
            .map(|k| u32_at(elf, a(r) + 16 + 4 * k, what))
            .collect::<Result<Vec<u32>>>()?
            .into_iter()
            .filter(|&p| p != 0)
            .map(|p| string(elf, p))
            .collect();
        options.push(json!({"label": string(elf, u32_at(elf, a(r) + 8, what)?), "config": u32_at(elf, a(r), what)? as usize - 0x439C60,
            "default": u32_at(elf, a(r) + 4, what)?, "values": values}));
    }
    Ok(json!({
        "screen": [640, 448],
        // (the fonts draw @ as ©)
        // (0x3BCC88: from white, then the logo and the blinking line until the demo; 0x3BC998 the line, the second of
        // 0x51EE68's, which a connected pad picks; 0x3C4930 the namco logo)
        "title": {"picture": "134_003.png", "press_start": string(elf, 0x51EE78),
            "copyright": [string(elf, 0x51EF40).replace('@', "©"), string(elf, 0x51EF70)], "copyright_y": [409, 429],
            "press_y": 329, "blink_mask": 0x30, "from_white": 43, "namco": {"picture": "134_000.png", "x": 257, "y": 350},
            // (the small font, from the left x, y the text's foot: the line at 336 - 16 less its width 0xD8 (0x3BC868),
            // the copyright at 0x24 and 0x123 (0x3BCC0C); their colours as the PS2 shows them: the code's 5 and 11 are
            // the font's palettes 31 and 0 [M])
            "press_x": 120, "press_colour": 31, "copyright_x": [0x24, 0x123], "copyright_colour": 0},
        // the sound ids the front end plays (group 0: 3 cursor, 4 select 1, 5 select 2; FRONTEND.md 9)
        "sounds": {"cursor": 3, "start": 4, "decide": 5, "cancel": 3, "pick": 4, "last_pick": 5},
        // the chrome font: 12 glyphs of 52 x 42 a row (read off the picture, not the code)
        "fonts": {"big": {"picture": sprite(2), "glyph": [52, 42], "rows": ["0123456789AB", "CDEFGHIJKLMN", "OPQRSTUVWXYZ", "!?.-':\"%<&()"],
            "colours": (0..FONT_COLOURS).map(|c| format!("003_001_c{c:02}.png")).collect::<Vec<_>>()},
            // the small font: 21 glyphs of 16 x 17 a row, in ASCII order with gaps (read off the picture), 16 apart
            // whatever their ink, the space too (as the PS2 spaces the title's line) [M]
            "small": {"picture": sprite(4), "glyph": [16, 17], "rows": ["!\"#$%&'()*+-,.0123456", "789:;<=>?@ABCDEFGHIJK", "LMNOPQRSTUVWXYZ[\\]/_"],
            "advance": 16,
            "colours": (0..FONT_COLOURS).map(|c| format!("003_002_c{c:02}.png")).collect::<Vec<_>>()}},
        // (0x3BD2E0: five rows about the cursor, the band behind it, the fades, the arrows; FRONTEND.md 9)
        "menu": {"address": "0x51EFE0", "items": menu, "rows": 5, "y": 323, "row_h": 26, "scroll": 4, "clip": [315, 417],
            "fades": [[315, 354], [380, 419]], "band": {"y": [353, 380], "colour": [192, 192, 255], "alpha_step": 8},
            "arrows": {"pictures": [sprite(129), sprite(130)], "x": [183, 445], "y": 358, "period": 24}, "idle": 480,
            "logo_step": 4, "dim": 64},
        // (QUICK SELECT off, the default: the faces at the slots' places, the lightning, the frame; on: 52 lower)
        "select": {"titles": titles, "slots": slots, "face_dy": 0, "hidden_slots": [34, 35],
            "lightning": {"pictures": (0..45).map(|k| format!("141_{k:03}.png")).collect::<Vec<_>>(), "rect": [0, 180, 640, 300], "level": 0x70},
            "frame": {"picture": "141_045.png", "size": [320, 309]}, "face_size": 51,
            "face_level": {"normal": 0x80, "hover": 0xA0, "picked": 0x30}, "cursors": [sprite(115), "003_057_p2.png"],
            "cursor_rect": [-1, -1, 55, 53], "cursor_pulse": {"low": 96, "high": 240, "step": 6}, "tag_dx": tag_dx, "tag_dy": 40,
            "random_roll": 3, "plates": [[[138, 264], [41, 238]], [[376, 264], [475, 238]]],
            "timer": {"frames": 2400, "x": 320, "y": 260, "blink": {"below": 6, "period": 30, "off": 8}}, "done": 90,
            // (1P's cursor starts on Xiaoyu's slot, as the PS2 shows it; 2P's on Jin's, the other end of the row [L])
            // (0x3CC660: "1ST" while the side has no pick, not in mode 5; 0x3CB4B4: "2ND" once it has, not in modes 5
            // and 6, while its partner's place stands nobody (select +0x14 + 4 x side) and `second_after` frames after
            // the pick (the side's +0x18, set by 0x3CA9B4))
            "start_slots": [18, 27], "ordinal": {"text": string(elf, 0x5205C8), "x": [62, 480], "y": 350,
                "second": string(elf, 0x5204B8), "second_after": 90},
            // (BGM 0 from the select's frame 18, select +0x10; it stops as the select ends: the VS screen is silent, as
            // the PS2 plays it)
            "music": {"bgm": 0, "from": 18},
            // (0x3CC4A4 -> 0x3BC828: the side nobody plays, while frame & 0x30: INSERT COIN (0x3BCA30, 0x51EE98), or
            // the credits (0x3BC650, 0x51EF00, its leading space kept) while frame & 0x40; 0x3BC868: centred on 160 +
            // 320 x side by a width of 12.8 a letter (16 x 4 / 5: size 4; the PS2 shows it there) [M], its foot
            // at y 400, the small font; the code passes colour 0x11, the PS2 shows the green of palette 29 [M])
            "insert": {"texts": [string(elf, 0x51EE98), format!(" {}", string(elf, 0x51EF00))], "x": [160, 480], "y": 400,
                "colour": 29, "show_mask": 0x30, "second_mask": 0x40, "measure": 12.8},
            "faces": faces, "random_face": sprite(162), "locked_face": sprite(165), "cursor": sprite(174), "cursor_dxy": [-2, -2],
            "tags": [sprite(119), sprite(121), sprite(123), sprite(124), sprite(125), sprite(126), sprite(127)],
            "selectable": set(0x51E850)?, "copy_pool": set(0x51E8A0)?, "portraits": portraits,
            // (0x3AE6E0: the camera, the projection's entry 24 of table 0x507FE0; 0x3AEF88: each model's place {x, y, z,
            // yaw} by slot (side + 2 * member) from 0x437D40, 1 ON 1's (mode 5) from 0x437D80; 0x3AE7C8: arena 22's light
            // table (0x363630(0x16)); 0x3AEA40: alias 0 looped,
            // a pick plays alias 3, then alias 0)
            "models": {"camera": {"eye": [0, -1152, 4608], "pitch": -128, "yaw": -2048, "h": s16(0x507FE0 + 10 * 24)?},
                "places": places(0x437D40, 4)?, "places_one_on_one": places(0x437D80, 2)?, "one_on_one_mode": 5, "light": 0x16,
                "motions": select_motions(elf, clips)?}},
        // the VS / loading screen (0x3C45B0's machine, drawn by 0x3C3FB8): the fire 142_000 (ids 0xF5 / 0xF6) over the
        // screen; each side's portraits (0x3C3948: 188 x 256, their edges faded over `feather` by 0x3C3370, 2P's
        // mirrored by 0x3C3648) at [x, dx, y, dy] + step x [dx, dy], the step 1 a frame to `steps` (state 3), then held
        // while the fight loads; TEAM BATTLE's (mode 2) up by `team_dy`; the partner over its leader; "VS" 142_001
        // (0xF7 / 0xF8) centred at y 340 in the VS modes (gp-0x7440 and -0x743E), else the stage's texts
        "vs": {"background": "142_000.png", "logo": "142_001.png", "logo_y": 340, "steps": 10, "portrait": [188, 256],
            "feather": 20, "places": [[[-257, 38, 20, 3], [-360, 38, 216, -3]], [[708, -38, 20, 3], [810, -38, 216, -3]]],
            "team_dy": [-22, -90],
            // (0x3C410C, by mode, table 0x51F740: modes 0, 3, 5 "STAGE" and the fight's number, the eighth "FINAL" over
            // "STAGE"; 2 and 4 "FIGHT" and the number; 7 "PRACTICE"; none in 1 and 6. The printf's size (7, 8, 10) as a
            // scale of the chrome font's 42-pixel glyphs (size / 9.6), the lines' tops (from its y 350 and 385) and the colour
            // (its %c 14: palette 14, silver edged in red) are as the PS2 shows them [M])
            "texts": {"stage": string(elf, 0x51F6C0), "final": string(elf, 0x51F6F0), "fight": string(elf, 0x51F700),
                "practice": string(elf, 0x51F710), "final_number": 8, "stage_modes": [0, 3, 5], "fight_modes": [2, 4],
                "practice_modes": [7], "tops": [330, 366], "scales": {"text": 0.73, "final": 0.83, "number": 1.04}, "colour": 14}},
        "hud": {"bars": {"x": [29, 370], "y": 30, "w": 240, "h": 28},
            "strips": (0..6).map(|i| sprite(45 + 2 * i)).collect::<Vec<_>>(),
            "angry_strips": (6..12).map(|i| format!("003_{i:03}_angry.png")).collect::<Vec<_>>(), "angry_at": 120, "angry_blink": 8,
            "trail_per_frame": [800, 400],
            "plates": plates, "plate_y": 58, "streaks": [sprite(57), sprite(59)],
            "lamps": {"lit": sprite(67), "unlit": sprite(69), "x": [250, 364], "dx": 22, "y": 60, "blink": [180, 11]},
            "team_icon": {"picture": sprite(73), "x": [272, 348], "dx": 16, "y": 60},
            "chicken": {"picture": sprite(71), "x": [13, 595], "y": 400},
            "timer": {"digits": sprite(0), "digit_w": 40, "y": 8, "infinity": sprite(7)},
            "stage": {"text": string(elf, 0x51ED10), "final": string(elf, 0x51ED28), "alternate": 32}},
        "arcade": {"fights": 8, "rivals": rivals, "fallback": fallback, "pairs": pairs, "rival_arena": 6,
            "boss": 33, "boss_arena": 14, "boss_rounds": 1, "early_fights": 3, "early_excluded": [14, 15, 20, 29],
            "continue": {"frames": 898, "digit_frames": 100}, "time_format": string(elf, 0x51ED40)},
        "team_battle": {"members": [1, 8], "refill_percent": 15},
        "unlocks": {"address": "0x440818", "schedule": schedule, "kinds": {"0": "cpu_pool", "1": "selectable", "3": "reset",
            "4": "gallery", "5": "tekken_bowl", "6": "last"}, "points_per_stage": 50},
        "options": options,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn sprites_name_their_pictures() {
        // an image, its 16-colour palette, an image: the second is picture 1
        let up = |psm, w, h| Upload { set: 0, dbp: 0, dbw: 64, psm, w, h, data: vec![] };
        let ups = [up(0x14, 32, 32), up(0, 8, 2), up(0x14, 32, 32)];
        assert_eq!((sprite(&ups, 0), sprite(&ups, 2)), ("003_000.png".to_string(), "003_001.png".to_string()));
        assert_eq!(
            (
                palette(3, 140),
                palette(3, 80),
                palette(141, 5),
                palette(141, 0),
                palette(3, 45),
                palette(3, 162),
                palette(3, 129)
            ),
            (Some(132), Some(76), Some(0), None, None, Some(163), Some(40))
        );
        assert_eq!(PORTRAITS.iter().collect::<std::collections::BTreeSet<_>>().len(), BODIES);
        assert!(is_2d(3) && is_2d(182) && !is_2d(46) && !is_2d(129));
        // the executable's tables outside the data: an error, not a panic
        assert!(front_end(&[0; 64], &ups, &[]).is_err());
    }
}

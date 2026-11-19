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

/// Entry `n` of the executable's archive 0x457380 (u32 count, then {u32 offset, u32 size} from the archive's start;
/// 0x364528): 0 the help line's font and its palette (GIF uploads), 2 the English help texts.
fn archive_entry(elf: &[u8], n: usize) -> Option<&[u8]> {
    let base = 0x457380 - VADDR;
    let at = |k: usize| u32_at(elf, base + k, "the help archive").ok().map(|v| v as usize);
    elf.get(base + at(4 + 8 * n)?..)?.get(..at(8 + 8 * n)?)
}

/// The help line's font (FRONTEND.md 8: 0x3DEA50 uploads archive entry 0, a 256 x 180 4-bit picture and its palette,
/// to the GS at 0x3B40, 4 rows down; 0x3DEDD8 draws a glyph from it): `help_font.png` and its PNG.
pub fn help_font(elf: &[u8]) -> Option<(String, Vec<u8>)> {
    let (w, h, px) = tex::pictures(&tex::uploads(archive_entry(elf, 0)?), &|_| None).into_iter().next()?;
    Some(("help_font.png".to_string(), tex::png(w, h, &px)))
}

/// The help texts (archive entry 2, 0x45D780: NUL-separated, as 0x3E4FA0 counts them), up to the first "null"; bytes
/// past ASCII (the font's button glyphs) as the characters of the same number.
fn help_texts(elf: &[u8]) -> Vec<String> {
    let e = archive_entry(elf, 2).unwrap_or_default();
    e.split(|&b| b == 0)
        .map(|s| s.iter().map(|&b| char::from(b)).collect::<String>())
        .take_while(|s| s != "null")
        .collect()
}

/// A picture of file 3 by its upload number among `ups` (its uploads): `003_NNN.png`, NNN its place among the
/// images, palettes not counted (sprites.py).
fn sprite(ups: &[Upload], k: usize) -> String {
    format!("003_{:03}.png", ups.iter().take(k).filter(|u| !tex::is_palette(u)).count())
}

/// A printf format of the executable with its place, font, colour and size codes (`%H%V%F%c%S%P%T%Z`) left out and
/// its numbers (`%d`, `%2d`, `%02d`) kept.
fn format(elf: &[u8], pointer: u32) -> String {
    let at = (pointer as usize).wrapping_sub(VADDR);
    let raw: Vec<u8> = elf.get(at..).unwrap_or_default().iter().take(96).take_while(|&&c| c != 0).copied().collect();
    let s = String::from_utf8_lossy(&raw).to_string();
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('%', Some(&n)) if n.is_ascii_uppercase() || n == 'c' => {
                chars.next();
            }
            _ => out.push(c),
        }
    }
    out
}

/// A string of the executable as it is (codes like `%H%V` left out), trimmed.
fn string(elf: &[u8], pointer: u32) -> String {
    raw_string(elf, pointer).trim().to_string()
}

/// A string of the executable with its spaces (codes like `%H%V` left out).
fn raw_string(elf: &[u8], pointer: u32) -> String {
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
    out
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
    // the small font's printf colour n (%c) -> its palette (0x3C68A8: table 0x51FE98, a sprite id of file 3 a colour) as
    // `fonts.small.colours`' index (palette uploads 9-44); one not among them (colour 11's upload 0, an image) shows
    // white on the PS2: colour 0's
    let small_colour = |n: usize| -> Result<usize> {
        Ok((u32_at(elf, a(0x51FE98 + 4 * n), what)? as usize).checked_sub(9).filter(|&c| c < FONT_COLOURS).unwrap_or(0))
    };
    // a round text: its format, x (the left), y (the foot), chrome colour, printf size (10: the glyphs' own)
    let banner = |p: u32, x: i32, y: i32, colour: usize, size: i32| -> Value {
        json!({"text": format(elf, p), "x": x, "y": y, "colour": colour, "size": size})
    };
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
    // the options 0x439CE0 (0x40 bytes: {config byte*, default, label*, kind, value names*[11], s16 arrow x[2]}), the
    // GAME OPTIONS page's 14 rows: kind 0 a list, 1 a number 0-10 (0 OFF), 2 DEFAULT and EXIT (no config); the arrows'
    // x (-1 none: a two-value list has only the right one) by 0x3B4440; the values as printed, a list of more than two
    // with its leading space (" NORMAL", " 60"); `modes` (+0x38): the main menu entries (bit = id) it applies to,
    // shown on its row (0x3B3F88)
    let mut options = vec![];
    for i in 0..14 {
        let r = 0x439CE0 + 0x40 * i;
        let values: Vec<String> = (0..6)
            .map(|k| u32_at(elf, a(r) + 16 + 4 * k, what))
            .collect::<Result<Vec<u32>>>()?
            .into_iter()
            .filter(|&p| p != 0)
            .map(|p| raw_string(elf, p))
            .collect();
        let config = (u32_at(elf, a(r), what)? as usize).checked_sub(0x439C60);
        options.push(json!({"label": string(elf, u32_at(elf, a(r) + 8, what)?), "config": config,
            "default": u32_at(elf, a(r) + 4, what)?, "values": values, "kind": u8_at(elf, a(r) + 12, what)?,
            "arrows": [s16(r + 0x3C)?, s16(r + 0x3E)?], "modes": u32_at(elf, a(r) + 0x38, what)?}));
    }
    // the records' defaults (0x3C88A0, a reset): TIME ATTACK's 10 times 99'00"00 (356400 frames) by the pairs of
    // 0x520388, SURVIVAL's 10 by the pairs of 0x5203A0 with (9 - n) / 2 + 1 wins, each named by its first body's
    // name's first 3 letters (0x3D01C0: body -> 0x415CC0 -> its name)
    let name3 = |body: u8| -> Result<String> {
        let p = u32_at(elf, a(0x415CC0) + 4 * usize::from(body), what)?;
        let p = u32_at(elf, (p as usize).wrapping_sub(VADDR), what)?;
        Ok(string(elf, p).chars().take(3).collect())
    };
    let defaults = |addr: usize, score: &dyn Fn(usize) -> u32| -> Result<Vec<Value>> {
        let b = u8s(addr, 20)?;
        b.chunks(2)
            .enumerate()
            .map(|(n, m)| Ok(json!({"members": m, "score": score(n), "name": name3(m.first().copied().unwrap_or(0))?})))
            .collect()
    };
    let default_times = defaults(0x520388, &|_| 356_400)?;
    let default_survivors = defaults(0x5203A0, &|n| (9 - n as u32) / 2 + 1)?;
    Ok(json!({
        "screen": [640, 448],
        // (the fonts draw @ as ©)
        // (0x3BCC88: from white, then the logo and the blinking line until the demo; 0x3BC998 the line, the second of
        // 0x51EE68's, which a connected pad picks; 0x3C4930 the namco logo)
        "title": {"picture": "134_003.png", "press_start": string(elf, 0x51EE78),
            "copyright": [string(elf, 0x51EF40).replace('@', "©"), string(elf, 0x51EF70)], "copyright_y": [409, 429],
            "press_y": 329, "blink_mask": 0x30, "from_white": 43, "namco": {"picture": "134_000.png", "x": 257, "y": 350},
            // (the small font, from the left x, y the text's foot: the line at 336 - 16 less its width 0xD8 (0x3BC868),
            // the copyright at 0x24 and 0x123 (0x3BCC0C); the code's colours 5 and 11)
            "press_x": 120, "press_colour": small_colour(5)?, "copyright_x": [0x24, 0x123], "copyright_colour": small_colour(11)?},
        // the sound ids the front end plays (group 0: 3 cursor, 4 select 1, 5 select 2; FRONTEND.md 9)
        "sounds": {"cursor": 3, "start": 4, "decide": 5, "cancel": 3, "pick": 4, "last_pick": 5},
        // GAME OPTIONS' BGM VOLUME 0-10: the music's level of 128 (0x39A1A8: table 0x518E58); SE VOLUME: an effect's
        // volume x n / 10 (0x375044); SPEAKER OUT MONO sent to the sound driver (0x39A208)
        "volumes": {"bgm": (0..11).map(|k| u16_at(elf, a(0x518E58) + 2 * k, what)).collect::<Result<Vec<_>>>()?, "se": 10},
        // the chrome font: 12 glyphs of 52 x 42 a row (read off the picture, not the code)
        "fonts": {"big": {"picture": sprite(2), "glyph": [52, 42], "rows": ["0123456789AB", "CDEFGHIJKLMN", "OPQRSTUVWXYZ", "!?.-':\"%<&()"],
            "colours": (0..FONT_COLOURS).map(|c| format!("003_001_c{c:02}.png")).collect::<Vec<_>>(),
            // (0x3C69D8: each glyph's advance at size 10 from table 0x51FF08, by its place in the font's characters
            // 0x51FEC8 (one not among them takes the first's); an A is drawn 6 further left, and what follows it too;
            // a size s scales both by s / 10, the glyph drawn 52 x 42 x s / 10 from its foot y less 20 (0x3C7080))
            "advance": u8s(0x51FF08, 48)?, "kern_a": 6, "foot_dy": -20},
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
            // at y 400, the small font, colour 3 (0x3CC4A4's a3))
            "insert": {"texts": [string(elf, 0x51EE98), format!(" {}", string(elf, 0x51EF00))], "x": [160, 480], "y": 400,
                "colour": small_colour(3)?, "show_mask": 0x30, "second_mask": 0x40, "measure": 12.8},
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
        // (0x3C2F40 per side: the member in back first (0x3C23F0 with a3 0: 15 up and out, colour level 0x50, 0x70
        // angry), the one in front over it (a3 1: level 0x80, its name plate); a tag swaps them over 30 frames, the
        // offsets n / 2 and 15 - n / 2 as n counts down; the strips 0x3CE318 without blending, the caps 9 wide; from
        // the bar's inner end: health, the trail of what it lost (dropping max / 400 a frame), the recoverable cap
        // (red), then empty to the outer end; FRONTEND.md 4)
        "hud": {"bars": {"x": [29, 370], "y": 30, "w": 240, "h": 28, "cap_w": 9, "back": 15, "levels": [0x50, 0x80],
                "angry_level": 0x70, "swap": 30},
            "strips": (0..6).map(|i| sprite(45 + 2 * i)).collect::<Vec<_>>(),
            "angry_strips": (6..12).map(|i| format!("003_{i:03}_angry.png")).collect::<Vec<_>>(), "angry_at": 120, "angry_blink": 8,
            "trail_per_frame": [800, 400],
            // (0x3C1DA0: the front member's streak, 124 x 34 added to what is behind (draw mode 4, 0x3CE398), and its plate
            // (0x3CF1C0, the table's x not added) at the bar's outer end less 9 (side 0) or at 490 (side 1), y 58)
            "plates": plates, "plate_y": 58, "plate_x": [20, 490], "streaks": [sprite(57), sprite(59)], "streak_size": [124, 34],
            // (0x3C1F08: 8 higher (+0x24) when the side has no partner)
            "lamps": {"lit": sprite(67), "unlit": sprite(69), "x": [250, 364], "dx": 22, "y": 60, "blink": [180, 11], "solo_dy": -8},
            "team_icon": {"picture": sprite(73), "x": [272, 348], "dx": 16, "y": 60},
            "chicken": {"picture": sprite(71), "x": [13, 595], "y": 400},
            // (0x3C6738 at (320, 6): the tens 39 left of the ones, 40 x 47; INFINITY at x 285)
            "timer": {"digits": sprite(0), "digit_w": 40, "y": 6, "x": 320, "tens_dx": -39, "infinity": sprite(7), "infinity_x": 285},
            "stage": {"text": string(elf, 0x51ED10), "final": string(elf, 0x51ED28), "alternate": 32},
            // (0x3BC540 -> 0x3BC048 per side, feet at y 24, the small font: the player's side in the ladder modes STAGE
            // (0x3BBF00: x 15 or 370, colour 0; " FINAL" from the eighth fight while frame & 0x20 is clear) and the
            // run's time 130 right (colour 5); the side nobody plays INSERT COIN / CREDIT as the select's (0x3BC828 at
            // x 160 or 480, colour 3), in Time Attack TIME ATTACK (x 408, colour 6))
            "top": {"y": 24, "stage_x": [15, 370], "stage": format(elf, 0x51ED10), "final": format(elf, 0x51ED28),
                "stage_colour": small_colour(0)?, "time_dx": 130, "time": format(elf, 0x51ED40), "time_colour": small_colour(5)?,
                "insert_x": [160, 480], "insert_colour": small_colour(3)?, "time_attack": format(elf, 0x51EDE0),
                "label_x": [56, 408], "label_colour": small_colour(6)?,
                // (under CONTINUE?, the player's side: PRESS Pn START, then the credits, as INSERT COIN alternates, seen
                // in PCSX2; 0x51EED0 / 0x51EEC0, 0x51EEF0)
                "press": [string(elf, 0x51EED0), string(elf, 0x51EEC0)], "free_play": format(elf, 0x51EEF0)},
            // (0x36EA88's table 0x500440 by round state: 0 the call (0x36EDE0: ROUND n on the first round's frames 66-104,
            // the others' 6-44, READY! from 106 / 46; TAG BATTLE for ROUND in a one-round match of pairs), 2 the end
            // (0x36EF58 by the result flags: TIME UP, PERFECT!, DOUBLE K.O., else K.O. at size 12), 4 the pose (0x36F0D8:
            // DRAW, YOU WIN! / YOU LOSE for a human against the CPU); the chrome font, x the left, y the foot; no FIGHT)
            "banners": {"round": banner(0x500488, 0xA5, 0xDC, 2, 10), "tag_battle": banner(0x500470, 0x74, 0xDC, 2, 10),
                "ready": banner(0x5004A0, 0xC0, 0xDC, 2, 10), "time_up": banner(0x5004B0, 0xBC, 0xDC, 12, 10),
                "perfect": banner(0x5004C0, 0xAA, 0xDC, 11, 10), "double_ko": banner(0x5004D8, 0x72, 0xDC, 12, 10),
                "ko": banner(0x5004F0, 0xF5, 0xDC, 12, 12), "draw": banner(0x500500, 0xE8, 0xDC, 12, 10),
                "you_win": banner(0x500520, 0x9D, 0x168, 2, 10), "you_lose": banner(0x500558, 0x98, 0x168, 12, 10)}},
        "arcade": {"fights": 8, "rivals": rivals, "fallback": fallback, "pairs": pairs, "rival_arena": 6,
            "boss": 33, "boss_arena": 14, "boss_rounds": 1, "early_fights": 3, "early_excluded": [14, 15, 20, 29],
            // (0x3BA560: CONTINUE? and the digit at (86, 360) in the chrome font's own colour, over the fight's scene and
            // HUD; at 0, 0x35B0E4: GAME OVER at (123, 360) colour 1 for 150 frames, voice 0x1056, then the CHARACTER
            // USAGE ranking (screen 7) and the title)
            "continue": {"frames": 898, "digit_frames": 100, "text": format(elf, 0x51E9F8), "x": 86, "y": 360,
                "game_over": {"text": format(elf, 0x4FB9F8), "x": 123, "y": 360, "colour": 1, "frames": 150, "voice": 0x1056}},
            "time_format": string(elf, 0x51ED40)},
        "team_battle": {"members": [1, 8], "refill_percent": 15},
        "unlocks": {"address": "0x440818", "schedule": schedule, "kinds": {"0": "cpu_pool", "1": "selectable", "3": "reset",
            "4": "gallery", "5": "tekken_bowl", "6": "last"}, "points_per_stage": 50},
        "options": options,
        // OPTIONS (screen 9; FRONTEND.md 8): the menu 0x3B19B0, its title at (193, 40) size 5 colour 19, the six entries
        // of 0x439CC8 from (40, 90) 30 apart, size 4, colour 5 (the cursor's 11), each over a line 0x4402B0 (4 high, 24
        // above the foot, blue (128, 128, 255) at alpha 0x60 from x 40 fading to 400), the cursor's a band 0x440100 (24
        // high, x 0 to 40 to 640, green (0, 255, 128) from 0 to 0x60 to 0) and the line white; help text = entry. GAME
        // OPTIONS (0x3B42A0): title at (194, 40), the rows of `options` from (40, 90) 20 apart, size 4, "%s:" then the
        // value in colour 16 (the default) or 12, a number 0 as OFF; the cursor's label 11 over the pause's bar
        // (0x3B15F8, x 0 to 40 to 170 past "%s:", colour table 0x43A148's first); the arrows on the cursor's row: a
        // list's left one always, a number's while above 0 and the right one while below 10; DEFAULT's YES / NO at x 200
        // (the chosen 11, the other 22); help text 6 + row, its \S alternative the value. On the cursor's row the main
        // menu's entries ARCADE..TEKKEN BOWL (Bowl once open) right-aligned at x 600 from foot 250, 17 apart, size 4, in
        // their colours where the option's `modes` bit is set, else 22, over a box 0x43FF50 (x 360 to 608, y 226 to 16
        // above the next foot, black from alpha 0 to 0x80 rightwards). `backdrop`: the screen's blue (0x3BF2F8: table
        // 0x440748's colour 11, (0, 0, 255), from black at x 0 and 640 to it in the middle, all at alpha 0x40 over the
        // dimmed logo, as the PS2 shows it: the logo at a quarter, the middle 132 blue)
        // RECORDS (0x3B2978): "RECORDS" at (243, 40); left / right turn the pages (CHARACTER USAGE 0x3B2090, GREATEST
        // SURVIVORS 0x3B2358, TIME ATTACK 0x3B2670), their names at (x, 70) size 4 between the arrows at x 153 and 483
        // blinking every 24 frames (0x3B1F10); 6 rows from foot 130, 50 apart, each over a bar (0x3B15F8 with 26
        // spaces, 26 x 19.2 + 170 = 669 past x 40: the colour table 0x43A148's entry 1, 2, 3); the rank (size 5 at x 70, 4 up) and its suffix (0x3B1EC8:
        // ST, ND, RD, TH; size 4), the faces 40 x 42 at x 160 and 210, 46 up; then the columns {x, dy from the foot,
        // size, colour}: the name and "%3d.%1d%%" of the picks; the wins (999 at most), WIN / WINS, the name; the time,
        // the name. Help text 24 + page. `more`: "u" / "d" (0x3B1F78, "%H%V%cu" at (40, 120) / (40, 152), the feet,
        // blinking with the page's arrows) where rows are above / below, the button icons' up and down (`003_003`'s 16 x 16
        // cells 9 and 8) in yellow, as seen. `defaults`: the records a new save starts with. CHARACTER USAGE lists the
        // selectable characters, most picked first (by id among equals), 0.0 % with none picked (0x3C84B8). The name entry (0x3D0450): 3 letters of table 0x520BE8, round in 42
        "records": {"title": "RECORDS", "title_xy": [243, 40], "page_y": 70, "page_size": 4, "arrows_x": [153, 483],
            "arrows_blink": 24, "row_y": 130, "row_dy": 50, "rank_x": 70, "suffixes": ["ST", "ND", "RD", "TH"],
            "faces_x": [160, 210], "bar_w": 669, "help": 24, "defaults": {"times": default_times, "survivors": default_survivors},
            "more": {"picture": "003_003.png", "cells": [9, 8], "cell": 16, "x": 40, "y": [120, 152], "tint": [255, 255, 0]},
            "pages": [
                {"title": "CHARACTER USAGE", "x": 192, "colour": 14, "bar": [255, 0, 0, 0x60],
                    "columns": [{"x": 210, "dy": 0, "size": 4, "colour": 11}, {"x": 410, "dy": -6, "size": 5, "colour": 14}]},
                {"title": "GREATEST SURVIVORS", "x": 173, "colour": 15, "bar": [0, 255, 0, 0x60],
                    "columns": [{"x": 260, "dy": -4, "size": 5, "colour": 11}, {"x": 338, "dy": 0, "size": 4, "colour": 11},
                        {"x": 460, "dy": -6, "size": 5, "colour": 15}]},
                {"title": "TIME ATTACK", "x": 230, "colour": 6, "bar": [0, 192, 255, 0x60],
                    "columns": [{"x": 260, "dy": 0, "size": 4, "colour": 11}, {"x": 460, "dy": -6, "size": 5, "colour": 6}]}],
            "letters": string(elf, 0x520BE8).chars().take(42).collect::<String>(), "name_len": 3,
            "new_record": format(elf, 0x51FA58)},
        // the ranking (screen 7, 0x3C8A88) where a new TIME ATTACK or SURVIVAL record is named: the banner by kind
        // (0x3C93A0: sprites 0x520448 253, 257, 255 of file 132: TIME ATTACK, GREATEST SURVIVORS, CHARACTER USAGE) at (0, 20); rows clipped to y
        // 20-446 (0x3DBAA8), `dy` apart from foot `first` (gp-0x6264 / -0x625C: 93 from 153, 4 shown; CHARACTER USAGE 75
        // from 135, 5), `132_000` (sprite 251) across at each foot (0x3C9458); a row (0x3C75E8, 0x3C7AB0): the rank "%2d" at x 16, 27
        // above the foot, size 10, and its suffix size 5, 7 above (0x3C7278); the members' faces (0x3CBFB8, 51 x 51 in
        // the cursor frame `003_110` 2 out, 0x3CC078), the second first, and their name plates (0x3CF1E8: less the plate's `x`); TIME ATTACK's
        // time size 5 at (300, -8); SURVIVAL's "WINS" size 5 at (275, -7) then the number size 10 at -26 (0x3C7428; "WINS
        // " below 100); the name at (488, -28) size 10. Colours: the entry's row 11, the others 13, a time 10, wins 12.
        // The entry's row: a band over the row (0x3CE5D8, from 93 above the foot, grey at alpha 0x40), the name so far
        // with the letter under the cursor in its place, under the place a bar 48 x 4 at -8 (0x3CE5D8, red to white:
        // (255, 2t + 1, 2t + 1), t 5 to 113 and back by 5 a frame), the seconds left "%d" size 5 colour 0 at (480,
        // -54) of the `frames` the entry may take (0x3C9124: 3000, then the name is taken as it is)
        "ranking": {"banners": ["132_001.png", "132_003.png", "132_002.png"], "banner_xy": [0, 20], "clip": [20, 446],
            "rows": [{"dy": 93, "first": 153, "shown": 4}, {"dy": 93, "first": 153, "shown": 4}, {"dy": 75, "first": 135, "shown": 5}],
            "separator": "132_000.png", "rank": {"x": 16, "dy": -27, "suffix_dy": -7, "suffix_size": 5},
            "colours": {"row": 13, "entry": 11, "time": 10, "wins": 12}, "faces": [[160, -74], [210, -57]],
            "face_frame": sprite(0xAE), "face_size": 51, "plates": [[220, -78], [270, -60]],
            "time": {"x": 300, "dy": -8, "size": 5}, "wins": {"x": 275, "dy": -7, "size": 5, "number_dy": -26, "number_size": 10},
            "name": {"x": 488, "dy": -28}, "band": {"h": 93, "colour": [128, 128, 128, 0x40]},
            "cursor": {"dy": -8, "w": 48, "h": 4, "low": 5, "high": 113, "step": 5},
            "counter": {"x": 480, "dy": -54, "size": 5, "colour": 0}, "frames": 3000,
            // CHARACTER USAGE's row (0x3C7F60): the rank in colour 14, one face at (160, -57) and the plate at (220,
            // -54); the share (0x3C74A8) "%2d" size 10 at (470, -24) and ".%d%" size 5 at -4 ("100" and "%" at 1000);
            // the bar (0x3C7530 -> 0x3CFE38) 212 x share / 1000 wide from x 228, 36 high from 38 above the foot, from the
            // character's first colour to its second (table 0x520140, 6 bytes a character)
            "usage": {"rank_colour": 14, "face": [160, -57], "plate": [220, -54],
                "share": {"x": 470, "dy": -24, "small_dy": -4, "small_size": 5, "colour": 14},
                "bar": {"x": 228, "w": 212, "dy": -38, "h": 36,
                    "colours": u8s(0x520140, 6 * 34)?.chunks(6).map(|c| c.to_vec()).collect::<Vec<_>>()}}},
        "options_menu": {"title": format(elf, 0x51E0C8), "title_xy": [193, 40], "title_size": 5, "title_colour": 19,
            "items": (0..6).map(|k| u32_at(elf, a(0x439CC8) + 4 * k, what).map(|p| string(elf, p))).collect::<Result<Vec<_>>>()?,
            "item_xy": [40, 90], "item_dy": 30, "size": 4, "colours": {"item": 5, "cursor": 11},
            "line": {"x": [40, 400], "y": [-24, -20], "colour": [128, 128, 255, 0x60], "cursor": [255, 255, 255, 0x60]},
            "band": {"x": [0, 40, 640], "y": [-24, 0], "colour": [0, 255, 128, 0x60]}, "help": 0,
            "game": {"title": format(elf, 0x51E4A0), "title_xy": [194, 40], "row_xy": [40, 90], "row_dy": 20,
                "colours": {"label": 5, "cursor": 11, "value": 12, "default": 16, "off": 22}, "bar_past": 170,
                "yes_no_x": 200, "help": 6,
                "modes": {"right": 600, "y": 250, "dy": 17, "size": 4, "off": 22, "box": {"x": [360, 608], "y": 226, "below": -16}}},
            "backdrop": [0, 0, 255, 0x40]},
        // (0x3E5268 -> 0x3E06A0: help text n at (x, y) in the help font, 12 x 18 glyphs 16 a row (character - `first`;
        // the picture is uploaded 4 rows down, where the code counts its rows from), each advancing its byte of table 0x4572E0; a space 4 (0x3DF808: a line wider than `wrap` breaks and
        // spreads its spaces), `\n` a new line 20 lower, `\S[...]` the alternatives of which only the current value's is
        // drawn (gp-0x67BA); the dark band under it (0x3DBCF0, RGBA 0x18181860) when HELP MESSAGES is on (0x3B1468);
        // colour level 0x60)
        "help": {"font": {"picture": "help_font.png", "glyph": [12, 18], "columns": 16, "first": 32,
                "widths": u8s(0x4572E0, 160)?, "space": 4, "line_h": 20, "wrap": 0x244},
            "x": 30, "y": 390, "level": 0x60, "band": {"rect": [0, 389, 640, 48], "colour": [0x18, 0x18, 0x18, 0x60]},
            "texts": help_texts(elf)},
        // the pause menu (0x3B7B98): its items 0x51E7A8 (CANCEL, RESET, COMMAND LIST, SCREEN CAPTURE, TOP BOWLERS,
        // BOWLING OPTIONS, ADJUST DISPLAY, CONTROLLER SETUP; SCREEN CAPTURE once the Gallery is open, the Bowl's two only
        // in Tekken Bowl and COMMAND LIST and SCREEN CAPTURE not there); the window 0x3DC768 (x, y, w, h; drawn 8 out,
        // navy RGBA 0x00004040 over the scene, `h_capture` high with SCREEN CAPTURE or in the Bowl) and its frame; "PAUSE
        // Pn" blinking (frame & 0x30) in the chrome font, size 5, colour 0; the items from (180, 180) 20 apart, size 4,
        // colour 5, the cursor's 11 over a bar (0x3BB680: from x 180 to 40 past the text, 12 to 4 above its foot, green
        // (0, 255, 128) at alpha 0x60, 0 at its right end: the template 0x4402F8); help text 31 + item. RESET? (0x3B8460):
        // the window (240, 206, 160, 40) alone, the text at (270, 230) colour 19, YES / NO at (258, 250) size 4, the
        // chosen 11, the other 22, the printf's "{" / "}" (the gold arrows `003_067` / `003_068`, as seen) at
        // (320, 250) towards it; NO first; help 40
        "pause": {"items": (0..8).map(|k| u32_at(elf, a(0x51E7A8) + 4 * k, what).map(|p| string(elf, p))).collect::<Result<Vec<_>>>()?,
            "header": format(elf, 0x51E728), "header_xy": [233, 150], "header_size": 5, "window": [160, 130, 320, 130],
            "h_capture": 150, "colour": [0, 0, 0x40, 0x40], "item_xy": [180, 180], "item_dy": 20, "size": 4,
            "colours": {"item": 5, "cursor": 11, "off": 22}, "bar": {"dx": 40, "y": [-12, -4], "colour": [0, 255, 128, 0x60]},
            "help": 31, "reset": {"window": [240, 206, 160, 40], "text": format(elf, 0x51E800), "xy": [270, 230], "colour": 19,
                "yes_no": ["YES", "NO"], "yes_no_xy": [258, 250], "arrow_xy": [320, 250], "help": 40}},
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

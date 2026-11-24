//! The front end (docs/FRONTEND.md): the 2D files' palettes, and the tables OpenKumite's menus, select screen, HUD,
//! arcade ladder and unlocks read, as roster.json `front_end` (docs/GAMEDATA.md "front_end"). The pictures are the
//! 2D files' images as `assets/pictures/FFF_NNN.png`, the names `textures/` has; `front_end` names them.

use crate::fight;
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

/// Pictures of file 3 drawn in another palette upload: the select cursor (upload 115) in 2P's, upload 117 (0x3CC9E0:
/// 1P's is 116, the one after it), `003_057_p2.png`; the ranking's face frame (upload 174) in the faces' 0x84
/// (0x3CC078), `003_110_p84.png`. Each its file name and PNG.
pub fn repainted(ups: &[Upload]) -> Vec<(String, Vec<u8>)> {
    [("003_057_p2.png", 115, 117), ("003_110_p84.png", 174, 132)]
        .into_iter()
        .filter_map(|(name, k, pal)| {
            let pair = [ups.get(k)?.clone(), ups.get(pal)?.clone()];
            let (w, h, px) = tex::pictures(&pair, &|_| Some(1)).into_iter().next()?;
            Some((name.to_string(), tex::png(w, h, &px)))
        })
        .collect()
}

/// The text colours (the printf `%c`: palette upload 9 + colour, 36 of them) of the chrome font (upload 2 of file 3) and
/// the small one (upload 4) and the button icons (upload 6, the small font's lower case): `003_001_cNN.png`,
/// `003_002_cNN.png`, `003_003_cNN.png`, NN the colour, and their PNGs.
pub fn font_colours(ups: &[Upload]) -> Vec<(String, Vec<u8>)> {
    [(2, 1), (4, 2), (6, 3)]
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

/// A fighter file's command list (part 2, a pack: section 0 the help font again, 1-5 the list, the same text in each;
/// 0x3DC870 keeps section 1): u8 the count, the character's name, then the count's moves, each its name and its keys
/// (NUL-ended). A name's leading 0xFC n (n < 0xFE) starts colour group n - 1 (0x3DC9C8); the keys' 0xFC 0xFF aligns
/// each line right, 0xFF 'n' starts a line, 0xDB-0xFB are icons (0x3DFA00). `{name, moves: [[group, name, keys]]}`,
/// bytes as the characters of the same number.
pub fn command_list(part: &[u8]) -> Option<Value> {
    let text = *crate::archive::pack(part)?.get(1)?;
    let (&count, rest) = text.split_first()?;
    let mut strings = rest.split(|&b| b == 0).map(|s| s.iter().map(|&b| char::from(b)).collect::<String>());
    let name = strings.next()?;
    let mut group = 0;
    let mut moves = Vec::new();
    for _ in 0..count {
        let (mut label, keys) = (strings.next()?, strings.next()?);
        if let Some(n) =
            label.strip_prefix('\u{fc}').and_then(|l| l.chars().next()).map(u32::from).filter(|&n| n < 0xfe)
        {
            group = n.saturating_sub(1);
            label = label.chars().skip(2).collect();
        }
        moves.push(json!([group, label, keys]));
    }
    Some(json!({"name": name, "moves": moves}))
}

/// A fighter file's training data (part 3; 0x3672B0 -> 0x37D0F8): u16 n, u16 m, n entries of 33 u16, one a move of
/// the command list (+0 its steps, +2 its first step, +6 a CHAIN TRAINING entry, +8 a COMBO TRAINING one, +0xE a
/// time, +0x12 eight entries (1 + the move) shown before it, +0x22 sixteen aliases that count as done), then m steps of
/// four u16: `{entries, steps}` as those numbers.
pub fn training(part: &[u8]) -> Option<Value> {
    let u16s = |at: usize, n: usize| (0..n).map(|k| crate::u16le(part, at + 2 * k)).collect::<Option<Vec<_>>>();
    let (n, m) = (crate::u16le(part, 0)? as usize, crate::u16le(part, 2)? as usize);
    let entries = (0..n).map(|e| u16s(4 + 0x42 * e, 33)).collect::<Option<Vec<_>>>()?;
    let steps = (0..m).map(|k| u16s(4 + 0x42 * n + 8 * k, 4)).collect::<Option<Vec<_>>>()?;
    Some(json!({"entries": entries, "steps": steps}))
}

/// The command list's icons (the executable's archive 0x4567A0, uploaded under the help font at 0x3B40 by 0x3DE930):
/// entry 0 the arrows (64 x 16, four 16-wide cells, 4 bits) and entry 1 the buttons (20 x 20) with their 16-colour
/// palettes (16-bit; row r at the GS's row 238 + r), one row each: `command_arrows.png` (the arrows in palette rows
/// 0-3: held (solid), tapped (outlined), then both lit; 0x3DF500 takes row 1 less the code's bit 3, 2 more lit; the
/// star row 1) and `command_buttons.png` (the button icon in rows 0-15, the row the buttons pressed, 0x3DF150), each row's
/// picture under the one before; `command_buttons_lit.png` the buttons in entry 2's palettes (uploaded at x 192, the lit
/// icons': 0x3DF150 with the code's bit 0x10).
pub fn command_icons(elf: &[u8]) -> Vec<(String, Vec<u8>)> {
    let base = 0x4567A0 - VADDR;
    let Some(entries) = elf.get(base..).and_then(|d| crate::archive::pack(d.get(..0x2000)?)) else { return vec![] };
    [
        ("command_arrows.png", 0, 0, 64, 16, 4),
        ("command_buttons.png", 1, 1, 20, 20, 16),
        ("command_buttons_lit.png", 1, 2, 20, 20, 16),
    ]
    .into_iter()
    .filter_map(|(name, k, kp, w, h, rows)| {
        let (ups, pals) = (tex::uploads(entries.get(k)?), tex::uploads(entries.get(kp)?));
        let (px, pal) = (&ups.first()?.data, &pals.get(1)?.data);
        let rgba: Vec<u8> = (0..rows * w * h)
            .flat_map(|i| {
                let (row, p) = (i / (w * h), i % (w * h));
                let v = px.get(p / 2).map_or(0, |b| if p % 2 == 1 { b >> 4 } else { b & 15 }) as usize;
                let at = 2 * (row * 16 + v);
                tex::rgba16(crate::u16le(pal, at).unwrap_or(0))
            })
            .collect();
        Some((name.to_string(), tex::png(w, h * rows, &rgba)))
    })
    .collect()
}

/// A colour word of the executable (0x3DBCF0 reads it as 0xRRGGBBAA) as [r, g, b, a].
fn rgba_word(elf: &[u8], at: usize) -> Result<[u8; 4]> {
    Ok(u32_at(elf, at - VADDR, "a colour")?.to_be_bytes())
}

/// The help texts (archive entry 2, 0x45D780: NUL-separated, as 0x3E4FA0 counts them), up to the first "null"; bytes
/// past ASCII (the font's button glyphs) as the characters of the same number.
/// Practice's four menus (FREESTYLE, VS CPU, CHAIN TRAINING, COMBO TRAINING) as the PS2 shows them: their rows,
/// values, defaults and help texts (a mode's help is its two lines; TRAINING DUMMY's and CPU LEVEL's values have one
/// each, `value_help` + value, for the first `value_helps`).
fn practice_modes() -> Value {
    let on_off = |label: &str, y: i32, default: usize, help: usize| json!({"label": label, "y": y, "kind": "list", "values": ["OFF", "ON"], "default": default, "help": help});
    let row =
        |label: &str, y: i32, kind: &str, help: usize| json!({"label": label, "y": y, "kind": kind, "help": help});
    let dummy = [
        "STAND",
        "STAND GUARD",
        "CROUCH",
        "CROUCH GUARD",
        "GUARD ALL",
        "AUTO GUARD",
        "QUICK ROLL  ",
        "QUICK ROLL  ",
        "QUICK TAG OUT",
        "SPRING FORWARD",
        "QUICK ROLL BACK",
        "CONTROLLER",
    ];
    let levels = ["EASY", "NORMAL", "HARD", "VERY HARD", "ULTRA HARD"];
    json!([
        {"title": "FREESTYLE", "rows": [row("MODE SELECT", 138, "mode", 135), row("CANCEL", 168, "close", 137),
            row("COMMAND LIST", 189, "commands", 138),
            // (0x37F5B8: QUICK ROLL's two, 6 and 7, end in two spaces with the icon font's (11) "d" / "u" over them,
            // `icon_dx` from the value's end on its foot: `icons` the cells of `003_003`, in the small colour 1
            // (0x51FE98: palette 9 + 27))
            {"label": "TRAINING DUMMY", "y": 210, "kind": "list", "values": dummy, "default": 0, "value_y": 231, "help": 139, "value_help": 145, "value_helps": 11,
                "icons": {"6": 8, "7": 9}, "icon_dx": -22, "icon_colour": 27},
            on_off("COUNTER ATTACKS", 252, 0, 140), on_off("ATTACK DATA", 273, 1, 141), on_off("HIT ANALYSIS", 294, 0, 142),
            row("CHARACTER SELECT", 325, "select", 143), row("RESET", 346, "reset", 144)]},
        {"title": "VS CPU", "rows": [row("MODE SELECT", 138, "mode", 156), row("CANCEL", 168, "close", 158),
            row("COMMAND LIST", 189, "commands", 159),
            {"label": "CPU LEVEL", "y": 210, "kind": "list", "values": levels, "default": 1, "help": 160, "value_help": 185, "value_helps": 5},
            on_off("ATTACK DATA", 231, 1, 161), on_off("HIT ANALYSIS", 252, 0, 162),
            row("CHARACTER SELECT", 283, "select", 163), row("RESET", 304, "reset", 164)]},
        {"title": "CHAIN TRAINING", "rows": [row("MODE SELECT", 138, "mode", 165), row("CANCEL", 168, "close", 167),
            row("RETRY", 189, "retry", 168), on_off("ATTACK DATA", 210, 1, 170), on_off("TIME DISPLAY", 231, 1, 171),
            row("CHARACTER SELECT", 262, "select", 172), row("RESET", 283, "reset", 173)]},
        {"title": "COMBO TRAINING", "rows": [row("MODE SELECT", 138, "mode", 174), row("CANCEL", 168, "close", 176),
            // (rows 0x506B60: options 9 and 10; 0x37F3E8: who does the combo, "%s %s" its name and (YOU) / (CPU)
            // (0x4318C0); 0x37F4B0: its combo, TYPE n, RANDOM (the CPU's only, with two or more) or --- with none;
            // left / right / cross round them, 0x37A7AC / 0x37A7CC)
            {"label": "COMBO DISPLAY", "y": 189, "kind": "who", "values": ["%s (YOU)", "%s (CPU)"], "default": 0, "value_y": 210,
                "help": 177},
            {"label": "", "y": 231, "kind": "type", "values": ["TYPE %d", "RANDOM", "---"], "default": 0, "help": 183, "colour": 23},
            on_off("ATTACK DATA", 252, 1, 178), on_off("HIT ANALYSIS", 273, 0, 179),
            row("CHARACTER SELECT", 304, "select", 180), row("RESET", 325, "reset", 181)]},
    ])
}

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

/// The select's 3D characters (FRONTEND.md 9): per character id the moves of its moveset's aliases 0 (standing,
/// looped) and 3 (the pick's pose), None where its set has none (Mokujin's and Unknown's stub).
fn select_moves(elf: &[u8]) -> Result<Vec<Option<[usize; 2]>>> {
    let sets = moves::roster(elf)?;
    let alias = |set: usize, alias: usize| -> Result<usize> {
        Ok(u16_at(elf, moves::ALIASES + 2 * (set * moves::ALIAS_COUNT + alias), "the movesets")? as usize)
    };
    (0..moves::BODIES)
        .map(|c| match sets.get(c).map(|s| s.1).filter(|&set| set < moves::MOVESETS) {
            // (7350: a missing alias)
            Some(set) => {
                Ok(Some([alias(set, 0)?, alias(set, 3)?]).filter(|m| m.iter().all(|&v| v < moves::MOVE_COUNT - 1)))
            }
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
    let select = select_moves(elf)?;
    let titles: Vec<String> =
        (0..9).map(|m| u32_at(elf, a(0x51EC18 + 12 * m), what).map(|p| string(elf, p))).collect::<Result<_>>()?;
    let quick: Vec<[u32; 2]> = (0..9)
        .map(|m| Ok([u32_at(elf, a(0x51EC18 + 12 * m) + 4, what)?, u32_at(elf, a(0x51EC18 + 12 * m) + 8, what)?]))
        .collect::<Result<_>>()?;
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
            // QUICK SELECT (0x3CCCB8 with settings +8 set): no lightning, frame, 3D characters (0x3AE7C8 is not run) or
            // timer; the mode's title (0x3BBC40: table 0x51EC18 by mode {text, width, colour}; `titles` the texts,
            // `quick.titles` [width, colour]) centred by its width at
            // foot 40 size 5, and under the plates white lines 4 high fading out 164 long (0x3BE8C0: 0x3BB680 with the
            // templates 0x4400B8 / 0x440070: from 6 to 2 above the point, the even ones from 4 right of it leftwards, the
            // odd ones from 4 left of it rightwards), at the points of 0x440708 (TEAM BATTLE 0x440728; 1 ON 1 the first
            // two 10 higher)
            "quick": {"titles": quick, "title_y": 40, "title_size": 5,
                "lines": [[230, 318], [410, 318], [200, 298], [440, 298]], "team_lines": [[160, 298], [160, 298], [480, 298], [480, 298]],
                "one_on_one_dy": -10, "line": {"y": [-6, -2], "near": 4, "far": 160, "alpha": 0x80}, "face_dy": 52,
                // (0x3BECC0: each side's members' faces at (x, 324), member 0 first (0x51F218 by side); a member not
                // picked yet shows the face under the cursor, the one after it the random face; the options' backdrop
                // under it all, as PCSX2 shows it)
                "faces": {"x": (0..4).map(|k| u32_at(elf, 0x51F218 - VADDR + 4 * k, what)).collect::<Result<Vec<_>>>()?, "y": 324,
                    // (0x3BF080: four opaque grey bars 3 thick about a face (table 0x440688: [x, y, w, h] from its
                    // corner), grey `picked` (0x40) about the picks' faces; about the face under the cursor the grey
                    // pulses |sin(2 pi f / `period`)| x 256 (at most 255) by the frame f (0x3BECC0, gp-0x629C); the
                    // face under the cursor at `same` (0x40) when it is the first pick's character)
                    "frame": {"bars": [[-3, -3, 57, 3], [-3, 52, 57, 3], [-3, 0, 3, 52], [51, 0, 3, 52]], "picked": 0x40,
                        "period": 128, "same": 0x40}},
                // (0x3BEAB0: no name plates; each member's name (0x415840: by character x 8 + costume) at the lines'
                // points (side + 2 x member), side 0's right-aligned on it, side 1's from it, size 4, colour 19, the
                // member being picked blinking colour 2 while frame & 4 is clear; the next member's place, the cursor
                // on the first pick's character, says SELECTED (0x51F1A8) in colour 10; 1 ON 1 10 higher)
                "names": {"size": 4, "colour": 19, "blink": [4, 2], "selected": [string(elf, 0x51F1A8), 10],
                    "texts": (0..34 * 8)
                        .map(|k| Ok(string(elf, u32_at(elf, a(u32_at(elf, a(0x415840) + 4 * k, what)? as usize), what)?)))
                        .collect::<Result<Vec<_>>>()?}},
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
            // yaw} by slot (side + 2 * member) from 0x437D40, 1 ON 1's (mode 5) from 0x437D80, its y less the body's
            // height by character (+0x2A) from 0x437DE8; on the select (screen 5) 0x34EF20 scales the body's root matrix
            // +0x508 by the character's 0x437DA0 (0x3AE6C0, 4096 = 1); 0x3AE7C8: arena 22's light table
            // (0x363630(0x16)); 0x3AEA40: alias 0 looped, a pick plays alias 3, then alias 0)
            "models": {"camera": {"eye": [0, -1152, 4608], "pitch": -128, "yaw": -2048, "h": s16(0x507FE0 + 10 * 24)?},
                "places": places(0x437D40, 4)?, "places_one_on_one": places(0x437D80, 2)?, "one_on_one_mode": 5, "light": 0x16,
                "heights": (0..34).map(|k| s16(0x437DE8 + 2 * k)).collect::<Result<Vec<_>>>()?,
                "scales": (0..34).map(|k| s16(0x437DA0 + 2 * k)).collect::<Result<Vec<_>>>()?,
                "motions": select.iter().map(|m| {
                    let clip = |v: usize| clips.get(v).cloned().flatten();
                    m.and_then(|[a, b]| Some([clip(a)?, clip(b)?]))
                }).collect::<Vec<_>>(),
                // (the two moves' timed ops [frame, op, arg], the hands' and the face's among them: 0x3AEA40 steps
                // them as a fight does, 0x372BD0)
                "ops": select.iter().map(|m| m.map(|[a, b]| Ok::<_, anyhow::Error>([fight::timed(elf, a)?, fight::timed(elf, b)?])).transpose()).collect::<Result<Vec<_>>>()?}},
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
            // (CHICKEN MARKING, FRONTEND.md 8: the counters' thresholds by ROUND TIME, their scale by rounds won)
            "chicken": {"picture": sprite(71), "x": [13, 595], "y": 400,
                "flagged": (0..5).map(|k| u32_at(elf, 0x521190 - VADDR + 4 * k, what)).collect::<Result<Vec<_>>>()?,
                "repeats": (0..5).map(|k| u32_at(elf, 0x5211A4 - VADDR + 4 * k, what)).collect::<Result<Vec<_>>>()?,
                "scale": (0..10).map(|k| u32_at(elf, 0x447590 - VADDR + 4 * k, what)).collect::<Result<Vec<_>>>()?,
                "flags": [0x4000, 0x8000], "history": 20, "voice": 0x1046, "keep": [6, 10]},
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
        // SURVIVAL RESULTS after the lost fight, before the name entry (0x3C5388, over the arena): 0x3BF180's backdrop
        // in the mode's colour of table 0x440748 (4: (0, 255, 64)); the title (0x51FAE8) centred on x 320 at y 40,
        // size 5, colour 19; "WINS %d" (0x51FB18) centred at y 150 in the font's own colours, the number counting up
        // (gp-0x6288 from 1: f x wins / 600 above `fast` wins, else f / 15; at most the wins, 9999 at most) with sound
        // 3 at each change and 5 when it stands; then the lines, each centred (their spaces measured), `dy` apart
        // (as seen): placed (0x3CD1E8: the first of the 10 survivors' records with fewer wins, 0 when none or when
        // a record above it is the same pair) "YOU ARE / THE %s / GREATEST / SURVIVOR!!" from y 250, the place
        // (0x440A28) blinking with "" every 15 frames, colour 2 for the first three, the font's own below; else "YOU
        // NEED / MORE / PRACTICE." from y 290 colour 1; after `hold` frames from the count's end, or a button
        // (0x3B6BD8: 0x8F4), out to black over `fade` (0x3C4AF8)
        "survival_results": {"title": [format(elf, 0x51FAE8), 40, 5, 19], "wins": [format(elf, 0x51FB18), 150],
            "max": 9999, "fast": [40, 600], "slow": 15, "sounds": [3, 5], "size": 10, "dy": 42,
            "placed": [([0x51FB40, 0x51FB68, 0x51FB98, 0x51FBC8].map(|p| format(elf, p).trim_end_matches('\n').to_string())), 250],
            "unplaced": [([0x51FBF8, 0x51FC28, 0x51FC58].map(|p| format(elf, p).trim_end_matches('\n').to_string())), 290, 1],
            "places": (0..11).map(|k| Ok(string(elf, u32_at(elf, a(0x440A28) + 4 * k, what)?))).collect::<Result<Vec<_>>>()?,
            "podium": [3, 2], "blink": 15, "hold": 3600, "fade": 60,
            "backdrop": {"colour": [0, 255, 64, 0x40], "middle": 224, "fade": 3}},
        // the ranking (screen 7, 0x3C8A88) where a new TIME ATTACK or SURVIVAL record is named: the banner by kind
        // (0x3C93A0: sprites 0x520448 253, 257, 255 of file 132: TIME ATTACK, GREATEST SURVIVORS, CHARACTER USAGE) at (0, 20); rows clipped to y
        // 20-446 (0x3DBAA8), `dy` apart from foot `first` (gp-0x6264 / -0x625C: 93 from 153, 4 shown; CHARACTER USAGE 75
        // from 135, 5), `132_000` (sprite 251) across at each foot (0x3C9458); a row (0x3C75E8, 0x3C7AB0): the rank "%2d" at x 16, 27
        // above the foot, size 10, and its suffix size 5, 7 above (0x3C7278); the members' faces (0x3CBFB8, 51 x 51 in
        // the cursor frame `003_110` 2 out in the faces' palette 0x84, 0x3CC078), the second first, and their name plates (0x3CF1E8: less the plate's `x`); TIME ATTACK's
        // time size 5 at (300, -8); SURVIVAL's "WINS" size 5 at (275, -7) then the number size 10 at -26 (0x3C7428; "WINS
        // " below 100); the name at (488, -28) size 10. Colours: the entry's row 11, the others 13, a time 10, wins 12.
        // The entry's row: a band over the row (0x3CE5D8, from 93 above the foot, grey at alpha 0x40), the name so far
        // with the letter under the cursor in its place, under the place a bar 48 x 4 at -8 (0x3CE5D8, red to white:
        // (255, 2t + 1, 2t + 1), t 5 to 113 and back by 5 a frame), the seconds left "%d" size 5 in the font's own
        // colours (font 10's colour 0 is palette 3, table 0x51FE30; `colour` null) at (480, -54) of the `frames` the
        // entry may take (0x3C9124: 3000, then the name is taken as it is)
        "ranking": {"banners": ["132_001.png", "132_003.png", "132_002.png"], "banner_xy": [0, 20], "clip": [20, 446],
            "rows": [{"dy": 93, "first": 153, "shown": 4, "hold": 150}, {"dy": 93, "first": 153, "shown": 4, "hold": 150},
                {"dy": 75, "first": 135, "shown": 5, "hold": 130}],
            // the list's pace without an entry (0x3C8A88's steps 4-10): in from black (+8 of 256 a frame), the last
            // page's rows from above it down `scroll` a frame to their places, each page held `hold` frames (0x3C80E8
            // / 0x3C83A8 / 0x3C84B8), the page before likewise, the first held `after` more, out to black
            "pace": {"fade": 8, "scroll": 12, "after": 60},
            // behind it the arena loaded (the last fight's; the attract's the demo's) without the fighters (0x3B77B8
            // puts both at 0 as screen 7 starts), on an orbit of the arena's middle (0x3B7888 each frame): the angle
            // from `angle` `step` a frame (4096ths of a turn; not while paused), the eye at (cos, sin) x `radius`, `y`
            // high (y down), level, the yaw (camera +4) `step` a frame from 0; H 615 as measured in PCSX2
            "arena": {"angle": 0xC00, "step": -3, "radius": 5000, "y": -1400, "h": 615},
            "separator": "132_000.png", "rank": {"x": 16, "dy": -27, "suffix_dy": -7, "suffix_size": 5},
            "colours": {"row": 13, "entry": 11, "time": 10, "wins": 12}, "faces": [[160, -74], [210, -57]],
            "face_frame": "003_110_p84.png", "face_size": 51, "plates": [[220, -78], [270, -60]],
            "time": {"x": 300, "dy": -8, "size": 5}, "wins": {"x": 275, "dy": -7, "size": 5, "number_dy": -26, "number_size": 10},
            "name": {"x": 488, "dy": -28}, "band": {"h": 93, "colour": [128, 128, 128, 0x40]},
            "cursor": {"dy": -8, "w": 48, "h": 4, "low": 5, "high": 113, "step": 5},
            "counter": {"x": 480, "dy": -54, "size": 5, "colour": null}, "frames": 3000,
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
        // COMMAND LIST (the pause's, 0x3DD098 for each side a player plays, and practice's): 0x3DC9C8 by side at x 20
        // or 336, y 89 (59 in practice); the head (x - 8, y - 8, 300 x 58) RGBA 0x40405073 with the member's face at
        // (x - 3, y - 4) (0x3CBFB8), the character's name at (x + 51, y) and "When Facing Right" (0x522B90) right-aligned
        // in 280 from x at y + 32, all in the help font at level 0x60; the list from y + 50, clipped to 288 high: four
        // moves 72 apart (`lists`, by body), each on its group's two bands (0x4564F0: 18 then 54 high, 300 wide from x -
        // 8; 72 of the second when the name is empty), the name at (x, top), the keys right-aligned in 280 from x + 2 at
        // top + 24 (0xFF 'n' a line 20 lower); arrows (0xDC + d, d + 8 held: cell and mirror flags (1 x, 2 y) 0x522D00 by
        // d & 7, the keypad's 1 2 3 4 6 7 8 9, 16 wide, the star 0xDB the ninth, 14 wide), buttons (0xEC + the buttons'
        // bits 1, 2, 4, 8: 20 wide), drawn 2 low (0x3DFA00).
        // With five moves or more: down / up one, right / left four (round; sound 0x103F), the list easing there (a
        // third of what is left a frame, 0x48 a move), and the bar at x + 286, 6 wide: its track (RGBA
        // 0xFFFFFF80) 288 high, the thumb (0xB0B0B080) 1152 / count high at index x 288 / count, wrapped round; fewer:
        // the rest below the moves in 0x4564F4. Then the window's frame about (x, y, 284, 330) (0x3DC188)
        "command_list": {"x": [20, 336], "y": 89, "practice_y": 59, "head": {"rect": [-8, -8, 300, 58], "colour": [0x40, 0x40, 0x50, 0x73]},
            "face_xy": [-3, -4], "name_xy": [51, 0], "facing": {"text": "When Facing Right", "y": 32}, "width": 280, "level": 0x60,
            "top": 50, "rows": 4, "row_h": 72, "bands": [18, 54], "band_x": -8, "band_w": 300, "keys_xy": [2, 24],
            "colours": (0..10).map(|k| Ok([rgba_word(elf, 0x4564F0 + 8 * k)?, rgba_word(elf, 0x4564F4 + 8 * k)?])).collect::<Result<Vec<_>>>()?,
            "below": rgba_word(elf, 0x4564F4)?, "bar": {"x": 286, "w": 6, "track": [0xFF, 0xFF, 0xFF, 0x80], "thumb": [0xB0, 0xB0, 0xB0, 0x80]},
            "frame": [0, 0, 284, 330], "sound": 0x103F, "step": 4,
            "arrows": {"picture": "command_arrows.png", "cell": [16, 16], "cells": (0..9).map(|k| Ok([u8_at(elf, 0x522D00 - VADDR + 2 * k, "the arrows")?, u8_at(elf, 0x522D01 - VADDR + 2 * k, "the arrows")?])).collect::<Result<Vec<_>>>()?, "advance": 16, "star_advance": 14},
            "buttons": {"picture": "command_buttons.png", "lit": "command_buttons_lit.png", "size": [20, 20], "advance": 20}, "dy": 2},
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
        // practice's menu (START in practice; its texts 0x506528-0x507200, help texts 135-190), placed as the PS2
        // shows it (PCSX2 captures): a window from (38, 82), 404 wide, to 12 below the last row (its frame 10 out), its blue from `colour`
        // [0] at the top to [1] at the foot (measured), in the pause's frame; the mode's
        // name at (67, 106) size 5; rows from x 48, size 4, by their feet; the cursor's bar from the window's left
        // edge; values right-aligned at 426 (on the row's foot, or `value_y`); ON / OFF in the GAME OPTIONS'
        // default / off colours, a list's value in its value colour (0x37F3E8 / 0x37F5B8: 20; COMBO TRAINING's TYPE
        // its row's `colour`, 23; seen teal and blue in PCSX2). MODE SELECT's left / right turn the modes; each row
        // `{label, y, values, default, help (and per value: help + value)}`, `kind` "mode", "close", "list", "select"
        // (CHARACTER SELECT), "reset", "retry", "commands"
        "practice": {"window": [38, 82, 404], "colour": [[0, 32, 60, 0x70], [8, 66, 112, 0x70]], "title_xy": [67, 106], "title_size": 5, "title_colour": 7, "x": 48,
            "value_right": 426, "below": 12, "size": 4, "colours": {"item": 5, "cursor": 11, "on": 16, "off": 22, "value": 20},
            "modes": practice_modes(),
            // the practice fight (PCSX2 captures): with ATTACK DATA on "TOTAL DMG: n" per side at x 32 / 352, foot 38,
            // size 4, colour 17; "DMG: n (nnn%)" (the last hit on the dummy and its share of the move's damage) at
            // (32, 58) colour 0; the mode's name at (18, 427) colour 0; no health bars or timer. FREESTYLE's COMMAND
            // CAPTURE (0x37FE58, state 0 with nothing recorded): "COMMAND CAPTURE" (0x5072B0) at (360, 410) size 4
            // colour 23, under it from (400, 430) "SELECT" size 4 colour 7, then in the small font (size 10) "+" colour
            // 6, the circle button (its "o", drawn from `003_003_cNN`) colour 8 and "=REC" colour 6 (0x5072D0; printf
            // colours, kept as `fonts.small.colours` indices)
            "hud": {"total": {"x": [32, 352], "y": 38, "size": 4, "colour": 17},
                "damage": {"xy": [32, 58], "size": 4, "colour": 0}, "mode": {"xy": [18, 427], "size": 4, "colour": 0},
                "capture": {"text": string(elf, 0x5072B0), "xy": [360, 410], "recorded_y": 390, "size": 4, "colour": 23,
                    "keys": {"xy": [400, 430], "select": "SELECT", "size": 4, "colour": 7,
                        "small": [["+", small_colour(6)?], ["o", small_colour(8)?], ["=REC", small_colour(6)?]], "button": 3,
                        "play": [["+", small_colour(6)?], ["x", small_colour(8)?], ["=PLAY", small_colour(6)?]], "buttons": {"o": 3, "x": 2}},
                    // (0x37FB70 / 0x37FE58: the states' texts at (196, 430) size 4 shown while frame & 0x10, and SELECT =
                    // STOP (0x507350) at (400, 430) while recording or playing; REC LIMIT's sound at 180, 160, 140 of
                    // its 180 frames; a recording is at most 254 runs of one pad (0x37C8A0))
                    "states": {"xy": [196, 430], "size": 4, "blink": 16, "ready": [string(elf, 0x507330), 16],
                        "rec": [string(elf, 0x507340), 12], "limit": [string(elf, 0x507370), 12], "play": [string(elf, 0x507388), 7]},
                    "stop": {"xy": [400, 430], "small": [["=STOP", small_colour(6)?]]},
                    "limit": {"frames": 180, "beeps": [180, 160, 140], "sound": 0x103D}, "runs": 254}},
            // CHAIN TRAINING (0x37EB04, state 0x8B8010; the moves are `command_list.lists[body].training`'s entries with
            // +6 set, in order, at most 128): 0 the first; 1 READY! (0x506DD8) at (192, 220) size 10 colour 2 for 90
            // frames, voice 0x105A at its start, 0x105B at its end; 2 the move's line with the lines of the moves its
            // +0x12 names before it (0x37B8D8, 0x3DD188: from (320, 100) 60 apart, the group's two bands 300 wide, 18 and
            // 42 high, the name at (x + 8, y + 1), the keys right-aligned in 280 from x + 8 at y + 21, the help font at
            // level 0x73), the time counting (to 216000; with TIME DISPLAY "%02d'%02d\"%02d" (0x506E08) at (240, 80)
            // size 4 colour 19: minutes, seconds, hundredths), "COMBO n:m" (0x506E28; 0x506E40 "COMBO 0:0" with no
            // moves) at (400, 80) size 4 colour 10 (0x37EEF8); the player's move entered by one of the move's aliases
            // (+0x10C, 0x37EE18): sound 0x103D, the next move, after the last 3: 180 frames, voice 0x1058 at 120, then
            // under 120 the result (0x3DD5B8: CONGRATULATIONS! (0x522C00) at (50, 100) size 8 colour 11, the face at
            // (294, 174), the name centred at 250 size 6 colour 19, the time at (220, 320) size 6 colour 23) over
            // 0x3BF180's backdrop (the screen from black at the top and the foot to table 0x440748's colour 7, (0, 0,
            // 255), at y 224: templates 0x43FD10 / 0x43FD58; its alpha up `fade` a frame from 0 at the result's start
            // to 0x40, 0x3BB4E8); START, or a face button after the 180, starts again
            "chain": {"ready": {"text": format(elf, 0x506DD8), "x": 192, "y": 220, "colour": 2, "size": 10, "frames": 90,
                    "voices": [0x105A, 0x105B]},
                "lines": {"xy": [320, 100], "dy": 60, "w": 300, "bands": [18, 42], "name_xy": [8, 1], "keys_xy": [8, 21], "width": 280, "level": 0x73},
                "time": {"text": format(elf, 0x506E08), "x": 240, "y": 80, "size": 4, "colour": 19, "max": 216000},
                "count": {"text": format(elf, 0x506E28), "none": format(elf, 0x506E40), "x": 400, "y": 80, "size": 4, "colour": 10},
                "done_sound": 0x103D, "end": {"frames": 180, "voice": 0x1058, "result": 120},
                "result": {"text": format(elf, 0x522C00), "xy": [50, 100], "size": 8, "colour": 11, "face_xy": [294, 174],
                    "name_y": 250, "name_size": 6, "name_colour": 19, "time_xy": [220, 320], "time_size": 6, "time_colour": 23,
                    "backdrop": {"colour": [0, 0, 255, 0x40], "middle": 224, "fade": 3}}},
            // COMBO TRAINING (practice's states 0xB-0xE, 0x379824 / 0x379E2C): "SELECT" (0x506310) at (276, 430) size 4
            // colour 7 and "=DEMO" (0x506328, the small font, colour 6) after it; SELECT shows the chosen side's combo
            // (COMBO DISPLAY's option 9, TYPE option 10: the training entries with +8 set, at most 16, 0x37D0F8) when
            // it has one: the player stops (0x36C468), the side waits to stand free (0x37DEE0) at most `wait` frames,
            // else both are put back `distance` apart (0x37D848 / 0x37D8C8), then its pad plays the entry's steps
            // ([frames, pad as facing right, icons lit, 0]: 0x37D558, 0x37C990 through fighter +0x420 = 3) while the
            // entry's line shows (0x37B8D8 as CHAIN TRAINING's, its first `lit` key icons lit: the sum of the steps'
            // third numbers so far; none for RANDOM); START stops it; after the last step, the side free again, back
            "combo": {"keys": {"xy": [276, 430], "small": [[string(elf, 0x506328), small_colour(6)?]]}, "wait": 120, "distance": 1200}},
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A command list: the count's moves after the name, a name's 0xFC n its group from then on, the keys as they are.
    #[test]
    fn command_list_groups_and_keys() {
        let text = b"\x02  Paul\0\xfc\x02Jab\0\xfc\xff\xed\0Kick\0\xfc\xff\xf0\0\xfc\x03\0 \0";
        let mut part = vec![2, 0, 0, 0, 32, 0, 0, 0, 1, 0, 0, 0, 33, 0, 0, 0];
        part.extend((text.len() as u32).to_le_bytes());
        part.extend(0xffff_ffffu32.to_le_bytes());
        part.resize(32, 0xab);
        part.push(0);
        part.extend(text);
        let l = command_list(&part).unwrap();
        assert_eq!(l["name"], "  Paul");
        assert_eq!(l["moves"], json!([[1, "Jab", "\u{fc}\u{ff}\u{ed}"], [1, "Kick", "\u{fc}\u{ff}\u{f0}"]]));
    }

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

# Tekken Tag Tournament (PS2, USA v2.00, SLUS-20001): the front end and the game modes

Worked out on 2026-11-18 from `SLUS_200.01` (static analysis) and the disc's 2D art files. Memory address = file
offset + 0x33F000. Confidence: [H] read in code, [M] read in code but the meaning is inferred, [L] a guess.
Section numbers of [FIGHT_DATA.md](FIGHT_DATA.md) are written "FD n". Characters are numbered by their
**character id** (0-33, the roster's byte 0, FD 5); costume bodies 34-38 are Tiger, Panda, Tetsujin, Alex, Angel.

| Id | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| | Paul | Law | Lei | King | Yoshimitsu | Nina | Hwoarang | Xiaoyu | Eddy | Jin | Julia | Kuma | Bryan | Heihachi | Ogre | Mokujin | Gun Jack |
| **Id** | **17** | **18** | **19** | **20** | **21** | **22** | **23** | **24** | **25** | **26** | **27** | **28** | **29** | **30** | **31** | **32** | **33** |
| | Roger | Anna | Kazuya | True Ogre | Jack-2 | P. Jack | Michelle | Wang | Lee | Armor King | Ganryu | Kunimitsu | Devil | Jun | Bruce | Baek | Unknown |

## 1. 2D sprites: how the code names a picture

Every 2D draw takes a **sprite id**: the n-th GS upload (image or palette, in file order) of a group of 2D files.
[H]

- Groups: `first id` table **0x520990** (53 x i32), group -> TEKKEN.BIN file table **0x4FBF50** (s16):

| Group | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11-49 | 50 | 51 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| File | 3 | 134 | 141 | 142 | 129 | 132 | 46 | 130 | 136 | 131 | (run time) | 143-181 | 182 | 135 (empty) |
| Ids | 0-185 | 186-195 | 196-244 | 245-248 | 249-250 | 251-258 | 259-264 | 265-298 | 299-310 | 311-320 | 321-322 | 323-400 | 401-404 | 405-406 |

- 0x3CF488 finds the group, 0x3CEB68 / 0x3CEB08 walk the file's GIF packets to the upload and fill the runtime
  sprite table 0x440F60 (32 bytes: packet, x, y, w, h, ...). Draw primitives 0x3CE258 / 0x3CE2B8 (sprite at x, y),
  0x3CE318 / 0x3CE398 (sprite stretched to w x h; a negative w mirrors) take `(layer, image id, palette id, x, y[,
  w, h])`; the palette is another sprite id, almost always image id + 1. Screen is 640 x 448.
- The pack's `textures/FFF_NNN.png` numbers only the images (palettes skipped), so a sprite id maps to an exported
  file through `sprites.py` (below), e.g. id 45 = file 3 upload 45 = `003_006.png`.
- `141_000`-`141_044` (ids 197-241, 320 x 150, 4-bit) take upload 0 of the file (id 196) as their palette; file 3's
  faces take upload 132 (0x84) and its name plates upload 76 (0x4C). The converter exports them so (`front.rs`). [M]

What the files hold (from the art and the draw calls):

| File | Content |
|---|---|
| 3 | the HUD and select atlas: timer digits `003_000` (400 x 47, ten 40-px digits), chrome font `003_001`, small fonts `003_002` / `003_111` (debug), button icons `003_003`, INFINITY `003_004`, URL `003_005`, health bar strips `003_006`-`011`, name plate streaks `003_012` / `013`, round lamps `003_014` / `015`, chicken `003_016`, team-member icon `003_017`, name plates `003_018`-`056`, select cursor frames `003_057` / `058` / `110`, 1P/2P/1A-1D/CPU tags `003_059`-`065`, arrows `003_066`-`068`, faces `003_069`-`109`, `003_112` |
| 46, 129 | Tekken Bowl: title (`046_002`, `129_000`), logos |
| 130 | practice / bowl gauges and pieces (bars `130_000`-`014`, gauges `130_015`, `130_019`-`021`) |
| 131 | five 64 x 64 sparks |
| 132 | records banners: TIME ATTACK, CHARACTER USAGE, GREATEST SURVIVORS (`132_001`-`003`) |
| 134 | title screen: logo `134_003`, namco logos, PAIR PLAY MODE `134_001`, `134_004` |
| 136 | `136_000` a controller, "VS" pieces [L: the VS screen's] |
| 141 | 45 blue lightning pictures (320 x 150), `141_045` the select grid's frame [L: the arcade-version select's] |
| 142 | the boot logo `142_000`, controller `142_001` |
| 143-181 | one 188 x 256 portrait per body, in name order (ArmorKing, Alex, Angel, Anna, Baek, Bruce, Bryan, Devil, Eddy, Ganryu, Heihachi, Hwoarang, Jack-2, Gun Jack, Jin, Julia, Jun, Kazuya, King, Kuma, Kunimitsu, Law, Lee, Lei, Xiaoyu, Michelle, Mokujin, Nina, True Ogre, Ogre, Panda, Paul, P.Jack, Roger, Tetsujin, Tiger, Unknown, Wang, Yoshimitsu) |
| 182 | two 292 x 391 pictures (ids 401-404): `182_000` Kuma, `182_001` a blue flame (403: drawn full-height and mirrored, 0x3C3A74) |

## 2. Modes and the main menu

**Main menu table 0x51EFE0** (12 x 8 bytes `{char *label, u8 id, u8 locked, u8 help, u8 0}`), built into the
visible list by 0x3BD130; a locked entry shows only when its test is true. Choosing an entry (0x3BD938, jump table
0x51F120 by id) calls the mode setup **0x3BA168(mode = id)**, which writes the match settings byte 0x43B7F8
(FD 11) — so **the game mode number is the menu id**. [H]

| Id = mode | Label | Shown when | Notes |
|---|---|---|---|
| 0 | ARCADE | always | section 5 |
| 1 | VS BATTLE | always | handicap health (FD 11) |
| 2 | TEAM BATTLE | always | 1-8 per side; 15 % refill between rounds (FD 11 "mode 2") |
| 3 | TIME ATTACK | always | the arcade ladder against the clock |
| 4 | SURVIVAL | always | "WINS %d" |
| 5 | 1 ON 1 (arcade) | always | the arcade ladder, one fighter per side |
| 6 | 1 ON 1 VS | always | handicap health |
| 7 | PRACTICE | always | FREESTYLE, VS CPU, CHAIN TRAINING, COMBO TRAINING (menus at 0x506528-0x507200) |
| 8 | TEKKEN BOWL | config +0x22 != 0 (0x3BFEB0) | section 6 |
| 9 | THEATER | any ending seen, config +0x10 != 0 (0x3BFED8) | screen 0xC; sets config +0x24 bit 2 |
| 10 | GALLERY | config +0x23 != 0 (0x3BFEE8) | screenshots ("SCRN-%02d.CPT"), screen 0x10, 0x3E5910 |
| 11 | OPTIONS | always | section 8 |

Mode 9 in the fight code (life 160, FD 11) is also the **attract demo**: the title loop calls 0x3BA168(9) at
0x3BCF88. [M] Mode 8 takes arena 20 (the bowling hall). The select screens' titles are table 0x51EC18 (by mode:
PLAYER SELECT, VS BATTLE SELECT, TEAM BATTLE SELECT, TIME ATTACK SELECT, SURVIVAL SELECT, 1 ON 1 BATTLE SELECT,
1 ON 1 VS BATTLE SELECT, PRACTICE SELECT, TEKKEN BOWL). [H]

Texts are plain ASCII with inline codes (`%H%V%F%S%c...`, alignment / font / size / colour) drawn with the fonts of
file 3; the help line under each entry is in the English block 0x45D780-0x45FC40 (French, Spanish, German, Italian
copies follow). Title screen: `134_003` logo, "PRESS START" / "INSERT COIN" / "FREE PLAY" strings 0x51EE78-0x51EF00,
"TM & ©1994 1995 1996 1999 NAMCO LTD." 0x51EF40. [H]

## 3. Character select

**Grid table 0x440A68** (initialised data): 36 slots x 12 bytes `{u8 slot, u8 up, u8 down, u8 left, u8 right, u8
visible, s16 body, s16 x, s16 y}` (36 = no neighbour). Init 0x3C9778 fills `body` from **slot -> character table
0x520460**: a character not in the selectable set (0x43B968) shows as body 0x36 (locked, face `003_101`), 0x37 is
RANDOM (face `003_099`). Draw 0x3CC6C8: each visible slot's face at (x, y + 52), a lit frame on cursors. [H]

| Row (y) | x = 56 | 109 | 162 | 215 | 268 | 321 | 374 | 427 | 480 | 533 |
|---|---|---|---|---|---|---|---|---|---|---|
| 4 | | RANDOM | Kunimitsu | Kazuya | Bruce | Kuma | Jack-2 | Lee | RANDOM | |
| 58 | Baek | Michelle | Armor King | Gun Jack | Anna | Bryan | Heihachi | Ganryu | Julia | Jun |
| 112 | Xiaoyu | Yoshimitsu | Nina | Law | Hwoarang | Eddy | Paul | King | Lei | Jin |
| 166 | | Mokujin | Wang | P. Jack | Devil | True Ogre | Ogre | Roger | Unknown | |

- Cursor moves follow the neighbour bytes (rows wrap left / right). Mokujin (slot 34) and Unknown (slot 35) are
  visible and linked (Yoshimitsu <-> Mokujin, Lei <-> Unknown) only when settings +8 is set; otherwise both are
  hidden (0x3C9804). Section 20 of FIGHT_DATA calls +8 QUICK SELECT; that this byte gates the two slots is read in
  code, its meaning here is [M].
- The 20 characters selectable from the start (table 0x51E850): Paul, Law, Lei, King, Yoshimitsu, Nina, Hwoarang,
  Xiaoyu, Eddy, Jin, Julia, Bryan, Heihachi, Gun Jack, Anna, Michelle, Armor King, Ganryu, Jun, Baek. The rest
  unlock in section 7. [H]
- Faces: table **0x520508** (s32 sprite per body, 0-38; palette id 0x84): `003_069`-`109`, e.g. Paul 160 =
  `003_097`, Tiger 173 = `003_109`. Cursor frame id 0xAE = `003_110` drawn at (x-2, y-2) (0x3CC078). [H]
- Costume: the chosen slot gives the character, the button its costume: the fighter's select value is
  `character * 8 + (costume & 7)` (0x35C8D0); bodies 34-38 come out of the roster through that value. Which pad
  bit gives which costume was not read. [M]
- Random: 0x35C7A8 picks uniformly among the set bits of (selectable AND a mask). Mokujin copies a random character
  from mask 0x51E8A0 (28 characters, no bosses). [H]
- Select music is BGM 0 (FD 14). The arcade-version layout also shows the big pictures of file 141 and the 3D model
  of the hovered character (0x3AECA8 loads it); the simple layout (QUICK SELECT ON) does not. [M]
- Tag-partner flow and the select timer: not decoded (per-player machine 0x3CA470, 7 states, jump table 0x5204D0;
  0x3C9888 stores 60 and 2400, probably the timer). [L]

## 4. In-fight HUD (0x3C1F08 and 0x3C23F0, per side and member)

| Element | Sprites (texture) | Position (640 x 448) | Rule |
|---|---|---|---|
| Health bar | six strips 45, 47, 49, 51, 53, 55 (`003_006`-`011`, 28 px high), palettes 0x2E-0x38; angry: 0x3D-0x42 | side 0: x 29-269, side 1: x 370-610 (240 px), y 30 (both minus the slide-in offset) | width = value * 240 / max; strips: current health, the recoverable cap, and two trails: +0x18 drops max/800 a frame, a second trail max/400 [M on which strip is which] |
| Angry flash | palette set 0x3D-0x42 | | team entry +0x28 >= 120 (FD 10), blinks every 8 frames |
| Name plate | table **0x5206B8** `{sprite, x offset}` per body: `003_018`-`056` (palette 0x4C) over a streak `003_012` or `003_013` (byte table 0x51F678 by body) | y 58, under the bar | 0x3C1DA0; shown for the member in front |
| Round lamps | 0x43 lit (`003_014`), 0x45 unlit (`003_015`) | x 250, 228, ... (side 0), 364, 386, ... (side 1); y 60 | one per round to win (only when 2 or more); a new win blinks 180 frames, 11-frame period |
| Team icons (TEAM BATTLE) | 0x49 (`003_017`, 20 x 25, mirrored for side 1) | y 60, 16 px apart (side 0 from about x 272, side 1 mirrored) | one per remaining member |
| Chicken mark | 0x47 (`003_016`) | x 13 or 595, y 400 | when gp-0x7428[side] (CHICKEN MARKING option) |
| Timer | `003_000` digits [M] | top centre | seconds = (t * 6 / 7 - 1) / 60 (FD 11); INFINITY = `003_004` [M] |
| Stage | "STAGE %2d" / " FINAL" (0x51ED10 / 0x51ED28, 0x3BBF00) | | alternate every 32 frames on the last stage |
| Banners | round number, READY!, FIGHT, K.O., TIME UP, PERFECT, GREAT, DOUBLE K.O. | | timings in FD 13; drawn with the chrome font `003_001` [L] |

Counter-hit text and combo counters were not found in the fight HUD (practice has COMBO DISPLAY). [M]

## 5. Arcade (modes 0, 3, 5)

**No Ogre sub-boss**: the ladder is 8 fights and the final boss is a solo **Unknown**. [H]

Builder 0x3C02D8(cpu side), once per run (settings +0xF == 0); ladder at **0x440A00**, 8 x `{member 0, member 1,
costume 0, costume 1, arena}`; 0x359038 loads stage settings +0x24 (fight counter, cleared by mode setup) into
the CPU team 0x440940.

| Fight | CPU team | Arena |
|---|---|---|
| 1-6 | random characters from the **CPU pool** 0x43B990, never the player's own; one fight gets a real pair from 0x5207F0 | random from a bag of 23, no repeats (0x3C11B0) |
| 7 | the **rival team**: per player character the first free of three in 0x51F3C8 (e.g. Paul: Kazuya, Kuma, Law; King: True Ogre, Ogre, Armor King); "any" falls back to Heihachi, Jin, Jun, Kazuya (0x51F560) | 6 (Ogre) |
| 8 | Unknown, solo; costume rand(2) once Devil is in the pool | 14 (Unknown) |

- Ogre, True Ogre, Devil and Mokujin stay out of fights 1-3. Fight 8 is one round (settings +0x1E = 0); others use
  FIGHT COUNT. Costumes rand(5). [H, M on the exclusions]
- A win increments +0x24 (0x359D3C); at 8 the run is clear: Time Attack saves its time (0x3CD298), the ending plays.
- Time Attack clock: settings +0x2C + +0x34 frames, shown `%02d'%02d"%02d` (0x51ED40). Survival: "WINS %d"
  (gp-0x7430, up to 999). The loading screen shows "STAGE n" / "FINAL" (modes 0, 3, 5) or "FIGHT n" (2, 4). [H]
- **Continue** (0x3BA560): 898 frames, "CONTINUE? 9" down to 0, 100 frames a digit, a face button skips 100;
  the announcer counts (ids 0x1047 + digit, 0x1056 at 0). START continues for free; CHARACTER CHANGE AT CONTINUE
  YES goes back to select. Timeout: GAME OVER. [H]
- **Team lose shot** and win poses: FD 13.

**Endings** (0x3AA9D4): `ending = 0x51B678[character * 8 + costume]` (34 x 8 bytes), using the front member; if
it was already seen and the partner's was not, the partner's. The bit is set in config +0x10 (0x3BFD68) and a new
bit advances the unlocks (section 7). They are real-time scenes with the fighters' models (descriptors 0x51B488,
38 x 12 bytes `{script*, u16 arena?, u16 0, cast*}`), not movies. [H, M on the descriptor]

| Ending | Character (costume) | Ending | Character | Ending | Character |
|---|---|---|---|---|---|
| 0 | Jin | 13 | Jun | 26 | Wang |
| 1 | Paul | 14 | Angel (Devil 2, 4) | 27 | Roger |
| 2 | Law | 15 | Michelle | 28 | Kazuya |
| 3 | Lei | 16 | Julia | 29 / 30 | Kuma / Panda (odd costumes) |
| 4 | King | 17 | Baek | 31 | P. Jack |
| 5 | Nina | 18 | Armor King | 32 | Devil |
| 6 | Anna | 19 | Ganryu | 33 | Tiger (Eddy 4, 6, 7) |
| 7 | Yoshimitsu | 20 | Bryan | 34 | Ogre |
| 8 | Kunimitsu | 21 | Gun Jack | 35 | True Ogre |
| 9 / 10 | Xiaoyu / (costumes 4, 6, 7) | 22 | Heihachi | 36 | Mokujin |
| 11 | Hwoarang | 23 | Bruce | 37 | Unknown |
| 12 | Eddy | 24 / 25 | Jack-2 / Lee | | |

Theater (0x5221E0): OPENING MOVIE, ARCADE MOVIE (the movies, files 14-16), MUSIC PLAYER, then one entry per seen
ending ("(1P)UNKNOWN", ...; names near 0x5F65D0-0x5F6850). [M]

## 6. Tekken Bowl (mode 8)

Arena 20; settings +0x1E = 10 (frames) (0x359C08). Code 0x3E5000-0x3F3000. [H]

| Item | Value |
|---|---|
| Pins | 10 (0x180-byte objects at 0x1FC5260, +0x170 = down) |
| Game | 10 frames, 1-2 players taking turns, a third ball in frame 10, 300 max |
| Throw | position (d-pad, x +-8 a frame, clamp +-1000) -> spin (button) -> power (button); left / right steers until release; START quits |
| Callouts (0x523A18) | 1 STRIKE!, 2 DOUBLE!!, 3 TURKEY!!!, 4+ !!GREAT!!, 12 PERFECT, PUNCH OUT [M]; MISS / FOUL / SPARE; summary STRIKES, SPARES, MISSES, FOULS, SPLITS |
| Per fighter | table 0x523618 (34 x `{s16 160-500 ball weight or speed [M], u8, u8 hand}`); bowl motions 0x6A-0x70 (left-handers 0x6C / 0x70); after the throw a win pose 4770-4773 |
| Options | HELP MESSAGES, SPECIAL EFFECTS (explosions), JUKEBOX (locked until a 200-point game: "BONUS AT 200 PTS") |
| Records | TOP 5 BOWLERS ("THE KING OF IRONBOWL"), save block 0x46BC70 |
| Music | the first session after the unlock always BGM 12, later 9 + rand() % 14 (config +0x22 counts 1-3) |

Not decoded: ball and pin physics, fouls and splits, the gauge timing. Title art `129_000`, `046_002`; gauges
probably `130_015`, `130_020` / `021` [L].

## 7. Unlocks and the save

**Config 0x439C60** (0x68 bytes, saved to the memory card by 0x35EBA0 / loaded by 0x35ED00; a save needs 360 KB):

| Offset | Field |
|---|---|
| +0x08 | u64 bits (0x3BFD30; no caller found) |
| +0x10 | u64 endings seen (bit = ending number, section 5); THEATER shows when != 0 |
| +0x18 | u32 unlock stage |
| +0x20 | u16 play points (0-49) |
| +0x22 | Tekken Bowl: 0 locked, 1 unlocked, 2 first session, 3 played |
| +0x23 | GALLERY / screen capture unlocked |
| +0x24 | u8 bits set when a mode is first entered (0x3BFE78: 4 Tekken Bowl, 2 Theater) [M] |
| +0x26-0x36 | options (FD 20) |
| +0x37 | set by the last unlock; changes the ending scene (0x3A97E0) [L] |

**Advancing** (0x3BFCC0: stage + 1, then 0x3BFBB0 applies the schedule): either an arcade clear whose ending is new
(0x3BFD68), or 50 play points: each finished match adds a per-match value (0x35AE14 -> 0x3BFDF8), not in Tekken
Bowl or the demo. [H]

**Schedule 0x440818** (`{u16 stage, u16 kind, u32 character}` up to stage 0xFFFF; kinds by jump table 0x51F2B0):
kind 0 puts the character in the **CPU pool** (it starts appearing in Arcade), kind 1 makes it **selectable**
the stage after, 5 unlocks Tekken Bowl, 4 the Gallery and screen capture, 6 sets +0x37, 3 (stage 0) fills a set
0x43B9B8 from 0x51E918. [H]

| Stage | Appears in arcade | Becomes selectable | Also |
|---|---|---|---|
| 0 | Kunimitsu | | |
| 1 | Bruce | Kunimitsu | |
| 2 | Jack-2 | Bruce | |
| 3 | Lee | Jack-2 | |
| 4 | Wang | Lee | |
| 5 | Roger (and Alex) | Wang | |
| 6 | Kazuya | Roger | |
| 7 | Kuma (and Panda) | Kazuya | |
| 8 | Ogre | Kuma | |
| 9 | True Ogre | Ogre | |
| 10 | P. Jack | True Ogre | TEKKEN BOWL |
| 11 | Mokujin (and Tetsujin) | P. Jack | |
| 12 | Devil (and Angel) | Mokujin | |
| 13 | Unknown | Devil | |
| 14 | | Unknown | GALLERY, screen capture; +0x37 |

Reset (0x35CAF8, new data): selectable = CPU pool = the 20 of section 3. Costume bodies come with their host
character. The engine needs to store only: the unlock stage, the points, the endings bits, the Bowl / Gallery
flags, the options, and the records (Survival wins 0x440658, Time Attack times, character usage, top bowlers). [H]

## 8. Options

Top menu (labels 0x51DF58-0x51DFA0, help 0x45D780): GAME OPTIONS, CONTROLLER SETUP (vibration, reset), RECORDS
(CHARACTER USAGE, GREATEST SURVIVORS, TIME ATTACK), ADJUST DISPLAY, MEMORY CARD (load, save, AUTO SAVE), EXIT.
Game options, ranges and defaults are FD 20: DIFFICULTY LEVEL (EASY..ULTRA HARD, NORMAL), FIGHT COUNT (1-5,
default 2 wins), ROUND TIME (30, 40, 60, 80, 99, INFINITY; 60), CHARACTER CHANGE AT CONTINUE, GUARD DAMAGE, CHARACTER
SELECT CURSOR (DEFAULT / HOLD), QUICK SELECT, CHICKEN MARKING, HELP MESSAGES, SPEAKER OUT, BGM / SE VOLUME 0-10.
Life is fixed at 140. [H]

Pause menu (0x51E710): resume, COMMAND LIST, SCREEN CAPTURE (once unlocked), CONTROLLER SETUP, ADJUST DISPLAY,
quit; in Bowl also TOP BOWLERS and BOWLING OPTIONS. [M: from the labels and help texts]

## Not decoded

Ranked by how much a player would notice:

1. The select screen's tag-partner flow (two picks per player, the 1P/2P/1A-1D tags), its timer, and which button
   gives which costume.
2. The VS / loading screen layout (file 136, "VS" `0x51F208`, the "%3d%%" loading percent).
3. The exact roles of the six health-bar strips and the timer and banner drawing (positions of the round / K.O.
   texts).
4. Tekken Bowl's ball and pin physics.
5. The ending scenes' scripts (descriptor `script*`), the staff roll, which cutscene files (41-128) they use.
6. The arcade-version select's layout (file 141's lightning pictures and its grid frame).
7. The attract loop's order (movie, demo, ranking; 0x3BCC88) and PAIR PLAY MODE (art only, no menu entry found).
8. What settings +8 means beyond the two grid slots; config +0x08 and +0x37.

## Scripts (`docs/re/`)

Same conventions as FIGHT_DATA.md. `extract_file.py DISC.zip OUTDIR N...` (TEKKEN.BIN files, first sector
unscrambled), `uploads.py FILE.bin` (a 2D file's uploads in order), `sprites.py RAW2D [scan]` (sprite id -> file,
upload, exported texture; `scan` lists every constant sprite the code draws with the caller), `memdump.py START END`
(data as words and strings), `xr.py ADDR...` (references from `xref.pkl`), `dump_ladder.py` (rival lists, fallback,
pairs), `basefld.py BASE_IMM OFF` (uses of a struct field through a base register).

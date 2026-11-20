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
| 142 | the VS / loading screen: its fire `142_000` (640 x 448) and "VS" `142_001` (section 9) |
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
| 4 | SURVIVAL | always | "WINS %d"; a tag battle: its records keep two members (0x5203A0), 0x3C1620 gives both 15 % of full health back to health and cap after a win (the winning human side), the CPU side starts afresh |
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
RANDOM (face `003_099`; both take palette 0xA3, 0x3CBFB8). Draw 0x3CC6C8: each visible slot's face at (x, y), 51 x 51;
with QUICK SELECT on (settings +8) 52 lower, over a backdrop of its own (section 9). [H]

| Row (y) | x = 56 | 109 | 162 | 215 | 268 | 321 | 374 | 427 | 480 | 533 |
|---|---|---|---|---|---|---|---|---|---|---|
| 4 | | RANDOM | Kunimitsu | Kazuya | Bruce | Kuma | Jack-2 | Lee | RANDOM | |
| 58 | Baek | Michelle | Armor King | Gun Jack | Anna | Bryan | Heihachi | Ganryu | Julia | Jun |
| 112 | Xiaoyu | Yoshimitsu | Nina | Law | Hwoarang | Eddy | Paul | King | Lei | Jin |
| 166 | | Mokujin | Wang | P. Jack | Devil | True Ogre | Ogre | Roger | Unknown | |

- Cursor moves follow the neighbour bytes (rows wrap left / right). Mokujin (slot 34) and Unknown (slot 35) are
  visible and linked (Yoshimitsu <-> Mokujin, Lei <-> Unknown) only when settings +8 is set; otherwise both are
  hidden (0x3C9804). +8 is QUICK SELECT (config +0x2C, FIGHT_DATA 20): seen in PCSX2 with Mokujin and Unknown made
  selectable through memory, the grid of QUICK SELECT OFF (the 3D characters') keeps them out; switched ON, the bottom
  row holds 8 and the select has no 3D characters (0x3AE7C8 only with +8 clear). So the PS2 never stands Mokujin or
  Unknown on the select: there is no select pose of theirs to decode (their movesets have no aliases 0 and 3). [H]
- The 20 characters selectable from the start (table 0x51E850): Paul, Law, Lei, King, Yoshimitsu, Nina, Hwoarang,
  Xiaoyu, Eddy, Jin, Julia, Bryan, Heihachi, Gun Jack, Anna, Michelle, Armor King, Ganryu, Jun, Baek. The rest
  unlock in section 7. [H]
- Faces: table **0x520508** (s32 sprite per body, 0-38; palette id 0x84): `003_069`-`109`, e.g. Paul 160 =
  `003_097`, Tiger 173 = `003_109`. Cursor frame id 0xAE = `003_110` drawn at (x-2, y-2) (0x3CC078). [H]
- Costume: the chosen slot gives the character, the button its costume: the fighter's select value is
  `character * 8 + (costume & 7)` (0x35C8D0); bodies 34-38 come out of the roster through that value. [H]
- **Costume per button** (0x3C9ED8, on the pad's raw word 0x415728, libpad's bits byte-swapped: L2 1, R2 2, L1 4, R1 8,
  triangle 0x10, circle 0x20, cross 0x40, square 0x80, SELECT 0x100, START 0x800, up 0x1000, right 0x2000, down 0x4000,
  left 0x8000; 0x3CA428 turns L1/L2/R1/R2/R3 into 4): square 0; cross 1; triangle 2 when the character has one
  (table 0x4FF690, 16 bytes a character: s16 model index per costume 0-7, -1 none; and its flag in 0x43B9B8, filled
  from 0x51E918), else 0; circle 3, else 1; a shoulder 4 (0x3AE608: Armor King's only once his ending, config +0x10
  bit 18, was seen), else 0; START 4 when it has one and not in TEAM BATTLE, else no pick. Both sides played by people
  (gp-0x6A88 / -0x6A84 clear): taking the other side's character in its costume gives another (0x3C0210: 0 -> 2 else 1,
  1 -> 3 else 0, others 0). The timer out picks for a side as a button drawn by 0x3800D8(5): square, triangle, a
  shoulder, cross, circle (0x3CA070). Xiaoyu takes costume 5 one time in 6 when gp-0x6230 is set (set by the cursor
  code 0x3C9C84; not followed). SELECT (0x100) cancels the last pick (0x3CB018). Checked in PCSX2: square, cross and
  triangle give Xiaoyu costumes 0, 1 and 2. [H]
- Random: 0x35C7A8 picks uniformly among the set bits of (selectable AND a mask). Mokujin copies a random character
  from mask 0x51E8A0 (28 characters, no bosses). [H]
- Select music is BGM 0 (FD 14), started 18 frames in (select +0x10). With QUICK SELECT off (the default) the screen
  is file 141's lightning under the frame `141_045`, the timer, and each side's character as its 3D model (0x3AEEF0
  on a cursor move, 0x3AEC58 / 0x3AE9D8 on a pick: motion 3); with it on, a backdrop of its own (0x3BBC40,
  0x3BE8C0) and the picks' faces at y 324 (0x3BECC0). Section 9. [H]
- Per-player machine 0x3CA470 (7 states, jump table 0x5204D0): 0 set up, 1 picking (a pick before the last plays
  sound 4, the last sound 5, 0x3CA3BC; input is ignored 10 frames after a pick, gp-0x6238), 3 / 4 VS BATTLE's
  handicap (1-150, +-1 or +-10) and round count (1-8), 6 done. Cancel (0x3C9518) takes the last pick back, sound
  3. Timer (0x3C9888): select +8 = 2400 frames, +0xC = 60 a second; 90 frames (gp-0x624C) from the last pick to the
  fight. What a side gets when the timer runs out: the character under its cursor (seen on the PS2, section 9). [H]

## 4. In-fight HUD (0x3C1F08 and 0x3C23F0, per side and member)

| Element | Sprites (texture) | Position (640 x 448) | Rule |
|---|---|---|---|
| Health bar | six strips 45, 47, 49, 51, 53, 55 (`003_006`-`011`, 28 px high), palettes 0x2E-0x38; angry: 0x3D-0x42 | side 0: x 29-269, side 1: x 370-610 (240 px), y 30 (minus the member's offset: below) | width = value * 240 / max from the inner end: 45 health, 47 the trail (max / 400 a frame), 55 the recoverable cap, 49 empty, 51 / 53 the caps (below) |
| Angry flash | palette set 0x3D-0x42 | | team entry +0x28 >= 120 (FD 10), blinks every 8 frames |
| Name plate | table **0x5206B8** `{sprite, x offset}` per body: `003_018`-`056` (palette 0x4C) over a streak `003_012` or `003_013` (byte table 0x51F678 by body) | y 58, under the bar | 0x3C1DA0; shown for the member in front |
| Round lamps | 0x43 lit (`003_014`), 0x45 unlit (`003_015`) | x 250, 228, ... (side 0), 364, 386, ... (side 1); y 60 | one per round to win (only when 2 or more); a new win blinks 180 frames, 11-frame period |
| Team icons (TEAM BATTLE) | 0x49 (`003_017`, 20 x 25, mirrored for side 1) | y 60, 16 px apart (side 0 from about x 272, side 1 mirrored) | one per remaining member |
| Chicken mark | 0x47 (`003_016`) | x 13 or 595, y 400 | when gp-0x7428[side] (CHICKEN MARKING option) |
| Timer | `003_000` digits [M] | top centre | seconds = (t * 6 / 7 - 1) / 60 (FD 11); INFINITY = `003_004` [M] |
| Stage | "STAGE %2d" / " FINAL" (0x51ED10 / 0x51ED28, 0x3BBF00) | | alternate every 32 frames on the last stage |
| Banners | round number, READY!, FIGHT, K.O., TIME UP, PERFECT, GREAT, DOUBLE K.O. | | timings in FD 13; drawn with the chrome font `003_001` [L] |

**Decoded (format 18; checked against PCSX2 captures of an arcade fight).** 0x3C2F40 per side draws the member in
back first (0x3C23F0 with a3 0: offset 15, i.e. 15 up and 15 out, colour level 0x50, 0x70 while angry) and the
member in front over it (a3 1: offset 0, level 0x80, its name plate). A tag sets a counter n to 30 while 0x3D3228
reports the tag under way, then it counts down while the round is fought: the offsets are n / 2 (front) and
15 - n / 2 (back). Per member: x of the inner end 269 - offset (side 0) or 370 + offset, y 30 - offset; widths
240 x value / max from the inner end: health (strip 45, cyan), the trail of the health it lost (47, yellow; team
member +0x18 refill aside, a per-member word at 0x1FAC8BC dropping max / 400 a frame to the health, 0 at 0 health),
the recoverable cap (55, red, from the health to the cap), empty (49) to the outer end; caps 51 / 53, 9 wide at
both ends; all 0x3CE318, no blending. Angry (+0x28 >= 120): palettes 0x3D-0x42, 5 frames on, 3 off (counter 8).
Team +0x18 / +0x1C: the refill from the old health and cap at max / 800 a frame, set only by Team Battle's and
Survival's 15 % between rounds (0x3C1620). Name plate 0x3C1DA0: streak (57 or 59 by 0x51F678) 124 x 34 in draw mode 4
(added) and the plate 0x3CF1C0 (the table's x offset not added) at x 20 - offset / 490 + offset, y 58 - offset. Lamps
y 60, 8 higher (+0x24) with no partner. Timer 0x3C6738(0, secs, 320, 6): the tens at x 281, the ones at 320; INFINITY
(sprite 7) at 285. The top row (0x3BC540 -> 0x3BC048, kinds per side by 0x3BC370, feet at y 24, small font): 4 STAGE
(0x3BBF00: x 15 / 370, colour 0) and the run's time 130 right (colour 5); 1 / 2 INSERT COIN / CREDIT (0x3BC828 at
160 / 480, colour 3); 3 GAME OVER (x 144 / 496 less 0x48, colour 2); 5 TIME ATTACK, 7 SURVIVAL, 8 TEAM BATTLE (labels
at 144 / 496 less 0x58 / 0x40, colours 6, 3, 5); 6 WINS n and the rank (Survival). The round texts by round state
(0x36EA88, table 0x500440): 0 0x36EDE0 (ROUND %d at (165, 220) colour 2 on the first round's frames 66-104, the
others' 6-44; "TAG BATTLE" at x 116 instead when gp-0x747E is 1 and neither side is solo; READY! (192, 220) from 106 /
46), 2 0x36EF58 by the result flags (TIME UP 188 colour 12; PERFECT! 170 colour 11; DOUBLE K.O. 114 colour 12; else
K.O. 245 at size 12, colour 12: GREAT shows K.O., only its voice differs), 4 0x36F0D8 (DRAW 232 colour 12; one human:
YOU WIN! (157, 360) colour 2 or YOU LOSE (152, 360) colour 12; two humans: the winner's name). No FIGHT text. The
printf (0x3C6C60): %H the left x, %V the foot (the glyphs' top 20 above, whatever the size), %S the size; the chrome
font's advances at size 10 are table 0x51FF08 by glyph, an A is drawn 6 x size / 10 further left (0x3C69D8).

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

**OPTIONS (0x3B19B0; format 24, `options_menu`)**: "OPTIONS MENU" at (193, 40), size 5, colour 19; GAME OPTIONS,
CONTROLLER SETUP, RECORDS, ADJUST DISPLAY, MEMORY CARD(PS2), EXIT (0x439CC8) from (40, 90) 30 apart, size 4, colour
5, the cursor's 11; under each a line 4 high 24 above the foot from x 40 to 400 (template 0x4402B0: blue (128, 128,
255) at alpha 0x60 fading out), the cursor's white over a band 24 high (0x440100 twice: green (0, 255, 128) from x 0
up to 0x60 at 40, out at 640); the help line = entry. **GAME OPTIONS (0x3B42A0)**: "GAME OPTIONS" at (194, 40); the
14 rows of 0x439CE0 (`options`) from (40, 90) 20 apart, size 4: a list "%s:%c%s", a number "%s: %c%d" (0: OFF),
DEFAULT and EXIT their label; the label colour 5 (the cursor's 11 over the pause's bar from x 0 to 40 to 170 past
"LABEL:", 0x3B15F8), the value 16 at its default, else 12; on the cursor's row the arrows (+0x3C, +0x3E: a list's
left one always, -1 none; a number's while above 0 / below 10); DEFAULT asks YES / NO at x 200 (the choice 11, the
other 22, the arrow at 262 towards it). Help line 6 + row with the value's `\S` alternative. A list's values are printed as the
executable has them, a longer list's with a leading space (" NORMAL", " 60", " INFINITY"). On the cursor's row
(0x3B3F88) the main menu's entries ARCADE..PRACTICE (and TEKKEN BOWL once open) stand right-aligned at x 600 from foot
250, 17 apart, size 4, each in its menu colour where the option applies (descriptor +0x38, a bit per mode: DIFFICULTY
0x25 = ARCADE, TEAM BATTLE, 1 ON 1; GUARD DAMAGE 0x67; ...), else colour 22, over a box (template 0x43FF50: x 360 to
608, y 226 to 16 above the next foot, black from alpha 0 to 0x80 rightwards). [H; checked in PCSX2 2.8.2]

**RECORDS (0x3B2978; format 24, `records`)**: one screen of three pages, state gp-0x6362 (0, 2, 4 build the sorted
list at 0x440D50 +0x124 / +0x126 by 0x3C84B8, 0x3C83A8, 0x3C80E8; 1, 3, 5 draw CHARACTER USAGE 0x3B2090, GREATEST
SURVIVORS 0x3B2358, TIME ATTACK 0x3B2670); right goes to the next page, left to the one before; up / down scroll
(gp-0x6346) six rows. "RECORDS" at (243, 40) size 5 colour 19; the page's name at (192 / 173 / 230, 70) size 4 colour
14 / 15 / 6, the arrows "{" "}" at x 153 and 483 while (frame / 24) is odd (0x3B1F10), "u" / "d" (40, 120) / (40,
152) when there are rows above / below (0x3B1F78, table 0x51E198). Rows from foot 130, 50 apart: the bar 0x3B15F8
(26 spaces; colours 0x43A148: red, green, blue), the rank "%2d" size 5 at x 70 4 up and its suffix (0x3B1EC8 by rank:
ST ND RD TH, 0x43A158) size 4, faces 40 x 42 at x 160 (and 210) 46 up (0x3CE318); CHARACTER USAGE: the name (210,
size 4, colour 11), "%3d.%1d%%" of the per-mille (410, 6 up, size 5, colour 14); SURVIVORS (entries of 8 bytes at
+0x80: u16 wins at +2, the name at +4): "%3d" (260, 4 up, size 5) of at most 999, "WIN" and "S" from 2 (338, size
4), the name (460, 6 up, size 5, colour 15); TIME ATTACK (10 entries of 12 bytes at +8: u32 time, the two characters
+4 / +5, the name +6): the time (260, size 4, colour 11), the name (460, 6 up, size 5, colour 6). Help 24 + page. A
name is 3 letters of "ABCDEFGHIJKLMNOPQRSTUVWXYZ-!?.0123456789<=" (0x520BE8, 42 round; 0x3D0450). The bar's 26
spaces at size 4 take the '0' glyph's 48 x 0.4 each: it fades from x 40 to 40 + 499 + 170 = 709. "u" / "d" (the up and
down marks, "%H%V%cu" at (40, 120) / (40, 152), blinking with the page's arrows) are the button icons' 16 x 16 cells 9
and 8 of `003_003` in yellow. CHARACTER USAGE lists every selectable character, most picked first and by id among
equals (0x3C84B8), "0.0%" with none picked. A new save's records (0x3C88A0): TIME ATTACK ten times 99'00"00 (356400
frames) by the pairs of 0x520388 (Paul / Law, Law / Lei, ..., Tiger / Baek, Jin / Julia), SURVIVAL ten by the pairs of
0x5203A0 with 5, 5, 4, 4, 3, 3, 2, 2, 1, 1 wins, each named by the first body's name's first 3 letters (0x3D01C0: PAU,
LEI, ...). [H; checked in PCSX2 2.8.2: the pages, the bars, the defaults]

**The options screens' backdrop** (0x3BF2F8 by screen: 9 OPTIONS, GAME OPTIONS and RECORDS take colour 11 of table
0x440748, (0, 0, 255); 0xD and 0xF colour 9, 0x10 colour 10): two quads from black at x 0 and 640 to the colour at the
middle (templates 0x43FDA0, 0x43FDE8), at alpha 0x40 over the logo dimmed to 0x40: the logo shows at a quarter, the
middle's blue at 132 (measured). [H; checked in PCSX2]

**Sound options**: BGM VOLUME n plays the music at table 0x518E58[n] of 128 (0, 32, 40, 48, 56, 64, 72, 80, 96, 112,
128; 0x39A1A8); SE VOLUME scales an effect's volume by n / 10 (0x375044); SPEAKER OUT sends mono or stereo to the
sound driver (0x39A208). [H]

**After GAME OVER** (screen 7, 0x3C8A88 and the draw 0x3C75E8) the ranking is not the RECORDS page: over an arena in
3D, a banner (CHARACTER USAGE / GREATEST SURVIVORS / TIME ATTACK with the TAG TOURNAMENT logo) and rows 75 apart with
large ranks, faces, names, a usage bar and large shares, scrolling up from the last row to the first. A new TIME ATTACK
or SURVIVAL record is named there (0x3C9144: 0x3D0350 each frame, 3000 frames to do it). The rows (format 29,
`ranking`): the banner (0x3C93A0) at (0, 20); rows clipped to y 20-446, TIME ATTACK and SURVIVAL 93 apart from foot
153 (4 shown), CHARACTER USAGE 75 from 135 (5); the window puts the entry third (0x3C9034); per row (0x3C75E8 /
0x3C7AB0) `132_000` at the foot, the rank (0x3C7278) and suffix, both faces with their frames and name plates, the
time or "WINS n" (0x3C7428), the name at (488, -28) size 10; the entry's row over a grey band, its place underlined in
a red-to-white pulse, the seconds left at (480, -54). Read in code; the CHARACTER USAGE frames seen in PCSX2 agree; the
arena behind (screen 7's camera: step 2 loads file 5 and 0x3CF0E0(5, ..., 15000)) not decoded. [H on the rows, M on
the rest]

**The ranking's pace** (0x3C8A88, step gp-0x74CE by table 0x5203C0; the list at 0x440D50: +0x11A the first row's foot,
+0x16C the page's first row, +0x11E / +0x120 the hold's count and length) [H]: steps 0-3 build the list (rows apart
gp-0x6264 = 93 / 93 / 75, gp-0x6260 = 4 / 4 / 5 a page, the first foot gp-0x625C = 153 / 153 / 135; the hold +0x120 150
/ 150 / 130 by 0x3C80E8 / 0x3C83A8 / 0x3C84B8) and load the arena; 4 in from black (gp-0x6BE0 up 8 a frame to 256);
without an entry 5: the last page's first row (count less count % per, or a whole page), the foot at first less
count rows; 6 down 12 a frame to it; 7 held +0x120 frames, then the page before (6 again) until the first; 8 60 frames;
9-10 out to black (256 down 8 a frame); 15 three frames, the next screen. With an entry, 0xB places it third (0x3C9034)
and 0xC eases to it ((target - foot + 15) / 16 a frame, 1 to 16). The rows are drawn first, the banner over them
(0x3C93A0).

**Name entry** (0x3D0350): right / down the next letter, left / up the one before (round in 42); a face button takes
the letter under the cursor ("<" takes the last back, "=" ends); the third letter ends; the entry shows the current
letter in its place as it changes; sound 3 a move, 4 a letter. [H]

**Pause menu (0x3B7B98; format 24, `front_end.pause`)**, checked against PCSX2 captures of an arcade fight: the
entries of table 0x51E7A8 (CANCEL, RESET, COMMAND LIST, SCREEN CAPTURE, TOP BOWLERS, BOWLING OPTIONS, ADJUST
DISPLAY, CONTROLLER SETUP): SCREEN CAPTURE only once the Gallery is open (0x3BFEF8), TOP BOWLERS and BOWLING
OPTIONS only in Tekken Bowl, where COMMAND LIST and SCREEN CAPTURE are not; so a fight shows five. Up / down move
round them (sound 3); a face button or L1 / R1 (0xFC) takes one (jump table 0x51E830: CANCEL closes, RESET opens
RESET?, sound 3; the others sound 5). The window 0x3DC768 (160, 130, 320 x 130; 150 high with six entries): navy at
half over the scene in a grey frame about 10 out; "PAUSE P%d" (0x51E728; "PAUSE 1-%C" with a multitap) at (233,
150) in the chrome font, size 5, colour 0, shown while frame & 0x30; the entries from (180, 180) 20 apart, size 4,
colour 5, the cursor's 11 over the bar 0x3BB680 (from x 180 to 40 past the text, 12 to 4 above the foot, green (0,
255, 128) at alpha 0x60 on the left to 0 on the right: template 0x4402F8). **RESET?** (0x3B8460) alone: window (240,
206, 160 x 40), "RESET?" at (270, 230) colour 19, "YES  NO" at (258, 250) size 4, the choice 11 and the other 22,
the printf's "{" / "}" (shown as the gold arrows `003_067` / `003_068`) at (320, 250) towards the choice; NO first;
left / right choose (sound 3), YES takes the reset (0x35A518, sound 4), NO or START back to the entries (sound 3).

**COMMAND LIST** (format 49, `front_end.command_list`; checked against PCSX2 captures of an arcade fight's pause:
Xiaoyu's first page and the page four down after three downs and a right): the pause's COMMAND LIST (0x3B7F74) and
practice's (0x379A98) call 0x3DD098, which for each side a player plays (gp-0x7440) uploads the side's font and draws
its list (0x3DC9C8) from the fighter's command text (0x3DC870 keeps part 2 section 1 of the character's file, by
fighter +0x22): u8 count, the character's name, then count pairs of NUL-ended strings, the move's name (a leading
0xFC n: colour group n - 1 from then on) and its keys. The texts go through 0x3DFA00 (0xFF + a letter: `H` / `V` x / y,
`h` / `v` x 16 / x 20, `w` the width, `i` the colour, `L`, `s` a string, `n` a new line 20 lower; 0xFC 0xFF aligns each
line right in the width; 0xDB the star, 0xDC-0xEB arrows (0x3DF500: direction d & 7 by table 0x522D00 {cell, flags},
the keypad's 1 2 3 4 6 7 8 9, palette row 238 + 1 - (d >> 3): bit 3 held, solid, else outlined), 0xEC-0xFB the
buttons icon with palette row 238 + the buttons' bits (0x3DF150), both from the executable's archive 0x4567A0
uploaded under the font at 0x3B40, row 200; glyphs the help font's widths, icons 16 / 20 / 14 wide, 2 lower). The
layout is in `command_list` and the converter's front.rs; START or a face button closes it (the pause: any of 0x8FC,
back to its entries). [H, layout checked]

**Practice** (checked in PCSX2): the select takes a team of two for the player and, with the CPU's cursor, a team
of two for the dummy; the fight starts in FREESTYLE with no menu; START opens practice's menu (format 34,
`practice`): the mode's name, MODE SELECT (left / right: FREESTYLE, VS CPU, CHAIN TRAINING, COMBO TRAINING), CANCEL,
then the mode's rows (FREESTYLE: COMMAND LIST, TRAINING DUMMY with its value under it, COUNTER ATTACKS, ATTACK DATA,
HIT ANALYSIS; VS CPU: COMMAND LIST, CPU LEVEL, ATTACK DATA, HIT ANALYSIS; CHAIN TRAINING: RETRY, ATTACK DATA, TIME
DISPLAY; COMBO TRAINING: COMBO DISPLAY (the character, "(YOU)", TYPE 1), ATTACK DATA, HIT ANALYSIS), CHARACTER SELECT,
RESET; help texts 135-190. In the fight: TOTAL DMG per side, DMG: n (nnn%), the mode's name at the foot, COMMAND
CAPTURE / SELECT + O = REC. The menus' layout is as seen [M]; their code (0x379000-0x37FB70, the texts
0x506528-0x507200) [H]:

- **The dummy** (0x37A258 each frame; 0x37AF30 by TRAINING DUMMY's value v): v 11 CONTROLLER: +0x420 = 1 (the pad),
  +0x12C = 1; else +0x420 = 4 and 0x37DC98 writes the dummy's pad word +0x430 (+0x12C = 0, the guard input flag).
  For v 6-10 while +0xF7 (set at a move's start from +0x131, the hit) only 0x37E0E8: in a move of posture class
  ((+4 >> 11) & 0x1F) 9 or 10, the input 0x60 (v 6, buttons 3+4: QUICK ROLL into the foreground), 0x90 (7, 1+2: into
  the background), 4 (8, tag: QUICK TAG OUT), 0x2000 (9, forward: SPRING FORWARD), 0x8000 (10, back: QUICK ROLL BACK),
  but only on a frame 0x37E1D8 finds it taken (the input pushed into the history, the move's list walked, or within
  the END look-ahead's window the END destination's at frame 1). Otherwise first 0x37DDE0: a lying move (+4 & 0x204)
  holds up (0x1000) and nothing else; then by v (table 0x506D60): 1 STAND GUARD back (0x8000) while the player's move
  has mask bit 0x10 and its frame is not past its last active frame (0x37E010); 2 CROUCH down (0x4000, 0x37DFF0); 3
  CROUCH GUARD down, down-back (0xC000) the same against bit 0x08 (0x37E180); 4 GUARD ALL in the attack's time
  down-back against 0x08, else back against 0x10 (0x37E090); 5 AUTO GUARD no input, +0x12C = 1 (the stance's guard
  counts). VS CPU: +0x420 = 2, the CPU. The options' getter 0x3794A0(k).
- **The training data**: section 3 of the body's file (TEKKEN.BIN 0x159 + 3 x body; 0x3672B0 -> 0x37D0F8 into
  0x897B90 + 0x40D8 x slot): u16 n, u16 m; n entries of 0x42 bytes (+0 steps, +2 the first step, +6 a chain
  training entry, +8 a combo training entry (the 19-step 10-hit combos), +0xE a time, +0x12 eight entries shown before
  it, +0x22 sixteen aliases that count as done); m steps of 8 bytes (the command list's tokens, drawn by 0x3DD188).
  The chain entries are listed at +0x3ED8 (up to 128), the combo entries at +0x3E94 (16).
- **CHAIN TRAINING** (0x37EB04, state 0x8B8010; format 50 `practice.chain`, the line's place checked in PCSX2; the
  training entry e is move e of the command list, the chain's are those with +6 set, at most 128 (0x37D0F8)): 0 the first entry (+4 = 0), its aliases, the count; 1 "READY!"
  (0x506DD8) at (192, 220) size 10 colour 2 for 90 frames, voice 0x105A at its start, 0x105B at its end; 2 the
  entry's commands with those before it (0x37B8D8: from y 100, 60 apart, at x 320), the time running (to 216000,
  "%02d'%02d\"%02d" at (240, 60) size 4 colour 19 with TIME DISPLAY), "COMBO n:m" at (400, 80) size 4 colour 10;
  the player's move (+0x10C) among the 16 aliases: sound 0x103D, the next entry; after the last, state 3: 180
  frames, voice 0x1058 at 120; START or a face button after them starts again. COMBO TRAINING (0x37CDA0, state
  gp-0x7070) places both sides (0x37D8C8) and plays the combo entries; not followed further.
- **COMMAND CAPTURE** (format 53 `practice.hud.capture`; FREESTYLE only, 0x37FA30; 0x37FB70 on the player's side, state 0x4318C8: +0 state, +2 loop,
  +4 count, +6 / +0xA the pad held / new): 0 shows "COMMAND CAPTURE" (0x5072B0) at (360, 410), or (360, 390) once
  something is recorded, size 4 colour 23, and under it at x 400 "SELECT" (size 4 colour 7) "+" "o" "=REC" (small
  font; colours 6, 8, 6; 0x5072D0) and, recorded, "SELECT + x = PLAY" (0x507300) 20 lower; SELECT + circle: 1 READY
  (0x507330 at (196, 430) colour 16, blinking with frame & 0x10) until any input (0xF4FE), then 2-3 REC (0x507340)
  recording the side's pad (0x37C650, 0x37C8A0) until SELECT or full: 4 REC LIMIT (0x507370, sound 0x103D at 180,
  160, 140 frames), "SELECT=STOP" (0x507350) at (400, 430) while recording or playing; SELECT + cross plays it once,
  SELECT + square in a loop: 6-7 PLAY (0x507388), the dummy's pad +0x430 from the recording (0x37C970, +0x420 = 4)
  until SELECT or its end (5 waits between loops: the last run's frames, 0x37C878). The recording is runs of one pad
  (frames, pad; at most 254, 0x37C8A0) of the player's input history (+0x226, its pad as facing right).

**GAME OPTIONS' select options** [H]: CHARACTER SELECT CURSOR (settings +0xA, HOLD 1): each pick of a side writes its
slot to 0x4157F8 + 8 x side + the pick's number (0x3CAB18); a played side's cursor starts on byte 0 (0x3C993C; at first
18 and 27, 0x3AE798), after its first pick goes to byte 1 when set (0x3CAB8C); 0x3C94C0 clears byte 1 (a pick taken
back); the bytes go to the config's +0x38 / +0x39 at the select's end (0x3CB6CC; 1 ON 1 only +0x38) and back at boot
(0x3AE760). QUICK SELECT (settings +8): 0x3CCCB8 draws no lightning or frame and 0x3AE7C8 no 3D characters, no timer
(0x3CC190); instead the mode's title (0x3BBC40: table 0x51EC18 by mode {text, width, colour}: "PLAYER SELECT" ...,
centred by the width at foot 40, size 5) and white lines (0x3BE8C0: 4 high, 6 to 2 above the points of 0x440708,
TEAM BATTLE 0x440728, 1 ON 1 the first two 10 higher; the even ones fading leftwards from 4 right of the point to 160
left of it, the odd ones rightwards), the grid 52 lower (0x3CC6D4, 0x3CCA28) with a panel per slot (0x3BE038, not
followed).

**CHICKEN MARKING** (settings +0xE, format 55 `hud.chicken`) [H, the counters followed in the code]:
- The counters (0x3D2A38, at each move a fighter starts, 0x36A7CC): for the side's member in front (fighter +0),
  +0x5C[member] += 1 for a move with flag 0x4000 at +0x24, += 2 for one with 0x8000; a move with a first active
  frame (an attack: +0x2C) goes into the member's last 20 aliases (fighter +0x64 + 0x28 x member), and when two or
  more of the other 19 are the same alias, +0x60[member] += 1 (the same attack again and again). Both reset with the
  match (0x36D480).
- The check (0x3D2B88, at the round's end, 0x36E4CC / 0x36FAE0): only with settings +0x1A = 2 (seen 1 in practice;
  by the life code, the match of two players) and +0xE on, for the side ahead in rounds won (+0x5A; none on a tie)
  when no CPU plays it (+0x418). With r the rounds won by both (0-9) and the column c the ROUND TIME option (+0xB, at
  most 4): not marked yet, the side is marked when either member's +0x5C >= A[c] x r x s(r) / 4096 or its +0x60 >=
  B[c] x r x s(r) / 4096 (A = 25, 33, 50, 75, 83 and B = 17, 22, 33, 50, 55: table 0x521190, its two rows the same;
  s = 0x447590: 4096, 4096, 3686, 3686, 3276, 3276, 2867, 2867, 2457, 2457); marked, it stays so while either
  counter is over A[c] x r x 6 / 10 x (T - t) / T (resp. B), T the round's frames by the option (2160, 2820, 4260,
  5640, 6960) and t the timer gp-0x74C0 left. Marked: gp-0x7428[side] = 1 and voice 0x1046 (again each round it
  stays); the HUD (0x3C21DC) draws sprite 0x47 (palette 0x48) at (13, 400) / (595, 400); gp-0x7428 is cleared with
  the match (0x3B7A58).

**The help line** (0x3E5268 -> 0x3E06A0, when HELP MESSAGES is on: 0x3B1468): a band (0, 389, 640 x 48) RGBA
0x18181860 (0x3DBCF0), the text at (30, 390) (its top) in the help font of the executable's archive 0x457380 entry 0
(`help_font.png`: 12 x 18 cells, 16 a row from the space), each glyph its byte of 0x4572E0 on, a space 4; `\n` a
line 20 lower; of the `\S[...]` alternatives only the current value's (gp-0x67BA). The pause's help is text 31 +
entry, RESET?'s 40. The English texts are archive entry 2 (0x45D780). [H; the space's 4 measured]

## 9. Motion, effects and sounds (`front_end`, formats 15 and 16)

Worked out on 2026-11-19 from the code and checked against the game running in PCSX2 2.8.2 (screens captured at
640 x 448 shown 4:3: a capture's y is the screen's times about 1.07). Draw modes (0x3CD868: TEST_2 / ALPHA_2 from
table 0x520648): 0 blended, 1 and 6 opaque, 2 and 4 additive. A level 0x80 draws a picture as it is; 0x3CE258 /
0x3CE318 draw without the PRIM's blending, 0x3CE2B8 / 0x3CE398 with it (gp-0x69A8 = 0x40). The screen state is
gp-0x74D0 (screens by jump table 0x4FB970 in 0x3578F0: 2 title and attract, 3 main menu, 5 select, 9 options, 12
Theater, 14 music player, 16 Gallery), its step gp-0x74CE. Fades (0x3A6770 to a colour, 0x3A67D0 from one, over n
frames; 0x3A6898 held). Sounds are sound ids of group 0 (bank 1): **3** `07_cursor`, **4** `08_select1`, **5**
`09_select2`, played by 0x374CD8. [H unless marked]

**Title (screen 2, 0x3BCC88)**: from white (0x3A67D0, 43 frames) to the logo `134_003`; then 300 frames of the logo,
the line 0x3BC998 (the second of table 0x51EE68, "PRESS P1 AND/OR P2 BUTTON", once a pad is found; "INSERT COIN"
otherwise) at y 329, shown while `frame & 0x30` (off 16 frames in 64), the namco logo `134_000` at (257, 350) and the
copyright at y 409 and 429; then to black over 33 frames and the demo (mode 9). START: sound 4, the main menu. The
attract loop seen on the PS2: opening movie, demo fight, CHARACTER USAGE ranking, namco, the movie again.
`front_end.title`: `press_start`, `press_y`, `blink_mask`, `from_white`, `namco`, `copyright_y`.

**Main menu (screen 3, 0x3BD2E0)**: the title's logo stays (0x3BCBC8(5); dimmed to 0x40 on other screens, 4 a frame,
0x3C4868 / 0x3BB8B8); five rows about the cursor at y 323 + 26 k (the text's foot; the PS2 shows them 19 high),
each in its entry's text colour (table 0x51EFE0 byte 6: the printf `%c` n is palette 9 + n of the chrome font, e.g.
11 gold ARCADE, 19 silver OPTIONS, 23 blue PRACTICE; 22 grey for the VS modes without two pads), clipped to y
315-417 (0x3DBAA8) under two black fades (gouraud quads 0x43FE30 / 0x43FE78: y 315-354 and 380-419). Behind the
cursor's row a band, y 353-380, black to (192, 192, 255) at x 320 and back (0x43FEC0 / 0x43FF08), its alpha up 8 a
frame to 0x80 from the menu's start. A move (only once the rows stand still): sound 3, the rows a row's height off,
4 a frame back; held, again after 18 frames. The arrows `003_067` / `003_068` (palette 40) at (183, 358) and (445,
358), on 24 frames and off 24 once the band is whole. A choice: sound 5. 480 frames without a key: the attract.
Newly unlocked entries blink colour 9 every 4 frames. `front_end.menu`: `rows`, `y`, `row_h`, `scroll`, `clip`,
`fades`, `band`, `arrows`, `idle`, `logo_step`, `dim`; the items' `colour`; `fonts.big.colours` the 36 colours.

**Select (screen 5, 0x3CB900; draw 0x3CCCB8)**, QUICK SELECT off:

| Layer | What | Rule |
|---|---|---|
| Lightning | `141_000`-`141_044` (ids 197-241), 320 x 150 stretched to 640 x 300 at y 180, level 0x70, opaque | the next picture each frame, a 45-frame loop (gp-0x69D0); still once select +0x1C is set (4 frames before the fight) |
| Frame | `141_045` 320 x 309 at x 0, mirrored at x 639 | blended |
| Faces | 51 x 51 at the slots' (x, y) | level 0x80; 0xA0 under a cursor; 0x30 under the cursor of the side that took it; RANDOM under a cursor shows the side's roll: the next selectable character every 3 frames, in id order, not a pick of the side's (0x3C99C8) |
| Plates | the name plates (0x5206B8) at (138, 264) and (41, 238) for 1P's two, (376, 264) and (475, 238) for 2P's (0x3CC260) | the first pick, then the partner or the character under the cursor |
| "1ST" | `%H%V%F1%S%VST` (0x5205C8) at x 62 / 480, y 350 | while picking (0x3CC660); the PS2 shows "2ND" for the partner |
| Timer | the HUD's digits (0x3C6738) at (320, 260) | seconds `(t + 59) / 60 - 1` from 2400 frames; in the last 6 seconds off 8 frames every 30 (gp-0x6224) |
| Cursors | `003_057` in palette 0x74 (1P, blue) or 0x75 (2P, pink: `003_057_p2`), 55 x 53 at (x - 1, y - 1); tags `1P` / `2P` at x + 2 / x + 25, y + 40 (0x5205D8) | 2P's drawn first; a level up and down 6 a frame between 96 and 240 (gp-0x69E0 / -0x69D8); both on one slot: 1P's shaded off diagonally (0x3CE078) |

**The 3D characters** (format 16; checked in PCSX2 through PINE memory reads of the running select). Set up by
0x3AE7C8 (when the QUICK SELECT byte 0x43B800 is 0): the camera 0x3AE6E0 writes the view block 0x415808 (pitch -0x80,
yaw -0x800, roll 0, eye (0, -1152, 4608)) and the projection entry 24 of table 0x507FE0 (H 850, gp-0x743A = 0x18);
0x363630(0x16) the light table of arena 22 (EFFECTS.md). Four fighter structs (0x879640 + 0xB8A0 x slot, slot = side
+ 2 x member) and a request table 0x437E30 (12 bytes a slot: +0 frame, +2 state, +4 character, +6 costume, +8 picked,
+0xA alias): a cursor move (0x3C9E30 -> 0x3AEEF0) drops the slot's body and loads the one under the cursor (costume
0); a pick (0x3CA6B8 -> 0x3AEC58, 0x3AE9D8) sets the alias to 3 with the pick's costume. 0x3AEF88 places a slot from
table **0x437D40** (16 bytes `{x, y, z, yaw}`, 65536ths): 1P (400, 0) yaw 0xE800, 2P (-512, 0) yaw 0x1800, 1P's
partner (896, 512) yaw 0xE000, 2P's (-960, 512) yaw 0x2000; mode 5 (1 ON 1 arcade) takes **0x437D80** (704, 128) and
(-720, 128) for slots 0 and 1. The table's y is not what the struct holds after the load (2 and -40 were read for 112
and 80): the body stands on the floor. 0x3AEA40 steps the motion: at its end alias 3 goes to alias 0 (0x363068(f, 5))
and alias 0 starts again. Aliases 0 and 3 are each moveset's select stance and pick pose (Xiaoyu: moves 6459, 110
frames, and 6490, 86). A side shows its picks and the character under its cursor, the last two; none while the cursor
is on a character it took (seen). `select.models`: `camera`, `places`, `places_one_on_one`, `one_on_one_mode`,
`light`, `motions` (by character id: the two clips).

A cursor move: sound 3; a pick: sound 4, the last sound 5; cancel: sound 3. The PS2 starts 1P's cursor on Xiaoyu
(seen; gp-0x6248). After the last pick 90 frames (gp-0x624C), counted only while 0x3AF448 says the models are in
(PCSX2: 131-135 frames with the disc's loading), then the select's last picture holds while the VS screen loads: no
fade shows (gp-0x71D0 = 20 is set 4 frames before the end). No announcer call and no flash in the select's own code:
a pick shows as the 3D model's pose. **1ST / 2ND**: "1ST" (0x5205C8, drawn by 0x3CC660) while the side has no pick,
not in mode 5; "2ND" (0x5204B8, drawn by the side's machine at 0x3CB4B4) once it has one, not in modes 5 and 6, only
while its partner's place stands nobody (select +0x14 + 4 x side clear: the cursor on a character it took) and the
side's +0x18 has run out (90 frames, set by the pick at 0x3CA9B4). **The timer** (0x3CC190) is drawn whatever the
picks (after the last one too), but QUICK SELECT; its blink-off skipped once select +0x1C is set. Also seen on the PS2: "INSERT COIN" and "CREDIT
0" alternating on the side nobody plays. `front_end.select`: `face_dy`, `face_size`, `face_level`, `lightning`,
`frame`, `cursors`, `cursor_rect`, `cursor_pulse`, `tag_dx`, `tag_dy`, `random_roll`, `plates`, `ordinal`, `timer`,
`done`, `start_slots` (2P's [L]).

**VS / loading (0x3C45B0, drawn by 0x3C3FB8; format 16)**: the fire `142_000` (ids 0xF5 / 0xF6, still: the
"moving" fire of the first pass was not seen again) over the screen; 0x3C3948 draws each side's portraits (188 x 256,
their edges faded over 20 pixels: 0x3C3370 draws nine pieces with gouraud alpha 0x80 inside, 0 at the border; 2P's
mirrored by 0x3C3648) at x = x0 + dx x step, y = y0 + dy x step: 1P's leader (-257 + 38 s, 20 + 3 s), its partner
(-360 + 38 s, 216 - 3 s), 2P's (708 - 38 s, 20 + 3 s) and (810 - 38 s, 216 - 3 s), the partner over its leader;
TEAM BATTLE's 22 and 90 higher. The step goes 0 to 10, one a frame (state 3), then holds while the fight loads
(states 4-6: about 4 s in PCSX2 from the arcade's select, 3.5 s in the attract). "VS" `142_001` (0xF7 / 0xF8) centred
at y 340 when gp-0x7440 and -0x743E are set (both pads played, the attract); otherwise by mode (0x3C410C, jump table
0x51F740): modes 0, 3, 5 "STAGE" (0x51F6C0) at y 350 and the fight's number (settings +0x24 + 1) centred at y 385,
the eighth fight "FINAL" (0x51F6F0) at 350 over "STAGE" at 385; modes 2, 4 "FIGHT" (0x51F700) and the number; mode 7
"PRACTICE" (0x51F710) at 385; modes 1, 6 nothing. The printf's sizes 7 (the words), 8 (FINAL STAGE), 10 (the
number) and colour 14 (palette 14 of the chrome font, silver edged in red): seen in PCSX2 (arcade, STAGE 1), the
scales size / 9.6 of the 42-pixel glyphs [M]. The VS screen is silent: the select's music stops as the select ends,
the fight's starts after the load. Bodies 0x32 and 0x7B take another drawing over the blue
flame `182_001` (not followed). `front_end.vs`: `background`, `logo`, `logo_y`, `steps`, `portrait`, `feather`,
`places` [side][member] `[x0, dx, y0, dy]`, `team_dy`.

**Small texts** (format 16): the small font `003_002` (upload 4 of file 3): 21 glyphs of 16 x 17 a row in ASCII
order from `!` (`/` after `]`, `\` drawn as a yen sign, `@` as ©), each drawn 16 apart whatever its ink (the
title's line measured in PCSX2). The printf (0x3C6C60) takes the left x and the text's foot y. Its colours are the
palettes 9-44 of file 3 as for the chrome font (`003_002_cNN.png`); the printf's `%c` n picks them through a table
of its own (0x3C68A8: font 11's table 0x51FE98, a sprite id a colour, 35-44 and 5; the chrome font's, font 10, is
0x51FE30: colour n -> upload 9 + n, 0 its own palette 3): the title's line (colour 5) -> upload 40, palette 31's
yellow; INSERT COIN (colour 3, 0x3CC4A4's a3) -> 38, palette 29's green; the copyright (11) -> sprite 0, an image,
white on the PS2 (palette 0 here). All three as the PS2 shows them. The printf's other codes: %H the x, %V the foot y
(the glyphs' top 20 above), %F the font (10 chrome, 11 small; 0x51FFA0: glyphs a row, sprite, size), %S the size (10
the glyphs' own, the default), %T measure only. [H] The title (0x3BC998, 0x3BC868): the line at x 336 -
16 - 0xD8 (its width) = 120, foot 329; the copyright (0x3BCC0C) at x 0x24 and 0x123, feet 409 and 429. The select's
side nobody plays (0x3CC4A4 -> 0x3BC828): while frame & 0x30, INSERT COIN (0x3BCA30) or, while frame & 0x40, " CREDIT
0" (0x3BC650), from x = 160 + 320 x side - 12.8 x letters (the code's measure "%T%F%S%s" at size 4; the place as seen),
foot 400. `fonts.small` (`picture`, `glyph`, `rows`, `advance`, `colours`), `title.press_x`, `press_colour`,
`copyright_x`, `copyright_colour`, `select.insert` (`texts`, `x`, `y`, `colour`, `show_mask`, `second_mask`,
`measure`).

**Options (screen 9, 0x3B4B20)**: over the logo dimmed to 0x40; ADJUST DISPLAY moves play entry 0x103F, its reset
0x1040.

`front_end.sounds`: `cursor` 3, `start` 4, `decide` 5, `cancel` 3, `pick` 4, `last_pick` 5.

**Sounds and music, measured** (format 17; PCSX2's video capture with its audio, the pad driven through PINE by a cheat
that makes 0x358698 return a word of memory: pnach `patch=1,EE,203586ac,extended,3C0A0010`, `203586b4 008A5821`, `20358718 8D62FF00` (t2 = 0x100000, t3 = t2 + port x 4, the result lw -0x100(t3)), so the pad of port n is the word at 0xFFF00 + 4n written over PINE (0x7F00, which a lone lw could reach, does not take PINE writes); movies ignore it; the samples matched by cross-correlation with the pack's): title START
sound 4; main menu moves 3, choice 5; select moves 3, picks 4, the last pick 5, cancel 3; no announcer anywhere in the
front end. Music: none on the title (the attract's movie and demo have their own) and the main menu; the select's
BGM 0 (file 2: `select.L` / `select.R`, a bank played on two SPU voices, written also as `audio/music/02.flac`) from
the select's frame 18, looping from 0.9416 s (the VAG frame flagged loop start, 1483: the PS2 repeats every 15.009
s), cut when the select ends; the VS screen silent. Levels: an effect of sample volume v (the group entry's byte 6)
plays at about v / 63 of full scale (volume 25: 0.40; the announcer's 29: 0.46-0.51), the music at its BGM entry's
u16 (`raw[2]`) / 16384 x 0.85 (the select's 9830: 0.50; arena 16's 10649: 0.56). `select.music` (`bgm`, `from`),
`select.ordinal` (`second`, `second_after`), `vs.texts`.

**Timing, measured** (the game's frame counter gp-0x74D8 read through PINE each frame, the screen gp-0x74D0 and its
step -0x74CE, and the capture's frames at 59.94 Hz): the title (2, 6-10) from white 43 frames, the logo 300, to black
33, then the demo (4); START on the title: the main menu (3, 0) the next frame, no fade; 480 frames without a key: the
attract (3, 2); a choice: the select (5, 0-4) 6 frames on, plus its loading (the counter runs: the select's 2400-frame
timer counts it); the last pick to the select's end (5, 5): 90 frames counted while the models are in (131-135 seen),
no fade; the select's last picture then holds while the VS screen loads (48 frames), the portraits slide 10 frames,
the VS screen holds while the fight loads (273 frames from its first in the arcade), 2 black frames, the fight. The
timer out takes both of a side's picks on the same frame. CONTINUE? (0x3BA560): t from 898 down one a frame, the
digit (t + 100) / 100 after the step; as each digit below 9 begins (remainder 99) the announcer's 0x1047 + digit.

## Not decoded

Ranked by how much a player would notice:

1. The VS screen's special portraits (0x3C3948's bodies 0x32, 0x7B).
2. What sets gp-0x6230 (Xiaoyu's costume 5).
3. Tekken Bowl's ball and pin physics.
4. The ending scenes' scripts (descriptor `script*`), the staff roll, which cutscene files (41-128) they use.
5. The attract loop's code (0x3BCC88's steps 0-2) and PAIR PLAY MODE (0x3C4A28: shown with a multitap).
6. What settings +8 means beyond the two grid slots; config +0x08 and +0x37.

## Scripts (`docs/re/`)

Same conventions as FIGHT_DATA.md. `extract_file.py DISC.zip OUTDIR N...` (TEKKEN.BIN files, first sector
unscrambled), `uploads.py FILE.bin` (a 2D file's uploads in order), `sprites.py RAW2D [scan]` (sprite id -> file,
upload, exported texture; `scan` lists every constant sprite the code draws with the caller), `memdump.py START END`
(data as words and strings), `xr.py ADDR...` (references from `xref.pkl`), `dump_ladder.py` (rival lists, fallback,
pairs), `basefld.py BASE_IMM OFF` (uses of a struct field through a base register).

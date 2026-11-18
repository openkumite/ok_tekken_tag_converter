# Tekken Tag Tournament (PS2, USA v2.00, SLUS-20001): fight data in the executable

Worked out on 2026-11-18 by static analysis of `SLUS_200.01` and by pairing moves with Tekken 3's decoded data.
Memory address = file offset + 0x33F000. Code is 0x340000-0x41FFFF, gp = 0x5FD0F0. Statistics are over 1194
attacks paired with Tekken 3 across movesets 0-13, matched by frame count, attack mask and active frames.

The data is the Tekken 3 format moved into the executable.

## 1. Move record (52 bytes, memory 0x531EA0)

A boot routine at 0x367FF0 turns the indices into pointers.

| Offset | Field | Tekken 3 | Evidence |
|---|---|---|---|
| +00 u32 | move number = motion number | +00 | code |
| +04 u32 | posture word, same bits; new bit 24 marks a tag state (56 moves) | +04 | 1158/1194 equal |
| +08 u32 | attack mask and level | +08 | 1193/1194 |
| +0C u32 | cancel list start, an index into 14-byte entries at 0x482248 | +0C | code |
| +10 u16 | alias of the move played at animation end | +10 | code |
| +12 u16 | turn added to facing at commit (0x7FFF = about 180 degrees) | +10 high | code 0x36AA48 |
| +14 u16 | hit record index (section 3), replacing in-record damage and reaction | +14, +32, +34 | code 0x36C1A0 |
| +16 s16 | air drift speed during jump frames | +16 | 1193/1194 |
| +18 u8 | frame count, 0 on disc, filled from motion byte 0 | +18 | code |
| +19, +1A, +1B | jump start, jump end, hold frame | same | 1192/1194 |
| +1C u32 | sound list index (u16 entries at 0x52C448, ends 0xFFFF); 0xFFFFFFFF = none | +1C | code |
| +20 u32 | timed property index (4-byte entries at 0x524690, ends at frame 0); 0xFFFFFFFF = none | +20 | code |
| +24 u32 | flags; the top byte is the camera script | +24 | 1192/1194 for camera |
| +28..+2B u8 | attack bones: end0, start0, end1, start1 | descriptor header | 1166/1194 |
| +2C, +2D u8 | first and last active frame | +2D, +2E | code 0x370808 |
| +2E s8 | hit-stop | +2C | code 0x371F64 |
| +2F u8 | unknown, non-zero in 36 moves | +2F | - |
| +30 u16 | push-sphere control, same modes | +30 low | 1192/1194 |
| +32 u16 | always 0 | - | - |

Flag bits at +24, Tekken 3 bit -> Tag bit:
- 8, 11, 12, 13 -> 0, 3, 4, 5 (exact on 138, 134, 241 and 146 moves).
- 18 -> 11.
- 28, 29, 30 -> 19, 20, 22.
- By code: 0x04 unthrowable, 0x08 not parryable, 0x40 guard by input, 0x200 ki charge (120 frames), 0x400 drops
  push spheres, bit 19 commits the root.
- New 0x100: the posture follows the stick.
- Unknown: 0x4000, 0x8000, bit 17 (toggles an effect), bit 21.

## 2. Cancel lists

The per-move table is at 0x482248 (file 0x143248), 0x82CF entries of 14 bytes; all 7351 lists end in exactly one
END. The shared lists are at 0x47F780 (file 0x140780), 782 entries with 40 entry points.

```
+0 u16 cmd | +2 u8 req | +3 u8 cond | +4 u16 param | +6 u16 dest alias
+8 u16 trans | +A u8 prefilter | +B u8 win_start | +C u8 win_end | +D u8 exec
```

The walker at 0x3691D8 checks prefilter, input, window, req, then cond. Paul's jab matches the Tekken 3 notes
exactly: 2 accepted on frames 1-12 with exec 8, END at 25.

Differences from Tekken 3:
- Special commands use 0x8000, not 0xC000: 0x8000 END, 0x8001 dash, 0x8002 backdash, 0x8003 new (tied to Unknown,
  body 33), 0x8004 call a shared list, 0x8005 return.
- Sequences: a command >= 0x8006 is a motion input stored at `0x4EFD44 + 2*cmd` as a frame window, then steps
  (`buttons<<8 | numpad direction`), then 0.
- Plain commands are unchanged (direction bits 5-13, buttons 1-4 in bits 0-3, bit 4 = held/any), plus bit 14 = the
  tag button (270 entries, mostly `cmd 0x4000` with trans 0x22).
- Prefilter (+A) is new: a mask of stick-direction classes (table 0x4FF8D0) plus 0x20 for any button, used as a
  quick reject.
- trans is 16 bits: 0x8000 turns 180 degrees (was 0x80), 0x4000 was 0x40. There are 46 types (jump table
  0x4FFA80). Types 0-28 kept their numbers; Tekken 3's 33 -> 32 and 44 -> 46 ("use END entry").
- req (0x369008) has more ranges:

| req | Passes when |
|---|---|
| 0x01-0x21 | own moveset = req - 1 |
| 0x22-0x42 | own moveset != req - 0x22 |
| 0x43-0x63 | opponent's moveset = req - 0x43 |
| 0x64-0x84 | partner's moveset = req - 0x64 |
| 0x85-0xA6 | own body = req - 0x85 |
| 0xA7-0xC8 | partner's body = req - 0xA7 (unused in the data) |
| 0xC9-0xEA | own body != req - 0xC9 |
| 0xEB-0xED | flags; 0xEC / 0xED = Kazuya-Devil pair or not |

- cond has 0x60 normal codes (jump table 0x4FF8E0). Codes 0-43 kept their numbers on the paired entries; Tekken
  3's 44-68 are +3; reversal codes start at 0x60 (Tekken 3 0x45 + k -> 0x60 + k). Bit 0x80 adds a partner/tag
  test (0x3693E0). Codes 0x48-0x5F are new and not decoded.

Window and exec bytes were equal on 1978 of 2015 paired entries, params on 2010.

## 3. Hit data

Hit records are at 0x5E8978 (file 0x2A9978), 10 bytes each:

```
u16 gate | u16 damage (14 bits) + type (2 bits) | u16 reaction | u16 close reaction | u16 close distance
```

- The record at move +14 starts a chain: the first record whose gate passes is used, and gate 0xFFFF ends it. The
  gate uses the req encoding, so 0x53 means "opponent is Gun Jack".
- The close reaction replaces the normal one when the fighters are nearer than the close distance.
- Damage equals Tekken 3's on 1164/1194, damage type on 1194/1194.
- The boot routine adds timed-property op 0x0C arguments into the damage, as Tekken 3 does with 0x0B.

Reactions are at 0x46BE78 (file 0x12CE78), 1429 entries of 56 bytes; id 0x592 means none.
- Layout: 7 x u32 (pushback angle, hit curve, second angle, counter curve, grounded angle, grounded curve, block
  curve), then 14 x u16 victim aliases in Tekken 3's slot order.
- Curves are pointers into 72 records of 24 bytes at 0x523FA0: frames, velocity, 10 impulses. Values match
  Tekken 3's.

Hit volumes are not baked paths. They are bare segments between bone positions, at most two per move (code
0x34F288, 0x34F820):
- A segment ends at bone `end`; it starts at bone `start`, or at the end bone's previous-frame position when
  `start` is 0.
- A second segment exists when `end1` is non-zero.
- Bones use Tekken 3's slot numbers. Bone 0x13 becomes 6 for body 14; 0x18 and 0x19 take the segment from a
  projectile object.
- The test (0x376CF8) is Tekken 3's: box reject, clip to the sphere's height band, horizontal distance against
  radius.

Timed properties: ops 1-10 kept their numbers on paired moves; damage to the opponent is 0x0C (was 0x0B), damage
to self 0x0D; hand poses moved up (0x0E -> 0x0F, 0x15 -> 0x18). 221 of 233 paired entries have equal frame and
argument.

Sound lists keep the type nibbles (6, 7, 2, 8, 9); the indices are renumbered.

## 4. Hurt and push volumes

- Hurt: 14 spheres. Bone slots are at 0x42F060 (17, 14, 10, 6, 16, 13, 9, 5, 2, 8, 4, 0, 15, 12, as in Tekken 3).
  Sphere 8 has y - 120, sphere 11 y - 60.
- Hurt radii: pointer per body at 0x42F098.
  - Normal: 200, 200, 120, 120, 300, 300, 100, 100, 250, 300, 300, 200, 400, 400.
  - Large (bodies 11, 16, 20, 21, 22): 300, 300, 200, 200, 450, 450, 150, 150, 450, 450, 450, 400, 600, 600.
- Push: 8 spheres on bones 2, 9, 5, 0, 16, 13, 17, 14 (0x42EFB8). Radii via 0x42EFD8: 240, 96, 96, 300, 120, 120,
  120, 120; large bodies 300, 96, 96, 375, 120 x4.

## 5. Per fighter

- Aliases: 33 movesets x 5532 u16 at 0x58F3D0 (file 0x2503D0); 7350 = missing. At match load 0x368168 builds a
  pointer table per player.
- Alias numbers differ from Tekken 3: stand is 58 (was 3), crouch 80 (was 40).
- There is no common bank: shared moves are the same record reached from several movesets.
- Health is 20.12 fixed point at player +0x3F4. Base comes from the life options at 0x51F5A0 (125, 130, 135, 140,
  160...), times a per-body percent at 0x51F5C0 (Paul 100, Lei 108, King 109, Kuma 120, Ogre 115, Armor King
  111...).
- Reversal motion lists are s16 lists ending in -1 at 0x5E8608.

## 6. Tag notes

- The team table is at 0x440940: 0x60 per side, 0x30 per member; word 0 is the body, +0xC health, +0x10
  recoverable health.
- Player +0x3F8 is the recoverable cap: each hit lowers it by damage x 0.6 (x 1.4 when +0x12A is 1).
- Attacker +0x400 multiplies scripted-event damage by 13/10 (probably the partner-anger boost).
- Partner gates are confirmed by data: Paul with Law 11 entries, King with Armor King, Nina with Anna, Hwoarang
  with Baek.

## Not decoded

- The damage-zone table (not present as a plain array) and counter-hit/juggle scaling.
- Sound index -> sample, and voice rows.
- Camera script data.
- Which life option is the default.
- How costume bodies 34-38 index the per-body tables (the dump script assumes they use their host body's).
- Condition codes 0x48-0x5F, the transition types and the damage-zone lookup were not read in code.

Risk: the hit segments have no radius and use raw bone origins, so the engine must pose Tag's skeleton exactly to
reproduce hits.

## Scripts (`docs/re/`)

Research aids, kept as they were used; they read the executable `disc/SLUS_200.01` beside them (`extract_elf.py`
takes it from your disc image) and the pack's `raw/` folder (`TTT_RAW`). Plain `python3`; `mdis.py` and
`fulldis.py` need `capstone`. `ttt.py` (loaders, table addresses), `dump_fighter.py` (`python3 dump_fighter.py
PAUL`), `dump_tables.py` (reactions, curves, shared cancels, health tables), `t3match.py` (pairing with Tekken 3's
converted game data), `xref.py`, `fn.py`.

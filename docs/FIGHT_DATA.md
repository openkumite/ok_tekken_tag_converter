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
| +24 u32 | flags; bits 23-31 are the camera id (section 18) | +24 | 1192/1194 for camera |
| +28..+2B u8 | attack bones: end0, start0, end1, start1 | descriptor header | 1166/1194 |
| +2C, +2D u8 | first and last active frame | +2D, +2E | code 0x370808 |
| +2E s8 | hit-stop | +2C | code 0x371F64 |
| +2F u8 | custom air reaction index (8-43) when this move juggles (section 9), non-zero in 36 moves | +2F | code 0x3740B8 |
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
- trans is 16 bits: type = bits 0-6, bits 7-12 a parameter, 0x2000 the swap / moveset bit, 0x4000 was 0x40,
  0x8000 turns 180 degrees (was 0x80) (0x368CB0). There are 46 types (jump table 0x4FFA80). Types 0-28 kept their
  numbers; Tekken 3's 33 -> 32 and 44 -> 46 ("use END entry"); the others are in section 8.
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

- cond has 0x60 normal codes (jump table 0x4FF8E0); reversal codes start at 0x60 (Tekken 3 0x45 + k -> 0x60 + k).
  Codes 0-0x24 kept their numbers; three new codes are inserted at 0x25-0x27, so Tekken 3's 0x25-0x44 are +3
  (read in code, section 7; the earlier "0-43 kept" came from pairs that never used 0x25-0x2B). Bit 0x80 adds a
  tag-state test (section 7).

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

Reactions are at 0x46BE78 (file 0x12CE78), 1431 entries of 56 bytes, up to the shared cancel lists (first counted
as 1429; hit records 3053 and 3872 use 1429 and 1430); id 0x592 means none.
- Layout: 7 x u32 (pushback angle, hit curve, second angle, counter curve, grounded angle, grounded curve, block
  curve), then 14 x u16 victim aliases in Tekken 3's slot order.
- Curves are pointers into 74 records of 24 bytes at 0x523FA0, up to the timed properties (first counted as 72;
  two reactions point at 72 and 73): frames, velocity, 10 impulses. Values match Tekken 3's.

Hit volumes are not baked paths. They are bare segments between bone positions, at most two per move (code
0x34F288, 0x34F820):
- A segment ends at bone `end`; it starts at bone `start`, or at the end bone's previous-frame position when
  `start` is 0.
- A second segment exists when `end1` is non-zero.
- Bones use Tekken 3's slot numbers. Bone 0x13 becomes 6 for body 14; 0x18 and 0x19 take the segment from a
  projectile object.
- The bones are the matrix slots player +0x630 + 0x50 x slot (world translation at +0x38). 0x3AFAA0 builds slots
  0-17 from the joint places of 0x439A80 by body (+0x2A; costume file 0x6B has its own at 0x4396C0), not from the
  model's nodes: Jack-2's, P.Jack's and Gun Jack's knee is 390 down the thigh where their model has 440, Ogre's and
  Armor King's arms differ too. The hurt and push spheres use the same slots, and so does the drawing (0x3478F0:
  every joint takes its slot's matrix, the clavicles less node 2's offset, the head its turn only, section 22); since format 57
  the glbs are drawn so too (each slot's node and twist copies at its place, the clavicles less node 2).
- Slots 18-23 (checked in PCSX2): 0x34F3D8 sets, before the strikes, for moveset (+0x28) 4 slot 22 = hand B x (100,
  0, 720) and 23 = hand B x (100, 0, 360) (the sword), for movesets 26 and 32 slot 22 = hand A x (100, 0, -400), for
  moveset 14 on bodies 20 and 15 slot 23 = head x (800, 0, 600) (vectors at 0x5F5E50). 0x3AFAA0 adds joints for
  bodies 17 (18 on the head, 19, 20) and 20 (18 on thigh A, 19 on 18, 20 on the head), swayed by 0x34C508. Every
  other slot past 17 stays cleared: a strike to it runs to the world's origin.
- The test (0x376CF8) is Tekken 3's: box reject, clip to the sphere's height band, horizontal distance against
  radius.

Timed properties: ops 1-10 kept their numbers on paired moves; damage to the opponent is 0x0C (was 0x0B), damage
to self 0x0D; hand poses moved up (0x0E -> 0x0F, 0x15 -> 0x18). 221 of 233 paired entries have equal frame and
argument.

Sound lists keep the type nibbles (6, 7, 2, 8, 9; a few entries have 0, 1 and 3); the indices are renumbered.

Over the whole data: 4174 hit records (3676 chains; the gates used are 0x4E, 0x51, 0x53, 0x56, 0x57, 0x5C: the
opponent's moveset is 11, 14, 16, 19, 20 or 25), 2283 timed property lists (ops up to 0x58, and 10 entries with op 0), 4064 sound
lists, 100 input sequences, 54 reversal motion lists.

### 3.1 Hand poses (read in code; PCSX2 over PINE)

The fingers are not in the motions. Each fighter shows a pose of its own track for each hand, eased in by a weight.
Fields at fighter + 0x8000 (index 0 = hand_B, 1 = hand_A): +0x306C[2] the pose, +0x3062[2] the speed (s16),
+0x307C[2] the weight (f32); +0x3074[2] a scene's track (BOWL_ENDINGS.md 7.3; 0 in a fight). [H]

- **Poses**: fighter pack part 4 (record 0x8794C0 + 0x2C x slot, +0x10), a track `u16 poses, u16 15`, then 15 s16
  triples a pose (17-23 poses). Poses 0-16 are the same for every character, open (0) to the fist (16); 17 on are
  its own. 0x364390 picks pose +0x306C (clamped to the last). Exported as `characters/hands_NNN.glb` (format 52).
- **Set** 0x362E90(fighter, mask, kind, blend) (mask bit 0 hand 0, bit 1 hand 1): pose = kind ? kind + 15 : 0,
  speed = 10000 / (|blend| + 1), weight 0. Kind >= 10 is a face (0x362B38(kind - 10)), not a hand.
  True Ogre (models 82-85, +0x3130 bit 0: 0x3ADAA0) takes another path: his wings (0x351A60), his hands never set.
- **Each frame** (0x3634F0): a weight that reached 1 goes to 0 with its speed; else weight += speed x 0.0001, at
  most 1. The draw (0x3DE0D8, nodes 64-78 by hand 0's pose and weight, 39-53 by hand 1's) mixes each finger's
  matrix rows, pose x w + last frame's x (1 - w) (0x34A180, VU0), and squares them up (0x34A710: x normalised, y less
  its x part normalised, z = x x y). The matrices are kept between frames: the hand moves w of what is left each
  frame, and stays when the weight is 0. Hand 0 takes the triples as a scene's hand_B (table 0x522CC0), hand 1 as
  hand_A (0x3DD788).
- **Timed ops** (0x372D28): 0x0F + v, v < 0x60: mask v >> 5 (0 = both), kind v & 31, blend the argument; kind 9
  is the model's own kind (0x416BA0, a byte a model: mostly 1; King and Nina 2, Xiaoyu 5, Eddy and Kuma 0 ...).
  Each sets +0x11F ("the move set the hands", cleared by every move start, 0x36A83C). In the data: 293 + 67 + 59 open, 301 + 83 + 77
  fist, kinds 2-7 and 9 the rest.
- **The fight's own** (0x370BB4, before the ops each frame): K.O.'d (+0x131): both to the kind of 0x416C28 (all 0,
  open), blend 10, every frame. Else unless +0x11F: a move that started a new motion (+0x134, set at the start when
  the motion changes or the transition is 1 or 3: 0x36A9A0) with no damage (+0xCA) and not +0xF7, in round states
  1-2: both to the model's kind, blend 10 (0x3630D0); a damaging move whose record +0x28 (the strike's end bone) is
  below 0xC (an arm) from frame +0x2C - 7 on: the same, every frame; a throw's victim (+0xE0 < 0) at its counter
  +0xC8 = 10: both open, blend 20.
- **The load** (0x363150): pose = 0x416A90[2 x model + hand] (0-3), no weight (never drawn), then both to the
  model's kind, blend 10 (0x3630D0): the fingers ease from the model's to it. 0x363068(fighter, blend) sets each
  hand to its 0x416A90 value as a kind (0x3AEAF4, 0x3AF1DC blend 5; 0x3D34D4 blend 1) [their contexts not read].
- Measured: Xiaoyu's stance 8 at the round start: frame 1 pose 20 (her kind 5) at 0.0909, then her op 0x11
  (kind 2, blend 20) at frame 1: pose 17, the weight 0.0476 a frame to 1 at frame 22, then 0; her jab (move 570,
  ops 0x0F at frames 1 and 2, blend 1): weight 0.5 at frame 3, 1 at frame 4; the finger matrices change only while
  the weight is above 0.

### 3.2 The face: blinks and expressions (read in code)

The face nodes 7-28 show a pose of the model's face track (fighter-pack part 5, `u16 poses, u16 33`, then 33 s16
triples a pose: 2-51 poses in 83 of the 128 model files, none for the masked ones),
eased like the hands. Fields at fighter + 0x8000: +0x3066 the face (s16), +0x3068 its timer (s16), +0x3084 the pose,
+0x308C the speed (f32), +0x3090 the weight (f32); +0x3088 >= 50 a scene's track (BOWL_ENDINGS.md 7.3; 0 in a fight).
Exported as `characters/face_NNN.glb` and the index's `face` (format 71). [H]

- **Set** 0x362B38(fighter, face, blend, hold): the timer = (rand() & 0xFE) + 1 for face 0, else hold; then unless
  the face is already shown: pose = face, weight 0, speed = 1 / (|blend| + 1). (Both cleared while 0x3D3228 of the
  fighter's side, two countdowns at 0x1FB6ED0 / 0x1FB6EE0, is running.) The timed ops' kinds 10 and up (0x362E90)
  set face kind - 10, blend 20, hold the op's argument (and +0x11F, as the hands').
- **Each frame** (0x362C28 from 0x34E838; not on the select, 0x3AF358, nor while frozen, gp-0x7210 / 0x720C /
  0x3A9698, nor in Tekken Bowl): by the model's s8 tables (0x86 models, `(file - 345) / 3`): while the move's flag
  0x800 (+0x24) is on, set its `move` face (0x416D38: 1 for most, 0 none), blend 4, hold -1; else while the move's +4
  bit 2 is on and +0x24 bit 22 off (hit), its `hit` face (0x416DC0), blend 8, hold -1. A shown `move` face without the
  flag, or `hit` face without the bit, goes back to face 0, blend 1. When the timer is 0: face 0 blinks (the `blink`
  face, 0x416CB0, blend 1, held 3 frames), the blink goes back to face 0 (blend 1; open (rand() & 0xFE) + 1 frames),
  any other face to the blink, blend 30, held 30. Then the timer - 1, and the weight: one that reached 1 goes to 0
  with its speed, else + the speed, at most 1. Blink face 0 (Lei, Bryan, the costume-7 models) never blinks.
- **The draw** (0x3DD918 before the hands 0x3DE0D8 and the look 0x351E98, 0x347B30): each face node's matrix (kept
  from the last frame, +0x8990 + 0x40 x node) goes the weight of the way to the pose's (BOWL_ENDINGS.md 7.3's
  mapping): a turn's rows mixed and squared up (0x3DD8E8), a scaled turn's rows mixed (0x34A180), a place mixed
  (0x34A998). Without a face track the nodes take the model's rest table (0x3B87B0).

## 4. Hurt and push volumes

- Hurt: 14 spheres. Bone slots are at 0x42F060 (17, 14, 10, 6, 16, 13, 9, 5, 2, 8, 4, 0, 15, 12, as in Tekken 3).
  Sphere 8 has y - 120, sphere 11 y - 60.
- Hurt radii: pointer per body at 0x42F098.
  - Normal: 200, 200, 120, 120, 300, 300, 100, 100, 250, 300, 300, 200, 400, 400.
  - Large (bodies 11, 16, 20, 21, 22): 300, 300, 200, 200, 450, 450, 150, 150, 450, 450, 450, 400, 600, 600.
- Push: 8 spheres on bones 2, 9, 5, 0, 16, 13, 17, 14 (0x42EFB8). Radii via 0x42EFD8: 240, 96, 96, 300, 120, 120,
  120, 120; large bodies 300, 96, 96, 375, 120 x4.

### 4.1 Root motion (read in code; checked frame by frame in PCSX2)

- Each frame (0x34E748, after the move logic, so also on the frame a move starts) the root's playhead (+0xB8 motion,
  +0xBC frame) is decoded (0x363AB8) and its height and distance multiplied by the player's motion scale +0x4FC
  (4096 = 1), each product cut toward zero (the distance's sign bit cleared). +0x4FC is set at the fight's load
  (0x3AF7D0) from the model's `f32` at +0xC: percent x 4096 / 100, cut (Paul 100 -> 4096, Xiaoyu 98 -> 4014, Lei 106 ->
  4341, True Ogre 103 -> 4218, Roger 96 -> 3932). A move whose +0xE0 is negative (the opponent's: a throw's victim)
  takes the other player's scale (0x363A80).
- The drawn root (+0x51C, 0x34E938) is the origin (+4, +8, +0xC) less the distance along the facing +0x1A plus the
  root's heading: the 4096 sine table at 0x4FCE88 at (angle >> 4) & 0xFFF (the cosine 0x400 on), each product cut
  toward zero (>> 12); the height is taken off y. A rebase (+0x126, 0x34FA60) puts the origin at the drawn root plus
  the same offset of the frame before. Transition 0x18 (the walk's start) keeps the origin: the new motion's frame 0
  shows at once.
- The float path beside it (gp-0x6E9C set, 0x3A9520) is the endings': raw motions turned with sinf / cosf.
- The body push (0x377338) takes the spheres of this frame's pose (+0x308, before the push) and each player's step
  since the last push in 3D, its height in (0x8A7A90); the result goes on the origin and the drawn root at once
  (+0x3E8, 0x371CF0).
- PS2 runs that force a body at the load screen keep the select's models: their travel carries that model's scale
  (Nina's 98: 0.98 of every step) and its skeleton's reach. Record with the fighters picked.

## 5. Per fighter

- Aliases: 33 movesets x 5532 u16 at 0x58F3D0 (file 0x2503D0); 7350 = missing. At match load 0x368168 builds a
  pointer table per player.
- Alias numbers differ from Tekken 3: stand is 58 (was 3), crouch 80 (was 40).
- There is no common bank: shared moves are the same record reached from several movesets.
- Health is 20.12 fixed point at player +0x3F4. Base comes from the life options at 0x51F5A0 (125, 130, 135, 140,
  160, then 135 three times: the table has 8 words), times a per-body percent at 0x51F5C0 (Paul 100, Lei 108, King 109, Kuma 120, Ogre 115, Armor King
  111...).
- Reversal motion lists are s16 lists ending in -1 at 0x5E8608.

## 6. Tag notes

- The team table is at 0x440940: 0x60 per side, 0x30 per member; word 0 is the body, +0xC health, +0x10
  recoverable health.
- Player +0x3F8 is the recoverable cap: each hit lowers it by damage x 0.6 (x 1.4 when +0x12A is 1).
- Attacker +0x400 multiplies damage by 13/10: the partner-anger boost (section 10).
- Partner gates are confirmed by data: Paul with Law 11 entries, King with Armor King, Nina with Anna, Hwoarang
  with Baek.

## 7. Conditions (second pass, read in code)

The test is 0x3693E0(entry, self, opponent). Player fields used below: +0x3E the body's turn off the opponent's
direction (16-bit angle), +0x40 its absolute value, +0x42 / opponent +0x42 the quadrant (0 front, 1, 2 back, 3; as
Tekken 3's +0x32), +0x4C, +0x54 the screen-relative values of section 12, +0x160 the knock-down stun, +0x16A the
120-frame ki timer, gp-0x74C8 the pair distance, on_right(side) the screen-side byte at gp-0x65D0 (section 12).

Tekken 3 numbering, corrected:

| Tag | Tekken 3 | Test |
|---|---|---|
| 0x00-0x24 | same | unchanged (0x24 = back turned, +0x40 > 0x4000) |
| 0x25 | new | not back turned (+0x40 <= 0x4000); 23 entries |
| 0x26 / 0x27 | new | +0x3E < 0 / >= 0 (which way the body is turned off the opponent); unused |
| 0x28-0x2E | 0x25-0x2B | back turned left / right (+0x3E >= 0x4E38, or < 0 and <= 0xB1C6), own quadrant 0, 1, 3, 2, opponent quadrant 0 |
| 0x2F-0x31 | 0x2C-0x2E | opponent quadrant 1, 3, 2 |
| 0x32 | 0x2F | always false (the default arm) |
| 0x33-0x47 | 0x30-0x44 | same tests (auto-duck, stun, lying, escapes 0x3D-0x3F, K.O. 0x40, sides 0x41-0x44, air recovery 0x45, ki 0x46-0x47) |

Confidence: high (each arm read; 0x28/0x29, 0x2A-0x31 match Tekken 3's code shapes and pairs). The engine's
dialect maps Tag 0x25-0x2E straight to Tekken 3's numbers, which is wrong for 0x25-0x2E (1 376 entries).

Two literal tests kept from Tekken 3's arms whose meaning is not settled: 0x36 = `self+0x3E8 (u64) != 0 or
self+0x3F0 != 0` (T3 0x33), 0x37 = the same and opponent +0x135 == 0 and not lying (T3 0x34).

New codes 0x48-0x5F:

| Code | Test | Uses |
|---|---|---|
| 0x48-0x4B | self +0x4C in [0x200,0x400), [0x400,0x600), [0xA00,0xC00), [0xC00,0xE00): where I stand from the opponent, camera-relative, 12-bit angle | 0 |
| 0x4C-0x4F | the same and back turned | 0 |
| 0x50 / 0x51 / 0x52 / 0x53 | self +0x54 == 1 / 2 / 0 / 3: which way I face on screen (quadrant against the camera's axis; PCSX2: 0 facing the screen's right, as the left fighter at the round start, 2 the right one) | 0 |
| 0x54-0x57 | +0x54 == 1 / 2 / 0 / 3 and on_right(self) == 0 | 0x54: 18, 0x57: 18 |
| 0x58-0x5B | the same and on_right(self) == 1 | 0x58: 18, 0x5B: 18 |
| 0x5C | off screen: the three projected bone x values at +0x628/+0x62A/+0x62C all >= 0x215, or all < -0x14 (0x34FE7C) | 101 (tag-out runs end on it) |
| 0x5D | pair distance < 0x1D01 (parameter ignored) | 0 |
| 0x5E | on_right(self) changed since the last frame | 0 |
| 0x5F | round decided: +0xB4 < 0, the round-result flag (gp-0x7444 bit 0), no pause, and the round count reached or either side has the wins (section 11) | 1 |

Bit 0x80 (0x3693E0 head): a test on the opponent's +0x129, which is set when a move with posture bit 24 (the tag
state) starts: +0x129 = the previous on_right of its side + 1, else 0 (0x36AA70).
- param != 0: passes when opponent +0x129 == 0, or on_right(self) + 1 == opponent +0x129;
- param == 0: fails when opponent +0x129 != 0 and on_right(self) + 1 == opponent +0x129.

Used only inside tag-out moves (56 entries, with codes 0x00, 0x54-0x5B): it picks the run-off direction when the
opponent is tagging at the same time. Confidence: medium on the meaning, high on the test.

## 8. Transition types 29-45

Types 2-25 are first remapped by 0x36BE08 (table 0x4FFB40), against the destination move: 2 -> 20 when the
destination is an attack whose next frame is still before its last active frame (0x36BEC0); 12-15 -> 11 when the
destination has no active frames; 20 / 21 -> 11 unless 0x36BEC0 holds; 25 -> 22 when it holds. Field +0x11E is the
per-frame facing mode (Tekken 3's +0xB9; jump table 0x500570, 15 modes).

| Tag type | Handler | Does | Tekken 3 | Uses |
|---|---|---|---|---|
| 28 | 0x36B4A4 | frame 1, mode 0xA, get-up lock 24 (+0x170) | 0x1C | 1005 |
| 29 | 0x36B4F4 | continue at the next frame (clamped to the move), root rebased, mode 0xA | 0x1D | 4 |
| 30 | 0x36B570 | enter a juggle with air reaction 4 (section 9): +0xFA = 1, +0xFC = 4, juggle physics 0x374148, stun from 0x36A2D0, mode 0xB | ~0x1E | 2 |
| 31, 32 | 0x36B638 | throw link: chain counter +0xE0, the pair's root and facing from the partner, a 16-frame turn | 0x21 (33) | 2253 on 32 |
| 33 | 0x36BAE0 | frame 1, root committed, yaw = toward the opponent (+0x3A), mode 8 | 0 + mode 8 | 109 |
| 34 | 0x36BB1C | frame 1, yaw = +0x44, mode 0xC: over 8 frames turn to the screen axis +0x50 (at most 10 degrees a frame) | new | 595 |
| 35 | 0x36BB48 | the same with mode 0xD and axis +0x52 (perpendicular) | new | 140 |
| 36 | 0x36B77C | victim side of the throw chain | 0x22 | 0 |
| 37-45 | 0x36B890.. | reaction entries set by the slot selector (37 blocked, 38 airborne, 39 lying, 40 counter, 41 side 1, 42 side 3, 44 front, 45 back) | 0x23-0x2B (+2) | code only |
| 46 | - | use the END entry | 0x2C | 103 |

Flag bits (0x368CB0, consumed at 0x36A4F0 when the next move starts):
- 0x2000 with parameter 0: the tag swap (section 10). With a parameter (bits 7-12): Unknown changes moveset
  (0x36C318, or 0x36C278 for body 33); only the 54 `0x8003` entries of Unknown use it.
- 0x4000 sets +0x17E, which 0x373428 treats like move flag bit 1 (a per-frame state machine at +0x35DC; meaning
  not settled).

Confidence: high for the actions; the Tekken 3 pairing of 29, 30 and 36 is by behaviour, not by data.

## 9. Reactions, juggles and damage

Slot selector 0x36A050(victim, attacker), Tekken 3's order:

| # | Case | Slot (reaction +0x1C + 2*slot) | Type |
|---|---|---|---|
| 1 | blocked (victim guard & attacker mask) | 10 standing, 11 crouching | 37 |
| 2 | airborne (+0x135, or posture 0x400) | air table, below | 38 |
| 3 | posture 0x200 | 13 | 39 |
| 4 | posture 0x4 (lying) | 12 | 39 |
| 5 | counter hit (+0x13A) and hit from the front (quadrant 0) | 2 / 3 | 40 |
| 6 | quadrant 0 / 1 / 3 / 2 | 0, 4, 6, 8 (+1 crouching) | 44, 41, 42, 45 |

Difference from Tekken 3: a counter hit from the side or back takes the side / back slot (Tekken 3 ignores the
direction). The close reaction replaces the normal one when the distance is below the hit record's close distance
and sets +0x13B (0x371F84).

Airborne hits use an air table, as in Tekken 3 (0x374010): entries of 28 bytes, index < 8 at 0x500678, otherwise
0x5F2BA8 + 28 * index. Entry: u16 victim alias, u16 target height, u16 fixed rise speed, u16 x3, then pushback
samples. The index (0x374048):

| Index | Alias | Height | When |
|---|---|---|---|
| move +0x2F | per move | - | the attacker's move has a custom index (36 moves, 8-43) |
| 1 | 4762 | 550, rise 400 | spike: victim strike vector Y (+0x3E0) >= 0x6E |
| 2 / 3 | 4766 / 4767 | 436 | hit from quadrant 3 / 1 |
| 0 | 4727 | 440 | otherwise |
| 7 | 4763 | 550, rise 400 | spike, victim face-down or in aliases 0x129B-0x129F (early frames) |
| 5 / 6 | 4764 / 4765 | - | the same victims, by height (|+0x520| above or below 0x2DF) |
| 4 | 4729 | 1357 | transition 30 |

Juggle physics 0x374148 equal Tekken 3's: juggle counter n = +0x100; +0x104 = clamp(0..100, damage + 7(n+1) +
10, +10 more for index 4); rise speed = clamp(-100..100, 4*damage + 40 - 7n - (velocity Y + strike Y)/8).

Damage (0x3720C8, victim s0, attacker s2; D = the hit record's damage, attacker +0xCA; throws (type 1) carry 0
and deal theirs through timed property 0x0C):
- No damage zones: the hurt sphere does not scale D (no table read on this path). Confidence high.
- Close range: D + D/2.
- Counter hit when the victim's move has damage and its frame is before its first active frame, or either ki
  timer (+0x16A) runs, or victim +0x12E is set (not juggled, +0xF7 clear): D = 12D/10 + bonus; bonus =
  3*Vdmg/10 * f / startup (Vdmg the victim's move damage, f its frame), or 3D/10 when a ki timer / +0x12E caused it.
  Tekken 3's bonus is 0.09*Vdmg*f/startup.
- Airborne victim: first air hit (n == 0, or n == -1 after a tag-in) x 80/100, later hits x 1/2 (Tekken 3's).
- Partner anger: attacker +0x400 set and no ki timer: x 13/10.
- Block (victim facing): chip = D/10 (20.12: D * 0xA000 / 100), only when the match flag byte 0x43B804 is set or
  the attacker's ki timer runs; otherwise 0. Tekken 3's chip is D/10 + 2.
- Health -= D << 12. Recoverable cap +0x3F8 -= 0.6 D (gp-0x7F88); on hits that set +0x12A (the first air hit)
  the cap drops by 1.4 D but not below health, so juggle damage is not recoverable.
- When gp-0x71FC is set the hit's damage is undone (health restored); game mode 7 (practice) refills health and
  cap to max after every hit.

## 10. Tag system

- Team table 0x440940, 0x60 per side, 0x30 per member: +0x0 body, +0x4 variant byte, +0x8 present, +0xC health,
  +0x10 recoverable cap, +0x14 max health, +0x18 / +0x1C display bars, +0x28 anger (100 at round start).
- Player structs: 0x879640 + side * 0xB8A0 (one per side). +0x0 active member, +0x2 partner, +0x1E side, +0x402 no
  partner (solo), +0x400 angry.
- Tag button: command bit 14 (0x4000); the stances send it to alias 5398 with transition 34 (turn to the screen
  axis), 90 frames, posture bit 24. Its list branches at frame 30 on 0x54-0x5B / bit 0x80 to the run-off variants
  (5394-5447); each ends on condition 0x5C (off screen) or its END, both to alias 5397 with transition 0x6021
  (type 33, swap bit, 0x4000).
- The swap (0x36A70C -> 0x3D3288(side)): +0x0 and +0x2 exchange; health and cap load from the new member's team
  entry; body, moveset and aliases are rebuilt (0x35C8D0, 0x368168), volumes (0x377F58); the new fighter starts
  alias 5397 of its own moveset (the entrance, 35 frames for Paul, then stance 58) at the old fighter's root,
  facing the opponent. If the opponent is airborne, its juggle counter is set to -1, so the next hit counts as a
  first air hit (tag combos). Invulnerability during the tag was not found in code.
- Round end: 0x36F408 scores each side as the members' health / max health (x 4096), averaged; any present member
  at 0 health makes the side's score 0, so one K.O. loses the round. At time over (gp-0x74C0 <= 0) the higher score
  wins; equal is a draw (0x36F56C).
- Recoverable health (0x3C1900, every frame while the round runs and nothing is paused): the benched partner's
  health (+0xC) rises by max / 6000 per frame up to its cap (+0x10), i.e. 1 % of max a second. The active fighter
  regains nothing, except a solo Unknown (same rate, only while not attacking). The active fighter's health and cap
  are copied to its team entry each frame.
- Anger (0x3C1B38, 0x3CF278): each clean hit with damage on the active member adds 4 to the partner's anger (6 for
  a zero-damage hit), +-1 or 0 by the relation table 0x5207F0 (76 x {partner body, variant mask, active body,
  variant mask, effect}; body 0x22 = any; effect -1 = 3, 1 = 5, -9 = none; e.g. Paul / Law +1, Paul / Kuma -1,
  Law / Baek none). An active member with anger >= 120 is angry (+0x400, x1.3 damage) and loses 1 a frame; when the
  partner's anger is >= 120 the active member's resets to 100.
- Partner requirements: req 0x64-0x84 (partner's moveset), 0xA7-0xC8 (partner's body), checked against team
  member +0x2 and false when +0x402 (solo).
- Not found: the tag-in invulnerability, how tag throws / tag combos start beyond the partner gates.

## 11. Round rules

- Match settings byte block at 0x43B7F8: +0x0 game mode (1 / 6 handicap health, 2 survival, 7 practice, 9 fixed
  160), +0xB round time option, +0x1E rounds option, +0xC chip flag.
- Rounds to win (gp-0x7480) = min(5, +0x1E + 1); round limit (gp-0x747E) = 2 * wins - 1; round number gp-0x71EC.
- Round timer (gp-0x74C0, frames, 0x36D640) by option 0-4: 2160, 2820, 4260, 5640, 6960; option 5 does not count.
  The display shows (t * 6 / 7 - 1) / 60: 30, 40, 60, 80, 99 seconds, each displayed second 70 frames.
- Life (0x3C1350): base = life option table 0x51F5A0[gp-0x7494] (or [gp-0x7492] in one sub-mode); the only
  writer (0x3B1488) sets both to 3, so every normal match uses 140. Modes 1 and 6: per-side value * 140 / 100.
  Mode 9: 160. Then x the body's percent / 100; a solo Unknown x 1.5.
- Mode 2 (probably survival) restores 15 % of max to health and cap between rounds (0x3C1620).
- Arenas: no wall or ring-out test was looked for. The K.O. sequence timings and the default round-time / rounds
  options (not written by code found here) are not decoded.

## 12. Camera and screen sides; CPU AI

- Camera struct at 0x415808 (+0x4 yaw in 12-bit units); framing code 0x35B760-0x35BC40, per-frame writer 0x35B7B0.
  Not decoded further.
- Screen side: on_right[side] (gp-0x65D0) = this side's projected x (+0x624) >= the other's (0x35B9A8); the
  previous frame's copy is at gp-0x65C8. Player +0x48 = (0x400 - camera yaw) << 4 (+0x8000 for side 1), +0x50 /
  +0x52 the axis nearest the body's facing and its perpendicular, +0x54 the facing quadrant against the camera's
  axis, +0x4C the direction from the opponent to me against the camera (0x370F28, 0x371330).
- CPU AI: not located with certainty. Tekken 3's LCG (x * 5 + 3) does not appear. Cancel-list walkers outside the
  move code cluster in 0x37E000-0x386FFF (about 40 loops of 14-byte entries), with per-side state at 0x8B7EF0 and
  a per-frame call per player (0x37D668 from 0x36DC9C); this is the likely AI, probably Tekken 3's candidate-list
  design. Low confidence.

## 13. Round flow (third pass, read in code)

The round machine is 0x36E198(p0, p1), one call a frame; its state is gp-0x7268 (jump table 0x500420):

| State | Code | Does |
|---|---|---|
| 0 call | 0x36E1FC | counter gp-0x7208 + 1; inputs off; ends at frame 151 (round 1) or 91 (later rounds): 0x36C490 sets +0x410 / +0x414 (input on) for both, state 1 |
| 1 fight | 0x36E250 | 0x36F3C8 ends the round when either side's +0x40C is 0 or the timer gp-0x74C0 reaches 0; the timer counts down unless the time option is 5 |
| 2 K.O. / time up | 0x36E3CC | gp-0x71C8 counts frames; see below |
| 3 result | 0x36E59C | win poses or the team-lose shot, camera mode gp-0x7200 |
| 4 pose | 0x36E888 | waits, then state 5 (next round or match end, outside this machine) |

**Round start motions** (0x368398, on the first call frame; the pad is gp-0x74B0 / gp-0x74AC for side 0 / 1):
- Round 1: alias 4 if buttons 0x90 are held, alias 6 if 0x60, else rand() & 1 picks 4 or 6. A move of 151 frames or
  more starts at frame `frames - 149` (so it ends with the call), else at frame 1. All intros on the disc are 150
  frames (alias 4 and 6 are the same move for most of the newer characters).
- Later rounds: alias 5 (30-240 frames, per character), from frame 1. The fight starts at frame 91 whether it ended
  or not (inputs come on; the move's own END link returns it to the stance).
- Body 33 (Unknown) in the mode tested by 0x3B6220: alias 9 (move 6591, shared, 150 frames).
- The button bits are the game's pad word (not yet tied to LP/RP/LK/RK); Tekken 3 maps □/△ to alias 0, × to 2.

**Announcer** (0x36EC50, by gp-0x7208; voice ids go through 0x374CD8, section 14):

| Call | Round 1 frame | Later frame | Id |
|---|---|---|---|
| "Round" | 61 | 1 | 0x5A; 0x5C ("final round") when the round number >= the round limit and neither side is solo |
| number | 100 | 40 | table 0x500458 by round: 0x48-0x4F for rounds 1-8, none for 9 (modes 5 / 6: 0x50); skipped on the final-round call |
| "Fight" | 150 | 90 | 0x5B |

These are Tekken 3's frames exactly (61 / 100 / 150 and 1 / 40 / 90). Text (0x36EDE0): the round banner shows on
frames 66-104 (round 1) or 6-44, "READY!" from 106 / 46 until the fight starts.

**Round result** (0x36F570, flags gp-0x7444, scores +0x408 from 0x36F408 in 1/4096 of full health):
bit 0x01 time up (both scores > 0 when the timer ran out); 0x02 K.O.; 0x04 double K.O. (with 0x02 and 0x20); 0x08
perfect (the winning team's two members still at their caps, +0x2C == +0x10, and gp-0x6A80[side] == 0); 0x10 great
(winner's score <= 0xCC, 5 %); 0x20 draw (equal scores).
0x36F6B8: the higher score gets +0xB4 = 1 and a win (+0x5A); the other +0xB4 = -1. A draw gives both sides a win and
both +0xB4 = -1, and lowers the round limit gp-0x747E by 1; when that ends the match on a tie the settings byte +5,
the wins, then 0x36F678 (which side is human, gp-0x7436) pick the winner.

Call at frame 1 of state 2 (0x36EEE0): time up 0x54, perfect 0x58, double K.O. none (id 0), great 0x57, else K.O.
0x59. Text: "TIME UP", perfect, "DOUBLE K.O.", "K.O.".

**State 2 timings** (counter gp-0x71C8, from the end of the fight):
- Every frame each fighter's input goes off (0x36C480), except a fighter with +0x40C set and +0x418 clear on a K.O.
  (keeps control, as in Tekken 3); on a time up the winner (+0xB4 > 0, +0xCC bit 2) is set to +0x420 = 4,
  +0x430 = 0x1000 instead.
- K.O. (0x36F920): frame 16 sets settings +0x39 = 1, camera mode 3 (replay camera) and prepares the replay
  (0x364698); frame 17 holds while the replay runs (gp-0x72A8 != 5) and restarts it while +0x39 < 3, so the K.O.
  replay plays twice; frame 50: if the match is over, 0x3D2B88; frame 110: 0x3660A8 / 0x3D4718 (the replay ends);
  frame 135: done. Seen in PCSX2 (the counter read each frame through PINE): it holds at 16 / 17 for 64 frames while
  the replay shows three shots of 21-22 frames at full speed, each the ring's last 20 frames (`replay_frames`,
  `replay_passes` in rounds.json); the last shot then stays on the live fight, the round's text shown again, to 135. In one mode (gp-0x7434 set, 0x3ABCC0 false) there is no replay: done at frame 120.
- Time up: frame 50 as above, done at frame 160.
- Otherwise done at frame 600.
- No slow motion was found in the K.O. path itself; the replay's speed is in 0x364B50 (not read).

**Win poses** (state 3, 0x36E77C): both players get +0x127 = 1; 0x3B5DB0(p0, p1, 0) replaces the loser's player with
the winner's partner (team entry +2 of the winner's side; body, aliases and volumes reloaded), placed at
x 0x400, z -0xC00 from the winner; in a solo match the loser is hidden (+0x127 = 0). 0x368528(winner, other,
frame-counter parity) picks the pose from the winner's pad: 0x80 -> 0, 0x10 -> 1, 0x40 -> 2, 0x20 -> 3, nothing ->
gp-0x74D8 & 1. Both play `alias 0x12A2 + pose` (4770-4773) of their own moveset (0x3685B8), so a tag team poses as
a pair. Costume slot 33 (player +0x2E, file 444: Xiaoyu's school uniform, costume 4) never uses pose 0, the
handstand: it takes 0x12A3 + gp-0x74D8 % 3 (PCSX2, square held: costumes 0-3 do the handstand 0x12A2, costume 4
0x12A5); no other costume or character has a rule. Then, when 0x3B5DB0 found the winner and the partner in its pair
table (0x43AC38 for the win, 0x43B140 for the team lose; 0x1C bytes: two members `{character, costume (-1 any),
alias, x, z, facing}` and a word, ended by character -1), each plays its member's alias at its x, z and facing
instead (player +0xB12C; not exported yet). Camera mode 0xE (side 0 won) or 0xF.
State 4 then lasts 300 frames (gp-0x725C), or until START (pad bit 0x800, from frame 2), or until gp-0x7258 is set
(0x36E888): with camera mode 0xD at frame 75, 0x11 / 0x12 (the team lose) at 60, 0x10 as soon as a winning player
(+0xB4 > 0; the winner and the partner posing beside it) reaches its motion's last frame (+0xC4 >= the length - 1).
Seen in PCSX2: 188 frames, the first pose to end.

**Team lose** (0x36E6C8; the arcade flag settings +0x1F set, the match over and the human side lost): 0x3B5DB0(...,
1) puts the loser's partner in the winner's player, both play alias 0x12AC (4780, 130-255 frames), camera 0x11 / 0x12,
state 4 ends after 60 frames. Alias 0x12AD (4781) is not used by this code.
Other modes (1, 2 with the draw flag, 6, 9) skip the poses and wait 300 frames.

| Character | intro (4 / 6) | later rounds (5) | win poses 4770-4773 (frames) | team lose 4780 |
|---|---|---|---|---|
| Paul | 6518 / 6519 | 6521, 54 | 240, 118, 159, 255 | 181 |
| Law | 6522 / 6523 | 6524, 98 | 255, 148, 230, 150 | 130 |
| Lei | 6549 / 6550 | 6551, 240 | 218, 200, 255, 255 | 181 |
| Jin | 6515 / 6516 | 6517, 128 | 230, 200, 209, 179 | 130 |

The full table (all 39 roster entries; Mokujin, Tetsujin and Unknown have none, they copy) comes from
`dump_round_sound.py`. Characters added after Tekken 3 repeat their poses (4772 = 4770, 4773 = 4771).

Confidence: high on the state machine, call frames, aliases and pose choice; medium on the meaning of +0x127 and on
which pad bits are which buttons; the round-time and rounds options' defaults are still not written by any code
found (the settings block 0x43B7F8 is filled by copies).

## 14. Sound (third pass)

Play: 0x374C60 / 0x374CD8(id, side) mask the id with 0xFFF; id 0 plays nothing. 0x3755C8 maps ids >= 0x320 (below);
0x374B48 finds the id's group in the **group table 0x42EBD8** (62 x 16 bytes: `u32 entries, u32 bank bytes, u16 kind,
u16 first id, u16 count`); the sample is `id - first`, the n-th `VAGp` of the group's bank file. Sample entry (8
bytes): `u32 SPU offset, u16 pitch (0x1000 = 48 kHz), u8 volume, u8 channel class`; class 6 and 8 add the side (one
voice per player). Kind 0 is resident, kind 1 loads with the arena, kind 2 is a voice bank (four slots at
0x8A7900: group, SPU address).

| Ids | Group | File | Content |
|---|---|---|---|
| 0x000-0x05F | 0 | 1 (96) | common effects; 0x48-0x5C the announcer (8 kHz) |
| 0x060-0x0A7 | 1-20 | 187 + 7k for arena k (3-7 each) | arena sounds: floor steps and landings, ambience |
| 0x0A8-0x0B5 | 21 | 327 or 334 (14, identical) | not placed |
| 0x0B6-0x0D1 | 22 | 45 (28) | floor sets and effects of another mode |
| 0x0D2-0x31F | 23 + voice row | 344 + 3 * costume slot (FORMATS.md) | a voice row's bank; the row is player +0x2C, the roster's byte 6, not the body (Bryan 13, Panda 12, Tiger 21); bank sizes and sample counts match |

Ids 0x321-0x326 (0x3755C8, jump table 0x500900) depend on the arena gp-0x743A: 0x321 = floor table 0x5007F8[arena]
word 1 or 2, alternating (footsteps), 0x322 = word 0 (landing / fall); 0x323 arena 0 -> 0x63; 0x324 arenas 9, 11,
18 -> 0x85, 0x8C, 0xA4; 0x325 arenas 1, 15 -> 0x67; 0x326 arenas 1, 15 -> 0x9A; otherwise none.

**Move sound lists** (0x375B60(self, opponent), every frame): up to 3 entries `type << 12 | index` (jump table
0x500CC0):
- 0, 6: nothing (6 is Tekken 3's swing; Paul's jab, move 806, has only 0x600F and the hit sound 0x7008).
- 2, 3: a voice on frame 1 (type 2 only when no voice cooldown runs; it then sets the cooldown to the body's
  table +0xC, 10 frames, 6 for bodies 21 and 35); 5: a voice at once. Voice (0x375AA8): table 0x501808 + 16 *
  voice row (+0x2C) points at four s16 counts (types 2-5) then the ids; id = ids[sum of the earlier counts + index]. Paul: 8, 5, 3, 5
  voices, ids 0xD3-0xE7 (group 23, samples 1-21). Only types 2-5 are voices; a fighter with +0xB6 set is silent.
- 4: an effect `index` on the fighter's side, each frame the entry is reached.
- 1, 7: hit sound, only on the frame the move connects (+0x132), on the victim's side. Guarded (victim +0x130):
  always 0x7010 (id 0x10). Not guarded: the generic ids 0x7000, 0x7006, 0x7007, 0x7009 go through 0x3769D0 by the
  attacker's damage +0xCA (bands < 13, < 23, < 33, more): move end bone (+0x28) >= 0x0C -> 0x09, 0x0C, 0x0B,
  0x0D; end bone < 0x0C -> 0x06 or 0x23 (random), 0x07, 0x08, 0x0A (tables 0x500D10 for attacker +0x2A = 5, 7, 10,
  18, i.e. Nina, Xiaoyu, Julia, Anna, and 0x500D30 for the rest; their first two columns are equal, the other
  columns are not read). Other ids play as written. Victims whose voice row +0x2C is 16 / 17
  (Mokujin, Tetsujin; with gp-0x71D8[side]) or 18, 30, 34 (Gun Jack, Jack-2, P. Jack) take special handlers (0x376818, 0x376790, 0x376728).
- 8: a timed script, index into 0x505C10 -> 4-byte steps at 0x501A78 `{u8 frame, u8 kind, u16 sound}` (kind 0
  effect, 1 own voice or effect, 2 the opponent's, 3 0x376578); sound 0x13 starts a footstep pair.
- 9: 0x376578(self, opponent, index).
Also: the hit that K.O.s plays 0x700A (end bone < 0x0B) or 0x700D; frame `jump start` (move +0x19) plays 0x1012; fighter flag +0x136
starts a footstep pair (0x1321 now and 4 frames later); +0x137 plays 0x1322.

**Music**: the fight's BGM is table 0x4FB9C0[arena] (u16), sent to the IOP as command 0x40 (0x39A0C0). Names
(0x455D80): 0 SELECT, 1 EMBU, 2 RESULT, 4 ENDING REPLAY, 5 NAME ENTRY, 6 STAFF ROLL, 8 CONTINUE?, 9-22 the arena
themes. Arena numbers (stage select 0x440770):

| Arena | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Name | Paul | Law A | Yoshimitsu A | Xiaoyu | Hwoarang | Lei | Ogre | School A | Jin A | Nina A | Eddy A | King | Heihachi | Eddy B | Unknown | Law B | School B | Jin B | Nina B | Yoshimitsu B |
| BGM | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 19 | 22 | 10 | 16 | 17 | 18 | 11 |

The A / B arenas share a theme. One mode (settings mode 8) picks 9 + rand() % 14, or 12 (0x3BFEC0).

**BGM id -> file** (0x397A00 at boot): for each id 0-27 the table 0x50CD00 (10 bytes: u8 kind, s8, u16, s16, u16 file,
s16 loop sector) is sent with the file's TEKKEN.BIN sector and size (from the table at 0x509E40) to the sound driver,
RSPU2DRV.IRX (Namco's, on the disc: `0x205E` calls 0x3110, a 20-byte entry per id; command 0x40 at 0xCE8 plays one).
Id 7 starts 0xCF7 sectors into its file. The files:

| Id | 0 | 1 | 2 | 3, 4 | 5 | 6, 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15-22 | 23-27 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| File | 2 (a bank) | 21 | 17 | 44 | 19 | 20 | 18 | 22 | 24 | 25 | 26 | 27 | 23 | 28-35 | 36-40 |

So the guess "file = id + 13" was wrong for 10-14 (Lei's theme is file 23). Kind 1 entries keep a restart point (the
loop sector, negative read as 0), kind 3 (id 3) clears it; kind 2 (id 0) is the select screen's two-sample bank.

Confidence: high on groups, ids, voice and hit selection, arena music ids and the BGM -> file map; medium on which
ids loop.

**Levels measured** (PCSX2's own capture of an arcade fight, Xiaoyu against Julia, BGM 34; each sample found in the
capture by normalised cross-correlation at 16 kHz, its gain the least-squares scale of the sample's WAV; the music's
gain fitted below 150 Hz, where the PS2's resampling does not smear it): music 0.534 of the stream; footsteps (001/067,
volume 25) 0.39-0.49, median 0.43; announcer and the CONTINUE countdown (001/083, 085, 073-078, volume 29) 0.48-0.55;
a voice (479/008, 25) 0.46; hits 001/015 (21) 0.44-0.46. So a sample plays at about volume / 58 of its WAV, and the
music at 0.82 x its BGM entry's volume / 16384 (BGM 21: 10649): OpenKumite's mix gives footsteps 0.43 and the music
0.552 (0.85 x, 0.3 dB louder) in its own export of the same fight; within the measurement's spread, left as it is.

## 15. Tag (third pass)

- No tag invulnerability exists in code. The only hit immunity is +0x14E: set to 4 frames on a hit that dealt
  damage or hit an airborne victim (0x37202C), counted down each frame (0x3707F4); while it runs the hurt-sphere loop
  (0x377198) skips the victim. Posture bit 24 (the tag state) is tested only at 0x36AA70 (section 7). So the leaving
  fighter can be hit until the swap, and the entering one from its first frame.
- The benched partner is not simulated: it exists only as its team entry (health regeneration, anger; section 10).
- Tag throws and tag combos: nothing beyond the partner gates and the juggle counter reset (section 10) was found.
- Camera during a tag: not decoded (section 16).

## 16. Fight camera (entry points; decoded in section 18)

0x38D518(p0, p1, &mode) runs the camera every frame on mode gp-0x7200 (jump table 0x507F70, 20 modes; state at
0x433330). Mode 0 is the round-start fly-in (0x395400), then mode 1, the fight camera (0x390E08 resets it with an
offset of y -1200, z -8000 and the field of view from state +0xA4; framing 0x38EC08 per fighter, 0x38F768, 0x396CF8,
0x391048); mode 1 switches to 0xD when 0x38CFC0 says so. 3 = replay, 0xD-0x12 the end shots of section 13.
0x35B7A8 / 0x35B858 copy a 36-byte eye / target block into 0x415808 and build the view (0x35B4B0); 0x35B8B8 turns a
segment into yaw and pitch. The per-move camera id and the framing are in section 18.

## 17. Hit effects

Not located. The damage function's only effect calls are on chip damage: 0x35BFF8(2, attacker) when the victim
survives, 0x35C0D8 when chip K.O.s (both lead to 0x35BF10 / 0x3A2FF8, likely vibration and an effect).

## 18. Camera (fourth pass, read in code)

Tekken 3's camera module (ok_tekken3_converter `docs/reference/tekken3/camera.md`, OpenKumite `camera.rs`) was
ported whole: the same CAM block idea, five key slots, the sampler, the second-order yaw chase, the four-pass framing
solver, the orbiting key blend, the director state machine and the per-move camera lists. Everything is integer
(no FPU) except the projection set-up 0x38DA28. Angles are 4096 a turn, kept `<< 6` inside the module; y is down.
The differences are constants scaled for the PS2 screen and a few additions, listed below. Confidence: high where a
constant is named with its address; medium on the meaning of the new director states and of the replay passes.

**Blocks.** CAM block 0x433330: +0x0 director state, +0x4..+0x14 weights w0-w4 (4096 = 1) of key slots 0-4,
+0x18..+0x28 last frame's weights, +0x30 thrower side, +0x34 orientation, +0x38 delay, +0x3C / +0x40 blend counter
/ length, +0x44 throw latch, +0x54 selected list value, +0x58 situation type (> 0) or motion (< 0), +0x60 move
that triggered it, +0x64 / +0x68 / +0x6C slot-1 pitch / yaw / roll, +0x7C / +0x80 per-side "move already used",
+0xA0 mirror, +0xA4 projection H, +0xA8 / +0xAC H min / max, +0xB0 dolly flag, +0xB4 arena variant, +0xC4 / +0xC8
cool-downs. Key slots 0x8B90B8, 5 x 28 bytes `{eye x, y, z, pitch<<6, yaw<<6, roll, H}` (Tekken 3: 24 bytes, no
roll); slot 3 is the output, 0x38E4F8(n) applies slot n to the view block 0x415808 (+0x0 pitch, +0x4 yaw, +0x14
eye; section 16), 0x38E408 / 0x38E460 load / store a slot. Slot state 0x8B9148 + 0x3C * slot: +0x0 chase state,
+0x4 yaw velocity, +0x8 acceleration, +0xC pitch lag, +0x10 its target, +0x14 its step, +0x18 |error|, +0x20
settle counter, +0x24 moving flag, +0x28 / +0x2C vertical pull step / pull, +0x30 horizontal pull. Samples 0x433520,
2 x 16 bytes `{x, y, z, -}`, last frame's at 0x433540; screen x of each side gp-0x6FA0 (section 12).

**Projection per arena** (0x38DBC0 at round reset, table 0x507FE0, 25 x 5 s16 by arena gp-0x743A): H = 615,
clamp [600, 900] ([500, 900] for arenas 5, 11, 12), dolly flag 1 (0 for arenas 3 and 7), variant 0; entry 24 is
H 850, variant 1. Tekken 3 has H = 500 for every stage. The variant (Tekken 3's CAM+0x6C) halves situation pitches,
drops pitched-up ones and changes the random pick of the camera lists (below); the dolly flag (Tekken 3's CAM+0x6A)
turns a camera motion's zoom into a dolly.

### 18.1 Fight camera, slot 0 (exact)

Per frame in a fight (mode 0xD, 0x38C1E8): sample (0x38EC08), director, then if w0 > 0 `0x390FA8(p0, p1, snap =
last w0 < 1, dist)` = yaw controller 0x38F7F8 + framing solver 0x38FD90, then the mixer.

- **Depth**: dist = H * 1900 / 256 (0x76C), 4564 at H 615 (Tekken 3: constant 5517); the solver raises it to at
  least 800. Slot 1 uses dist * 114 / 128 (same as Tekken 3).
- **Reset** (0x390E08): eye (0, -1200, -8000), pitch and yaw 0, H from +0xA4 (Tekken 3: y -1450); slot states
  cleared.
- **Sample** (0x38EC08) per fighter: x / z = the logical position (+0x4 / +0xC), or the root (+0x51C / +0x524)
  while the fighter is in a throw (+0xE0 != 0) or its move's flags bits 20-21 = 1; changes between the two are eased
  1/4 a frame. y = chest (+0x70C) - 290 (Tekken 3: the same 290), kept from last frame when bits 20-21 = 2. Equal
  samples are pushed 1 apart. Camera ids 0x17-0x1A (below) add smoothing.
- **Yaw controller** 0x38F7F8: Tekken 3's, constant for constant: target = ratan2 of the sample line, oriented so
  the side with the smaller screen x (+0x624) stays left; start / settle 0xE38 (5 deg), brake zone 0x38E4 (20 deg),
  freeze above 0x1AAAA (150 deg) or while the round frame counter gp-0x74C8 < 200; acceleration 0x600 (0x300 when
  braking), damping 14/128, velocity cap 0xDDD0, settle after 32 frames; pitch lag above 20 deg =
  sin((err - 0x38E3) * 4096 / 0xC71D) * 279616 / 4096, eased 1/16, at most 0x100 a frame. Move caps by camera id
  (0x38F6B0): 0x13 -> 0x200, 0x14 -> 0x180, 0x15 -> 0x80, 0x16 -> 0x40, cap = value * 0xDDD0 / 256 (Tekken 3 ids
  0x27-0x2A). A fighter whose flags bits 20-21 = 2 uses its root instead of the sample.
- **Framing solver** 0x38FD90 (same four passes as Tekken 3; 0x391358 is a copy used by the replay paths):

| Constant | Tag | Tekken 3 |
|---|---|---|
| H / depth | 615 / 4564 | 500 / 5517 |
| centre point height | -450 | -450 |
| depth push limit (new) | at least -H * 3300 / 256 per frame | - |
| margin per fighter | 150 | 150 |
| fit width | 392 px (target half width 196) | 280 (140) |
| horizontal pull | in 1/32, out 1/8, decay 1/32 / 1/8 | same |
| lateral easing | 1/8; 1/16 while an id 0x17-0x1A move runs | 1/8 |
| dead zone | +-68 px | +-48 |
| centre drop | 140 - 450 * H / dist (80 px); 180 for a pitched-up slot 1 | 100 (60 px); 128 |
| slot-1 height offset | (y + 1000) * 512 / 640, +-28 a frame | * 368 / 640 |
| keep the top fighter above | 156 px | 112 |
| vertical pull | out 1/8 (10/128 with ids 0x17-0x1A, 4/128 with 0x19), at most 800; in 1/16, at most 1200 | 1/8, 1/16 |
| moving flag | XZ step squared > 1000 | same |

  The pixel constants are Tekken 3's x 1.4 (392 = 280 x 1.4, 196, 68, 156) and the drop 140 / 180 is 100 / 128 x 1.4;
  so the same algorithm frames the pair a little wider on the PS2's 640-wide screen. New: when the eye must come
  forward (D.z < 0) during an id 0x17 / 0x18 / 0x1A move by a fighter with +0xF3 set and posture bit 0x400 or
  +0x135, D.x takes D.z * 0x17FA00 / H instead.
- **Mixer** (jump table 0x507F20 on the mask of weights > 0): {0}, {1}, {2}, {4} apply; {0,1} and {0,2} blend
  (0x38E6D0, orbit blend as Tekken 3); {0,4} and {1,4} S-curve sin(w/2 - 0x400)/2 + 0x800; else reset.

### 18.2 Per-move camera scripts

The camera id is **move flags >> 23** (9 bits, 0-0x1FF; section 1 says "top byte", it is bit 23 up).

| Id | Effect (code) | Moves |
|---|---|---|
| 0 | none | 6816 |
| 1-0x10 | random situation camera (lists of types 15-26) | 87 |
| 3 | also aborts a running special camera (states 3 / 6 back to 1) | 40 |
| 0x11 | not restarted by 0x391170 | 13 |
| 0x13-0x16 | yaw speed cap (above) | 33 |
| 0x17-0x1A | smoothed sampling and softer framing; 0x18 = the tag run-off moves (5394-5398, 5427-5447), 0x19 = the tag entrance 5397 and 5399-5426 | 181 |
| 0x1C-0x20, 0x23-0x66 | situation cameras by fighter (0x3943A8, tables 0x509700 / 0x5096E8 / 0x509820, 12-byte rows matched on player +0x28 and +0x10C) | 18 |
| 0x69-0x110 | throws and specials: a list mixing situation types and camera motions | 202, almost one id per move |

Lists (0x393A90; table 0x434C78, 0x134 x 12 bytes `{u32 list, s16 count, s16 total, s16 split, s16 first
mirrored}`; entries `{u16 flags, s16 weight, s16 value, s16 value when mirrored}`; weights made cumulative / 4096 at
boot by 0x393898): r = rand() & 0xFFF picks the first entry whose cumulative weight >= r (variant 1 / 2 limit r to
before the split / mirrored part). value < 30 -> situation type value + 1 (identity table 0x508B60); 30-561 ->
camera motion value - 29; -1 none. The mirrored value is taken when the orientation +0x34 is set. `dump_camera.py`
prints every list. Example: 0x69 = 30 % type 18 (or 26 mirrored), 30 % type 28, 40 % motion 1 (2 mirrored).

**Situation types** 0x508908 (30 x 20 bytes `{in frames, hold, pitch[4], yaw offset[4]}`), Tekken 3's layout and
values: types 1-15 blend in over 35 frames, 16-30 cut; yaw offsets 0, +-341, +-682 (0, 30, 60 deg); pitch 0, -102
(9 deg up) or 113 / 284 / 455 / 341 (10-40 deg down, one picked of four at random); pitch x 32, yaw x 64 into slot 1
(0x394110). Slot 1 is framed at depth dist * 114 / 128.

**Camera motions**: 485 in disc file 12 (`012.bin`, 506 656 bytes, the "not identified" file of FORMATS.md), u32
offsets at 0x508130 (by motion - 1); streamed in 64-byte pieces (0x3923F0). A motion: a flag byte, then a frame each
up to the next motion's offset (motions 1 and 2: 109 frames): a u16 of eight 2-bit codes (low bits first, channel 0
first) and the deltas they name: 0 none, 1 the channel's last delta again, 2 an s8, 3 an s16 (little-endian); the
last delta is kept per channel (0 after a code 0). Each channel adds its delta to its total (0x8B92A0, zeroed at the
start; s16 for 0-5, s32 for 6-7): channels 0-5 to the shot 0x8B92D0's s16 +2, +0xA, +0, +4, +8, +0xC (jump table
0x5088D0), 6 and 7 to its s32 +0x14 and +0x10. Checked: 484 of 485 end exactly at the next's offset, the last one
byte before the file's end. The shot: +0 / +2 / +4 the eye **from the target**, +8 / +0xA / +0xC the target, +0x10
the fight camera's share (128 all of it: motions start and end at 128 and go to 0 for the shot proper), +0x14 the H
the shot was made for (703 on 397 motions, 955 on 38). Motion 1's frame 48: eye (-3484, -539, -2835) from the target
(-454, -869, 322); motion 2 is motion 1 with z negated.

Play (0x3928B8 start / 0x392B68 a frame): at the start the flag gives the yaw from the victim's facing f (+0x1A,
65536 a turn; jump table 0x5088F0): 0 and 4 0x400 - f/16, 1 0x800 - f/16, 2 -f/16, 3 and 5 -(f/16 + 0x400), else 0;
the anchor is the victim's position (+0x4..+0xC). Each frame: z of eye and target negated when mirrored (CAM+0xA0);
with the arena's dolly flag (CAM+0xB0) the eye offset is scaled by the live H / the shot's H and the shot is seen at
the live H, else at the shot's H; eye = target + offset (s16); both turned by the yaw (x' = cos x - sin z, z' = sin x
+ cos z, >> 12) about the anchor; slot 2 = look-at(eye, target) (0x38E308); it returns the shot's share. While it
plays, a new move of the thrower (still +0xE0 > 0, camera id not 0x11) re-picks (0x392888): a camera id under 0x68
picks anew from its list (0x38D1B8: a situation is held, state 2; the frame is not drawn), 0x9E then 0x9F starts the
new list's motion with yaw -(f/16 + 0xA00) from the victim's root (0x38D290), else the same entry of the new move's
list restarts from its first frame (0x393BF0).

**Director** (0x38C1E8 each fight frame, state CAM+0, jump table 0x507EE0, 15 states; weights w0 +4 .. w4 +0x14, last
frame's +0x18 ..). In order: sample; cool-downs +0xC4 / +0xC8 down (+0xC4 set to 30 while the state is not 1); the
throw latch +0x44 (the thrower's side) freed when that side's +0xE0 is 0; the state; then if w1 > 0 slot 1
(0x391170, snapped when last w1 < 1; a situation frames slot 1 at its pitch / yaw at depth dist x 114 / 128), if w0 >
0 slot 0 (snapped when last w0 < 1), then the mixer (0x507F20 on the mask of weights > 0: {0} {1} {2} {4} apply,
{0,1} {0,2} orbit blend on w0, {0,4} {1,4} the S-curve on w0 / w1; any other mask resets 0x38C0F0).

- 1 (the fight camera): when the latch is free and a fighter has +0xE0 > 0 (the thrower; the victim's is < 0), unless
  both moves' camera ids are 0x11-0x68: side = the thrower, orientation +0x34 = the thrower right of the other on
  screen (+0x624; a tie -1). The thrower's move list (camera id, 0 when 0x134 or more; 0x393A90): r = rand() &
  0xFFF (variant 1: randint(split)) picks the first entry whose cumulative weight >= r; the value (the mirrored one
  with the orientation) gives a situation type (value < 30: 0x508B60, value + 1) or a motion (value - 29); the
  mirror +0xA0 = orientation and flags & 3 = 1. A situation: 0x3910D0 draws randint(0xE3), takes the fighters' axis
  (the screen-left one first), one of the type's four pitches (rand & 3, << 5; halved on variant 1) and yaw offsets
  (rand & 3, << 6; subtracted with the orientation); state 3 with a 16-frame delay, count = length = its frames in;
  a cut when they are under 2. A motion: state 6.
- 3: the delay (an id-3 move ends it: state 1), then w0 = count / length, w1 = 1 - w0, count down; at -1 state 2.
  When the throw ends (below) with the blend under way: the count and length = the type's hold, then state 1 (w1 0),
  5 (w0 > 0: slot 3 frozen into slot 4, w4 = 1) or 4.
- 2: w1 = 1; when the throw ends, count and length = the hold, state 4. 4: w1 = count / length back to 0, then 1.
  5: w4 = count / length, w0 = 1 - w4 (the S-curve from slot 4), then 1.
- 6: the motion (started on the first frame, delay -1 to -2), w0 = share x 32, w2 = 1 - w0. An id-3 move: 8 frames
  of state 5 (state 1 if the share is 128); the thrower's +0xE0 < 0: 35 frames of state 5; else when the share has
  been 0 (delay -3) and is 128 again: state 1.
- "The throw ends" (0x393CB8, thrower t, other o): t no longer +0xE0 > 0 and o not < 0; t's move camera id 3;
  else never on a cut; o's move posture bit 0x400000; else the frames to t's move's last timed property op 0x0C
  and to o's op 0x0D (frame - move frame +0xC4, -1 without): at the later one's 0 (not while t's +0x17A is 0x20).
- A chain throw (the thrower's new move with camera id 0x71, 0xA1-0xA3, in a situation; 0x38D3D0) picks anew: a
  motion starts at once with yaw 0x400 - f/16 from the victim's root; a situation is held (state 2, w1 = 1).
- 7-15: the per-fighter situation cameras (0x38DEA0, checked in states 1, 2 and 6; once a move, +0x7C / +0x80
  holding the move that set one): 0x3943A8 looks at each fighter's move: camera ids 0x23-0x66 (35 moves; alias
  5459 in every set has id 0x29); once the other fighter is down (+0x131, set by the damage that K.O.s it at
  0x372414 / 0x37271C, cleared at the round's set-up) also 0x1C-0x20 and the rows (`rows`) by the mover's character
  (+0x28) and alias (+0x10C, the move's alias +0x184 copied as it starts), whose script is the entry's first, second
  or third by +0x13A / +0x13B (set by the damage function); a kind-4 script of ids 0x23-0x66 only on a fighter down; the list (`fighter_cameras`) picks a script by rand & 0xFFF; the camera looks at the other
  fighter (its root; +0x39C for kinds 9 / 17). By the script's kind: 3 / 6 state 10, 11 / 14 state 13 (16 frames'
  delay, an id-3 move ends it), 10 state 14 / 15 (the fight camera blended into the view frozen at the delay's end),
  12-17 state 12 (from the frozen slot 4), 4-9 and the rest state 7 (from the fight camera): blended over the
  script's blend in (+4), held (8) to its blend out (+6) frames before its count (+2) ends, then back (9; 15 to the
  S-curve). Slot 1 (0x394648) by kind: 0 a situation's angle from the script's four pitches and yaw offsets (<< 6;
  pitch 0 on variant arenas), 1 an orbit of the point at that angle, the distance easing from +0xC to +8 (x H /
  955); 3-5 / 11-13 the point + Ry Rx Rz (pitch, yaw + the other's facing kept within 90 degrees) of (0, 0,
  distance), 6-8 / 14-16 an orbit at pitch and yaw (off the fighters' axis, taken on its first two frames) easing
  from / to over the count; both ease the point halfway to the other's root a frame, keep it and the eye at heights
  -1400..-900, lift the eye by +0x14, and freeze the view once the other's move has flags bits 20-21 = 2.

Slot 1's framing (0x38FD90 with slot 1): pitched up (pitch < 0) its centre drop is 180 (slot 0 140); pitched down
it is lifted by (y + 1000) x 512 / 640, y the top fighter's sample height x cos pitch >> 12 when >= -1000, eased 28
a frame unless snapped (gp-0x6FA8).

### 18.3 Other cameras (modes of 0x38D518)

- **Round start** (mode 0, 0x395400 / per frame 0x3954E8): the base is the snapped fight key; one offset only
  (0x509C20: eye z -3700, i.e. the eye starts 3700 units nearer than the fight distance along world z, target
  unchanged), eased by s = sin(0xC00 + 1024 * min(t, 90) / 90) + 4096 over the last 90 frames: round 1 lasts 120
  frames (the first 30 hold the offset), later rounds 90. w2 = min(t * 4096 / 13, 4096), so the hand-off to the fight
  camera lasts 13 frames (Tekken 3: 120 / 53 frames, 10 offsets, 16-frame hand-off). Mode 1 blends, then mode 0xD.
- **Attract / demo** (gp-0x7434 set, by boot 0x357550 and the set-ups at 0x3A97C4, 0x3B91C8, 0x3B96EC...; cleared
  at 0x3BA2CC; skipped while the game state gp-0x74D0 = 8, 0x3ABCC0; the AI uses a fixed profile in the same flag,
  section 19, so this is most likely the attract / demo fight [I]): mode 0 picks
  one of 10 programs (0x433010, 10 x {case, frames}; cases 6 / 8 / 12 / 13, 300-600 frames) and cycles modes 5-12
  (orbit 0x395D20, 0x395EE0, 0x396320, 0x396620; 0x3963C0 picks a preset of 0x509C10 by counter % 5) and plain fight
  framing. Only here (0x36D420 -> 0x393838 / 0x392E00, per frame 0x393550 in float) are the arena camera paths played:
  file id 0x40 + 4 * arena (the `190 + 7n` files), eye, target and H per frame, H clamp lifted to [1, 10000].
- **Win pose** (modes 0xE / 0xF, 0x395718 -> 0x35BB58 / per frame 0x35BD20; one camera, the A / B bit is stored but
  not used): eye at the winner's root + R (cos a, sin a), R = H * 5200 / 955 + 1440 (H the live projection gp-0x7420;
  4788 at H 615; Tekken 3 3978), a = 0x400 + rnd(+-113) - L / 16, where L (0x3D7DA8, 65536 a turn) is the angle of
  light 0's direction (0x353C88: entry 0 of the light table 0x1FC42A0, turned in game state 4) by atan2(-x, -z),
  negated: the winner is shot from beside the key light, not by its facing (in a fight, game state 4, 0x353C88
  turns light 0 by gp-0x7278, the far background's turn: 0x366C18 adds a share of each small change of the camera's
  yaw on the arenas of 0x417780 (all but 8, 9, 17, 18, 24), so the angle also follows the fight's camera history); per frame the eye closes 8 units a frame
  while farther than H * 5200 / 955 and the view eases at >> 5 (0x35BA58, as Tekken 3's camera A); y = -850
  (0x35BCDC), chest - 290 on the variant arena; the aim (0x35BA58) is Tekken 3's camera A's without its 5800
  minimum distance, its pitch not up on the variant arena. Its random number is the LCG gp-0x73FC: x = (x + 1) * 69069, r = (x >> 16) *
  0xE3 >> 16 - 0x71. Team lose (0x11 / 0x12) 0x395C90 -> 0x3959C8; 0x10 after mode 0xE / 0xF (0x395750).
- **Tag**: no camera mode; the run-off and entrance moves carry ids 0x18 / 0x19, so the normal camera follows them
  with the smoothed samples (id 0x19 also holds the sample until the fighter's projected points, +0x628-0x62C, are
  on screen, 21-492).

### 18.4 K.O. replay

Recording (0x364E7C each fight frame): a 120-frame ring (counter gp-0x6568, 0x36 bytes a side at 0x8658xx); replay
positions are half frames (mod 240), so +2 a frame is full speed and +1 half speed. At the K.O. 0x364678 sets the
start to (counter - move frame - 30) * 2 (30 frames before the K.O. move, as Tekken 3); 0x364698 (round state 2,
frame 16) checks for at least 150 frames, then starts at counter - 20 (state 2, gp-0x72A8). Passes (0x364B50, jump
table 0x4FC630; pass counter gp-0x654C): passes 0-2 run at full speed, from the rewind point counter - 118 to
counter - 85; the last pass at half speed to counter - 21; each pass sets camera mode 3 and a new shot (0x3D0748).
Shots: one of 14 random sequences (0x520F88, 4 shot numbers, one per pass) of 42 shots (0x520C40, 20 bytes: target
and follow {0 the winner's root +0x51C, 1 the loser's, 2 the loser's point +0x39C kept at the K.O. in 0x1FAD3F0
(follow: none)} (the loser: player 1 while player 0's +0xB4 >= 0; after a throw both the loser's), pitch, yaw (+ facing / 16), roll, three random ranges,
distance 3250 / 5000 / 7000 +- 100 / 500 scaled by H / 600); eye at least 1500 up, target at most 1800 up, pitch at
most 568 (50 deg). Confidence: medium on the pass bounds, which are read from the end checks only.

## 19. CPU AI

**Entry.** Match start 0x36D010 calls 0x36C420(player, is_cpu, pad) for each side. That sets +0x424 = 2 for a CPU and 1 for a human, and is_cpu goes to +0x418. Each frame, 0x36C680 switches on +0x420 through jump table 0x4FFD20:

| +0x420 | Input source |
|---|---|
| 0 | none |
| 1 | pad |
| 2 | CPU AI, 0x380A50(player, &held, &pressed) |
| 3 | replay / demo playback 0x37C950(side), state 0x8B7EF0 + 8·side, run by 0x37D668 |
| 4 | hold +0x430 (the time-up winner) |

gp-0x6A78 forces every side to mode 2 (attract mode). [C]

**0x380A50**:
- State block: gp-0x6FCC = 0x8B8078 + 1000·side (1000-byte blocks, at player +0x20).
- SELF / OPP pointers: gp-0x6FC8 / gp-0x6FC4.
- Output goes to state +6; the previous frame's output is at +8; pressed = held & ~prev.
- It outputs nothing while 0x393810 is true (flag gp-0x7434 and an arena camera path playing, 0x8BC620 ≥ 0; section 18.3). [C]

**Random numbers.** [C]
- `rand()` at 0x40FBD0 (libc): `x = x·0x41C64E6D + 0x3039`, returns `x & 0x7FFFFFFF`.
- AI generator at gp-0x64E8: `x = x·5 + 3`, seed 640 in the data.
- `randint(n)` at 0x3800D8: `(n·((rand() + ai) & 0x7FFF)) >> 15`, then the AI generator steps. Same as Tekken 3, except Tekken 3's rand returns bits 16-30 where Tag takes bits 0-14.
- Some draws use `rand() & 3` or `& 0xFFF` directly.

**Round init** 0x380568, both sides, called at 0x36D734:
- Clears the block and sets candidate states to -1 at 0x8B8848 (180 × 12 bytes: `{u32 move*, u32 cancel*, s16 state}`).
- Range thresholds: table 0x432E10 + 12·body (6 u16 each). Its own row goes to +0x3C4, the opponent's to +0x3D0. Defaults are {1408, 1728, 2304, 3072, 1792, 0}; bodies 3, 7, 11 and 16 differ.
- Moveset handlers: +0xB4 / +0xB8 from 0x38BEE0 (below).
- Profile load 0x380120 (below).
- Initial attack cooldown +0x94 = 60 + P0A·(r15 >> 8)/256; movement cooldown +0x5C = 50 + randint(48).

**Profile selection** 0x380120. The 128-byte profile is copied to state +0x344 from `0x431918 + 0x400·diff + 0x80·level`, with diff 0-4 and level 0-7. [C]

| Mode (settings +0) | diff | level |
|---|---|---|
| 4 (a streak mode, "survival" [G]; section 11 guessed mode 2; streak gp-0x7430, capped at 9999) | first row of ladder 0x432D18 (15 × `{u16 max_wins, diff, lo, hi}`, count gp-0x6FC0) with max_wins ≥ streak. randint(255) odd → row diff; even → 1 (0 if the row diff is 0). | lo + randint(hi - lo + 1) |
| 3 | 1 | settings +0x24 |
| other | settings +0xD (the DIFFICULTY option) | settings +0x24 (fight number) |

- If gp-0x7423 ≥ 8, or the result is out of range: diff 1, level 6.
- With gp-0x7434 set (the attract / demo flag of section 18.3 [I]): fixed profile 0x432D90; cooldowns +0x0A..+0x10 are halved and +0x20 is quartered (Tekken 3 does the same).
- +0x25C = max(P36, P3A) (consecutive-attack limit); +0x3DC = diff.

Ladder:

| Streak up to | 4 | 9 | 14 | 19 | 29 | 39 | 49 | 54 | 59 | 64 | 74 | 99 | 763 | 764 | more |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| diff | 0 | 1 | 1 | 2 | 2 | 3 | 3 | 3 | 3 | 3 | 4 | 4 | 4 | **0** | 4 |
| level | 0-3 | 2-4 | 3-7 | 2-4 | 3-7 | 3-7 | 4-7 | 5-7 | 6-7 | 7 | 3-7 | 5-7 | 7 | 0 | 7 |

The 764 row is exact; it may be an easter egg.

**Profile layout** [C for the offsets read, I for the names]:
- Tekken 3's 110-byte layout with 4 bytes inserted at +4, so Tekken 3 offset X ≥ 4 is Tag offset X + 4.
- +0..+3: wake-up tap intervals.
- +4 / +6: new (7/7 at EASY down to 3/5 at the top); no reader found.
- +8: cooldown base. +0x0A..+0x10: cooldown choices, e.g. EASY L0 {300, 300, 400, 400}, NORMAL L0 {150, 150, 200, 200}, HARD-to-ULTRA {10, 10-30, 30-90, 30-150}.
- +0x20 guard, +0x24 interrupt, +0x5E parry / reversal, +0x3E evade / handler gate, +0x4A always 0, as in Tekken 3.
- New readers: +0x68 / +0x70 (stun recovery), +0x6E (step 0x14), +0x72 (step 8), +0x74, +0x76 / +0x78 (first-attack reaction).

**Run-time changes** (0x381D30, 0x380B10):
- When the opponent's health ≤ max / 10, P1C, P1E, P24 and P26 are zeroed.
- Learning on a hit raises P20/P22, P24/P26, P2A/P2C and P2E/P30 in Tekken 3's pairs.
- Tekken 3's 600-frame "rage" (reload at the top level after the same move hits three times) was not found.

**Decision chain** 0x380F38. Bookkeeping comes first; then the first handler that returns non-zero produces the pad word, and its id goes to gp-0x6FD0:

| Id | Function | Tekken 3 equivalent (via the profile fields read) |
|---|---|---|
| 0 | 0x381418 | canned input (+0xA one-shot word, +0xC sequence of `dir \| btn<<8`) |
| 1 | 0x381D30 | bookkeeping: zones, frames until his hit (999 / 0x3E5 / 0x3E7), 0xB87 cut-off, low-health zeroing; returns 0 |
| 2 | 0x381BF8 | hit-confirm strings (P12) and throw escape (P16) |
| 3 | 0x388DE0 | stun recovery (0x4C02) |
| 5 | 0x381408 | player +0x188 set (hold) |
| 4 | 0x388FA8 | wake-up (P00) |
| 6, 7 | 0x38A408, 0x38A500 | vs-opponent and own moveset handlers (gate P3E) |
| 0x16, 0x17 | 0x38AD00, 0x38AC40 | handler follow-ups |
| 8, 9, 10 | 0x388170, 0x388DA8 (practice dummy), 0x385C80 | |
| 11-13 | 0x3864E8, 0x389B70 (in-string, P28-P2E), 0x389D58 (first-attack reaction, P20/P3E/P5E/P76) | |
| 0x14 | 0x38A780 | |
| 14-16 | 0x387600, 0x3879A0, 0x387DA8 | the three planned follow-up lists (P32/34, P36/38, P3A/3C); dead code in Tekken 3, live here [I] |
| 0x15, 0x11 | 0x38A928, 0x388278 | |
| 0x12 | 0x387120 | neutral offence (0x3865A0: throws P14, poke pools P1C/P1E) |
| 0x13 | 0x388840 | movement (0x388530, P50) |
| 0x63 | 0x388D70 | fallback: hold down (+0x70 ≥ 0) or keep the last direction bits |

The tag bit (pad 4) is removed from the output only for a solo Unknown (body 0x21 without +0x401) or in the gp-0x7434 modes. Otherwise the CPU tags through cancel entries with command bit 14.

**Shared helpers** [C]:
- 0x383B88 (in range): range class is move +8 & 0x1C0000 (default 0xC0000), tested against zone masks 0x4318D8 {CML, CML, CM, L, 0, 0} and 0x4318F0 {CML, CML, ML, L, L, 0} (equal to Tekken 3's).
- 0x383BD8: posture-versus-attack-mask tests (0x412 high, 0x217 mid, 0x31F, 0x10F low, 0x51F, 0x4C02).
- 0x383D70(predicate): marks pool candidates.
- Picker 0x3853E8: randint over the pool, then 0x381278.
- 0x381278 (command → pad):
  - 0x8001 / 0x8002 and commands > 0x8005 replay motion sequences (0x36CF30).
  - 0x8003 sets +0x12B.
  - Plain commands: direction from the highest set of bits 5-13 (0x507410 → 0x507428: numpad 1 = 0xC000, 2 = 0x4000, 3 = 0x6000, 4 = 0x8000, 6 = 0x2000, 7 = 0x9000, 8 = 0x1000, 9 = 0x3000); buttons from 0x507440 {0x80, 0x10, 0x40, 0x20}; bit 14 adds 0x4.
- 0x385250 after an attack: cooldown +0x94 = P08 + P[0x0A + 2·(rand & 3)]; movement cooldown 40.

**Per-moveset handlers**: table 0x507A80, 34 rows of `{moveset, vs-this-moveset handler, own handler}`; most entries return -1.
- Live vs-opponent handlers: Paul 0x38B698 (opponent alias 0x39C / 0x39D), Yoshimitsu 0x38B368, Ogre 0x38B820, Ganryu 0x38B718, Devil / Angel 0x38AFC0.
- Live own handlers: King 0x38BC00, Hwoarang 0x38BE78.
- Also: an opponent on moveset 14 in alias 0xBA9 or 0x1A6 sets +0x294.

Confidence:
- High: entry, generators, profile selection, tables, chain order, helpers.
- Medium: the profile field names (Tekken 3's shifted by 4) and the follow-up lists being live.
- Not read: the bodies of steps 8, 10, 0x11, 0x14 and 0x15; the handler bodies.

## 20. Default options

The options config is 0x439C60 (0x68 bytes, saved and loaded to the memory card by 0x35EBA0 / 0x35ED00). Its initial data in the ELF is the boot default. The option menu's descriptors are 0x40-byte entries at 0x439CE0: `{u32 config byte*, u32 default, u32 label*, u32 0, u32 value-name*[≤6]...}`; "Reset all options" uses their defaults, which equal the boot image. [C]

0x3B1488 loads the config into globals. Mode setup (for example 0x3B9AE0) copies the globals into the match settings 0x43B7F8:

| Option | Config | Global | Settings | Values | Default |
|---|---|---|---|---|---|
| DIFFICULTY LEVEL | +0x26 | gp-0x7498 | +0xD | EASY, NORMAL, HARD, VERY HARD, ULTRA HARD (AI diff 0-4) | 1 NORMAL |
| FIGHT COUNT | +0x27 | gp-0x7496 | +0x1E | 1-5 rounds to win | 1 (2 wins, best of 3) |
| ROUND TIME | +0x28 | gp-0x748A | +0xB | 30, 40, 60, 80, 99, INFINITY | 2 (60 s, 4260 frames) |
| CHARACTER CHANGE AT CONTINUE | +0x29 | gp-0x7486 / -0x7488 | +3 / +9 | NO, YES | YES |
| GUARD DAMAGE | +0x2A | gp-0x7490 | +0xC (chip flag) | OFF, ON | OFF |
| CHARACTER SELECT CURSOR | +0x2B | gp-0x748C | +0xA | DEFAULT, HOLD | DEFAULT |
| QUICK SELECT | +0x2C | gp-0x7482 | +8 | OFF, ON | OFF |
| CHICKEN MARKING | +0x30 | gp-0x7476 | +0xE | OFF, ON | ON |
| HELP MESSAGES | +0x36 | - | - | OFF, ON | ON |
| SPEAKER OUT | +0x2D (bit 0) | gp-0x749E | - | MONO, STEREO | STEREO |
| BGM / SE VOLUME | +0x2E / +0x2F | gp-0x747A / -0x7478 | - | 0-10 | 10 / 10 |

**Life** is not an option: 0x3B1488 always sets gp-0x7494 / gp-0x7492 = 3, which gives 140 (section 11). [C]

Confidence: high. The descriptor defaults equal the ELF's initial data.

## 21. CPU AI (fifth pass, read in code)

Notation: S = the AI state block (gp-0x6FCC, 1000 bytes), P*xx* = the profile field at profile offset *xx* (the
copy at S+0x344, so P*xx* is S+0x344+*xx*), T3 P*yy* = Tekken 3's field (`ai.md` section 5.2; P*xx* = T3 P(*xx*-4)).
"rand < P" is `(rand() & 0xFFF) < P`. Tekken 3's step names are those of its `ai.md` section 7 and of the engine's
`ai.rs` (4.A-4.T).

### 21.1 The chain, complete

0x380F38 calls 27 steps, not 26: section 19 missed 0x38ABE8 (id 0x16 again). The order, with Tekken 3's step:

| # | Id | Function | What it does | Tekken 3 |
|---|---|---|---|---|
| 1 | 0 | 0x381418 | canned sequence word | 2 (4.B) |
| 2 | 1 | 0x381D30 | bookkeeping; returns 0 (21.5) | 1, 7.0 (4.A, 4.D) |
| 3 | 2 | 0x381BF8 | hit-confirm chain / throw escape | 5 (4.E) |
| 4 | 3 | 0x388DE0 | stun recovery, three tiers (21.4) | 6 (4.F) |
| 5 | 5 | 0x381408 | hold while player +0x188 | 4 (moved after stun) |
| 6 | 4 | 0x388FA8 | wake-up | 7 (4.G) |
| 7 | 6 | 0x38A408 | vs-opponent handler (+0xB4), gate P3E, lock S+0xCC | 8 (4.H) |
| 8 | 0x16 | 0x38AD00 | the same through 0x38BF38, a stub that returns -1: dead | - |
| 9 | 7 | 0x38A500 | own handler (+0xB8) | 8 (4.H) |
| 10 | 0x17 | 0x38AC40 | delayed low guard (21.2) | new |
| 11 | 8 | 0x388170 | guard after a reaction / already safe (21.2) | 9 (4.I), changed |
| 12 | 9 | 0x388DA8 | attract mode (gp-0x7434) frame skip: hold on the other side's frames (gp-0x74D8 & 1 != side) | 3 (4.C) |
| 13 | 10 | 0x385C80 | follow-up list collection, candidates, 221-frame rule | 10, 11 (4.J, 4.K) |
| 14 | 0x16 | 0x38ABE8 | while player +0xF7: pick an entry with trans 0x1A / 0x1B | new |
| 15 | 11 | 0x3864E8 | stance continuation | 12 (4.L) |
| 16 | 12 | 0x389B70 | inside his string | 12 (4.M) |
| 17 | 13 | 0x389D58 | reaction to his first attack (chooser 0x389488) | 13 (4.N, 4.O) |
| 18 | 0x14 | 0x38A780 | tag out (21.2) | new |
| 19-21 | 14, 15, 16 | 0x387600, 0x3879A0, 0x387DA8 | follow-up executors C, B, D (21.3) | dead code in Tekken 3 |
| 22 | 0x15 | 0x38A928 | delayed input executor (21.2) | new |
| 23 | 0x11 | 0x388278 | anti-air, back turned, turning round | 4.Q |
| 24 | 0x12 | 0x387120 | neutral offence (21.5) | 14 (4.S) |
| 25 | 0x13 | 0x388840 | movement (0x388530) | 15 (4.T) |
| 26 | 0x63 | 0x388D70 | fallback | - |

The order is Tekken 3's except: the hold (5) after stun recovery, the attract frame skip at 9 (not before the hit
confirm), four new steps, and three live executors. [C]

### 21.2 The new and changed steps

**Step 8, 0x388170** (Tekken 3's 4.I without its "attack now"). [C]
- Only when a punish is being timed (S+0x9C ≥ 0), he is idle, or his active frame is under 30 frames away;
  otherwise the step returns 0.
- Then: when S+0xA4 is set (never: S+0xA4 is only ever cleared) and not already safe, a guard pad (0x386428).
- Else, when player +0x11A is set (the transition code at 0x36B430 sets it, 1 or 2 [I: after a reaction]), with
  rand < **P72**: the guard pad of 0x386470 (back 0x8000 vs his 0x31F / 0x217, d/b 0xC000 vs 0x412, else none),
  movement reset.
- Else, when already safe (S+0x98): hold the direction bits.
- Tekken 3 raised "attack now" here when his attack whiffed (P20 + string × difficulty × 30); Tag has no such rule.

**Step 0x17, 0x38AC40** (delayed low guard). The guard pool 0x384D78 (Tekken 3's 0x80057FB8) arms S+0x26C when
his move is a low 0x10F at his zone < 3 (or 0x31F with randint(0x3FFF) = 0), with S+0x270 = 4 + randint(frames
until his hit - 4) (randint(4) if fewer). The step counts S+0x270 down and at -1 outputs 0x386428's pad (d/b 0xC000;
back 0x8000 vs 0x217): a crouch guard timed into the last frames before the low. [C]

**Step 0x14, 0x38A780** (tag out). [C]
- When S+0x3E0 ("tag wanted", 21.5): mark the tag entries (0x384B80: cmd bit 14, or trans bit 0x2000); at my zone
  ≥ 2 or when a throw is ruled out (his posture & 0x605, he is airborne or unthrowable, his hit < 10 frames away),
  only the non-throw ones (0x384BB0). If any, with rand < **P6E**: pick one.
- Else for Unknown (body 0x21) only, when S+0x278 is armed (stun recovery, 25/4096 at difficulty < 2) and the timer
  S+0x274 has run out and player +0x12B != 1: with 1/4, entries whose trans has a parameter (bits 7-12) and the
  swap bit 0x2000 (0x384C70); S+0x274 = 20 + randint(20).

**Step 0x15, 0x38A928, and picker 0x3854B0** (delayed input). Neutral offence picks through 0x3854B0, not 0x3853E8.
When the chosen entry has window start 1, exec frame > 1, exec < window end and end != 0xFF (0x384BF0; a buffered
entry), with rand < **P74** it is not input now: S+0x260 = 1, S+0x264 = the entry, S+0x268 = (exec - frame + 1) +
(end - exec)/2 + randint((end - exec)/2) (randint(end - frame) when the exec frame is past), and the output is 0.
Step 0x15 then each frame: the entry must still be in its window, its req, its per-alias rule (the same table as
the chain-throw rule: aliases 0xEE3, 0x197, 0x181, 0x3BC, 0x4CA, 0x1245, 0x1246, 0x1253, 0x1255, 0x1262 by moveset
and coin) and its cond must pass, I must face him (< 0x2000) and his hit must not come before the move's; it
waits until my frame reaches S+0x268, then inputs it with the after-attack cooldown. [C]

**Step 0x16, 0x38ABE8**: while player +0xF7 is set [I: an air / recovery state], pick from the entries with trans
0x1A or 0x1B. [C for the test, I for the meaning]

**Step 11, 0x3864E8** (stance): as Tekken 3's but tag entries are excluded (0x383ED8), the trigger also accepts
player +0x42 = 2 [I: Tekken 3's quadrant test], it is off while S+0x294 (Ogre's 0xBA9 / 0x1A6), and when nothing is marked it returns 0:
the chain goes on (Tekken 3 ends the frame). Picking sets the cooldown to -1. [C]

**Step 12, 0x389B70** (inside his string): the guard is the guard pool 0x384D78 first (only when he faces me,
turn < 0x5000), the plain guard pad when the pool is empty; when the pool armed the delayed low guard, difficulty
< 3 waits for it and higher ones guard at once; the
interrupt uses P2A (T3 P26), the guard P2E (T3 P2A), the sidestep P28. [C]

**Reaction chooser 0x389488**: Tekken 3's rules, plus one before the counter rule: when his current attack move
is in the 20-entry ring S+0x2D0 (the last 20 attack moves he started, pushed by bookkeeping) at least **P76** times
(3; 2 at ULTRA HARD), guard with rand < **P78**. It replaces the rage (section 19). [C]

### 21.3 Follow-up executors (all live)

All three use the same frame filter as step 0x15 (window, req, per-alias rule, cond), give up when turned ≥ 0x2000
and exclude tag entries unless S+0x3E0. After every input: the after-attack cooldown (0x385250). [C]

| List | Collected (step 10, on a move change) | Armed by | Executor | Limits |
|---|---|---|---|---|
| C, S+0x134, ≤ 4 | 0x3856C0: entries leading to string starters (flags24 & 0x20) | neutral picking a string starter (S+0x12C = 1) | 0x387600 (ported) | P32 count, P34 chance |
| B, S+0x154, ≤ 48 | 0x3857F8: entries in range of my zone (0x383B88) into jump / hop moves (rec4 0x80000, rec8 0x200000) or throws (kept with rand < P1A; none while he is down) | neutral: I am in a jump (1/2) or the pick is a jump (3/4; a tag jump only when S+0x3E4); 0x3865A0; King's canned throws | 0x3879A0 | P36 count, P38 chance |
| D, S+0x228, ≤ 10 | 0x385A60: entries with window start 1 and end = exec (the string follow-ups of an attack); string starters only with rand ≥ P4A (always) | every move change while I am in an attack (my move has active frames), not in a stance, C and B idle; S+0x220 += 1 | 0x387DA8 | P3A count, P3C chance |

Executor B (0x3879A0): pick one at random; a pick into a move with rec8 0x200000 counts (S+0x14C += 1). An attack
is input only if his hit does not come first (frames until his hit ≥ its start-up), the count ≤ P36, my attack
count S+0x258 < its limit S+0x25C and 0x387518 allows it; otherwise hold and disarm. A jump pick keeps the list
armed; another keeps it with rand < P38.

Executor D (0x387DA8), each frame of the window: with rand < **P3C** pick one at random, else **hold the direction
and end the frame** (the later steps do not run). An attack is input only if S+0x220 < **P3A**, S+0x258 < S+0x25C,
his hit is not first and 0x387518 allows it; otherwise both D fields are cleared and the step returns 0.

0x387518 (veto), skipped with 1/32: veto at my zone < 2 while he is inside a string (S+0x2C0 ≥ 0), at my zone ≥ 4,
when he is lying (posture & 4), not airborne, and the move neither hits the ground (rec8 0x10000) nor is a low
(0x10F / 0x51F), and when I am turned more than 0x3800.

P3A is 1 at every level 0, so level 0 never continues a string; from NORMAL level 4 it is 5-10 with P3C 2048-3276
(50-80 % a frame), so a follow-up is nearly always input inside its window. [C]

### 21.4 Stun recovery, three tiers (0x388DE0)

When my posture is 0x4C02 and my move has no active frames: movement reset; candidates with END links (0x3835A0);
then, first match:
1. rand < **P68**: entries with trans 0x1E (0x384B68); cooldown -1 and "attack now" (S+0xE0 / +0xE4 = 1).
2. rand < **P70**: tag entries (0x384B80): tag out of the stun.
3. rand < **P66** (T3 P62): entries with trans 0x1C / 0x1D (0x384B50), Tekken 3's only tier.
4. otherwise output 0. With no candidates also 0. The step always ends the frame.

Values: P68 0-40, P70 2048 (819 at VERY HARD level 7), P66 0-2048. [C]

### 21.5 Attack rate: what differs from Tekken 3

The neutral gate (0x387120) and the attack decision (0x3865A0) are Tekken 3's step for step with the 4-byte shift
(the 1/12 and 1/8 approach draws, the run stop, the punish timer, the anti-air P4C, P1C / P1E / P24 / P40 reads,
the throw rules, the pools, the desperation pick with P24). Movement 0x388530 has Tekken 3's thresholds. The
differences:

1. **Cooldowns (data).** After an attack S+0x94 = P08 + P(0x0A + 2·(rand() & 3)), and it only runs down while not
   attacking. Tag's top levels keep a long fourth choice:

| Profile | P0A | P0C | P0E | P10 | Mean |
|---|---|---|---|---|---|
| Tag EASY L0 / L7 | 300 / 80 | 300 / 120 | 400 / 200 | 400 / 200 | 350 / 150 |
| Tag NORMAL L0 / L4 / L7 | 150 / 50 / 30 | 150 / 60 / 60 | 200 / 100 / 60 | 200 / 150 / 120 | 175 / 90 / 68 |
| Tag VERY HARD L7 | 10 | 30 | 30 | 60 | 33 |
| Tag ULTRA HARD L0 / L3-7 | 30 / 10 | 60 / 30 | 90 / 60 | 150 / 150 | 83 / 63 |
| Tekken 3 HARD L9 | 10 | 30 | 30 | 30 | 25 |

   ULTRA HARD level 7 waits 2.5 times Tekken 3's top on average; VERY HARD level 7 is the most aggressive profile.
2. **Distance jitter.** Bookkeeping adds `sin(phase) · 192` to the distance before both zone tests (my zone and his
   zone): phase S+0x2C += rand() & 0x3F a frame, sine table 0x4FCE88 (4096 s16, ±4096), the term kept at S+0x28.
   The perceived distance swings ±192 over about 130 frames, so zone 2 / 3 edges flicker. Tekken 3 has none.
3. **Tag entries.** Neutral excludes the tag entries (0x383E20 with 0x384B80) unless S+0x3E4 (my life below my
   partner's); every list and the 221-frame rule exclude them unless S+0x3E0.
4. **Delayed input** with P74 (21.2): 0 at level 0 of EASY-HARD, 1228 (30 %) at most other levels, 2048 at ULTRA
   HARD level 0, 204 at the top.
5. **String follow-ups live** (21.3): Tekken 3 never continues a string by list D; Tag does, so one decision can land
   2-5 hits.
6. **No "attack now" on his whiff** (step 8) and no rage; instead P76 / P78 guard a repeated move.
7. The cooldown runs down as in Tekken 3: one a frame while not attacking or while he is airborne (opponent
   +0x135), one more in zone 1, two more when either side is pushed.

**Tag wanted (S+0x3E0) and behind (S+0x3E4)**, bookkeeping 0x3827AC. Life words come from 0x440940 + 0x60·team
(player +0x1E) + 0x30·slot, +0xC current and +0x14 maximum [I], for my slot (player +0) and my partner's
(player +2); in mode 7 the values are reduced by S+0x27C [I]. S+0x3E4 = my life < my partner's. S+0x3E0 is 0 while
S+0x50 (running) or for a solo Unknown, else 1 when any of:
- my life < 1/2 of my maximum (10/16 in attract mode), < my partner's, and his ≥ 11/16 of his maximum;
- my life < 1/3 of my maximum, my partner's ≥ 1/2 of his, and mine < his;
- my life < 1/8 of my maximum and my partner's > 1/6 of his.
Both are 0 in mode 5 and in attract mode unless gp-0x6FAC; S+0x3E0 is 0 while stunned against his alias
0x338 / 0x33A. [C for the arithmetic, I for the life fields]

### 21.6 Per-moveset handlers (table 0x507A80)

Handler calls: step 6 / 7 when S+0xCC < 0 and rand < P3E. Returns: -1 nothing, -2 handled, -3 pick the pool,
0x2000 / 0x8000 walk 16 frames (0x380540(16)), other = pad word. Every other row is `jr ra; li v0, -1`. [C]

| Moveset | Handler | Behaviour |
|---|---|---|
| 0 Paul (vs) | 0x38B698 | his alias 0x39D at my zone < 3: walk back (0x8000) from his frame 16, before that cooldown -1. Alias 0x39C returns at once (a branch bug: both were meant). |
| 3 King (own) | 0x38BC00 | cooldown ≤ 0 and rand < P1A (T3 P16). He stands, 1/8, my zone < 3, not airborne / unthrowable, I am idle, his hit ≥ 9 frames away, both +0x42 = 0, walk alias 0x3F reachable: one of `n,f,d/f,d/f+1+3,d/f` / `…d/f+2+4…` / `…d/f+1+2…` (0x507A52, 14 bytes each), and list B armed. He crouches, my zone < 2 (same tests, 0x3838B0): `d,d+2+4,d,d,n` (0x507A32, both rows equal). He is down, my zone < 2, his +0x42 0 or 2: `d/b,d/b+1+3,…` / `d/b,d/b+2+4,…` (0x507A12). |
| 4 Yoshimitsu (vs) | 0x38B368 | Tekken 3's 0x80061B88 step for step, Tekken Tag's aliases; by his alias (OPP +0x10C): 0x1A8 at his zone < 3 cooldown -1. 0x5C7 at his zone ≥ 3 forward 0x2000, else lock S+0xD0 = 4, the low / ground filter (0x38A748: S+0xBC = 0x3840F8), cooldown -1. 0x5CA-0x5CC: at my zone ≤ 0 outside his active frames the same with -2, inside 0; facing me (turn - 0x2000 < 0x4001) lock 20 and at my zone < 2 the filter and cooldown -1, else forward (0x38AEB0); turned, at my zone < 3 with 1/8 and not crouching / 0x400000 a sidestep (0x38A690(-1)), else lock 10 and forward. 0x1A1 / 0x5A9: after his active frames cooldown -1; his turn > 0x1000: forward at his zone > 1; at his zone ≥ 3 within 0xA8D a backdash (0x38A760); his zone < 2 and his hit ≥ 11 frames away -1, his zone ≥ 3 or his hit < 11 frames away 0, else a sidestep (rand & 1). 0x5C9, 0x623-0x625: his turn ≥ 0x4000: at my zone < 2 cooldown -1; else at my zone < 2 cooldown 1, < 3 the filter and cooldown -1, else forward. Others (0x5EE ...): -1. |
| 6 Hwoarang (own) | 0x38BE78 | in alias 0xA8B (stance): unless frame 13-17 with his hit < 10 frames away, forward 0x2000 with 15/16. |
| 14 Ogre / True Ogre (vs) | 0x38B820 | his alias 0xBA9 or 0x1A6, by his frame: lock S+0xD0 = 8; 1/32 at frames 9-13 and my zone < 2 a sidestep; early: walk in / cooldown -4 / sidestep by stage side (0x38AF28); during the move: back off or cooldown 8; after: at my zone < 2 a punish pad from 0x507A08 {d/f+2, d/f+1, d/f+4, d/f+4}, at zone 2 d/f (0x6000), else 1/8 walk in. |
| 25 Ganryu (vs) | 0x38B718 | his alias 0x1407, I am not attacking, not airborne, he faces me: at his frames 24-25 and distance < 3000 the sequence 0x507618 (n, d/f); else at distance < 4000 crouch guard d/b (0xC000). |
| 27 Devil / Angel (vs) | 0x38AFC0 | Ogre's handler for his aliases 0x1459 / 0x1458, punish pads 0x507A00 (the same four). |

Per move set, table 0x507620 (16 bytes, Tekken 3's 0x80022D14): +0 can parry, +4 / +8 the high-mid reversal and
low parry inputs, +0xC the combo preludes {s16 prob, u16 alias, input pointer} to a negative prob. The neutral
offence (0x386CD4) takes a prelude as Tekken 3 does, but draws rand() & 0xFFF anew for each one (Tekken 3 draws
once). [C]

Section 19's "opponent on moveset 14 in alias 0xBA9 or 0x1A6 sets S+0x294" stops movement (0x388530 returns 0)
and the stance step during Ogre's move. [C]

### 21.7 Levels

- Arcade: 0x359D3C increments settings +0x24 after each won fight (and gp-0x7424 = min(fights, 7)); 8 fights. So
  the profile is (DIFFICULTY option, level = fight index 0-7): the first fight level 0, the eighth level 7. gp-0x7423 is only
  ever cleared (0x3BA2BC), so the "≥ 8 → diff 1 level 6" fallback of 0x380120 is dead. [C; that s0 there is the
  settings block is I]
- Mode 3: difficulty 1 (NORMAL), level = settings +0x24. Survival-type mode 4: the streak ladder of section 19
  (exact; ai.json `survival`). Attract: profile 0x432D90.
- Measured (PCSX2, PINE reads of the state block's profile copy S+0x344 and S+0x3DC): arcade stages 1, 4 and 8 at the
  default DIFFICULTY load NORMAL levels 0, 3 and 7. Against a 1P who stands still the CPU does about 195 damage a
  fight minute at stage 1 (Lei, Julia: a K.O. in 31-36 s), 240 at stage 4 (Michelle), 120 at stage 8 (Unknown);
  against one holding back 38, 140 and 120. A still fighter blocks its highs and mids; its lows and throws knock him
  down, and a 1P who never presses a button lies until the next hit, so most of the damage is its ground attacks
  (Lei's 420; the stomps 409 / 419 at active frame 16 strike 700 high and whiff a lying body here too).

### 21.8 Profile fields, final map

| Field | Reader | Values (EASY L0 … ULTRA HARD L7) |
|---|---|---|
| +4 / +6 | none in the AI (no S+0x348 / 0x34A reads) | 7/7 … 3/5 |
| P08, P0A-P10 | cooldown | 21.5 |
| P1A | King's handler; list B throw keep | 0 … 2048 |
| P32 / P34, P36 / P38, P3A / P3C | executors C, B, D | 21.3 |
| P3E | handler gate; evade (chooser) | 819 … 3891 |
| P4A | always 0 (every `rand ≥ P4A` passes) | 0 |
| P4C | anti-air | 0 everywhere |
| P50 | keep a walk-back / backdash | 12 … 245 |
| P66 / P68 / P70 | stun tiers 3 / 1 / 2 | 21.4 |
| P6E | tag out | 245; 409 at VERY HARD L7 and ULTRA HARD |
| P72 | guard after a reaction (step 8) | 0 … 2457 |
| P74 | delayed input | 0 … 2048; 204 at the top (21.5) |
| P76 / P78 | repeated-move guard | 3 / 0 … 2 / 2048 |
| P7C / P7E | not read by the AI | 32392 … 31768 / 80 |

### 21.9 What the engine's port (`kumite_sim/src/ai.rs`) should change

1. `run()` 4.P: port executors B (0x3879A0) and D (0x387DA8) for `ttt` beside `string_followup`; D holds and ends the
   frame when its draw fails. Collection (4.J) must drop tag entries unless "tag wanted".
2. `run()` 4.F: the two tiers before P66 (P68 trans 0x1E with attack-now, P70 tag entries); return in every case.
3. `run()` 4.I: for `ttt` drop the whiff "attack now"; add step 8's P72 guard on player +0x11A.
4. Bookkeeping (4.D): the ±192 sine jitter on both zones; S+0x3E0 / S+0x3E4; the 20-move ring.
5. `reaction()`: the P76 / P78 rule before the counter rule.
6. `neutral()` / `pick()`: exclude tag entries unless behind; pick through 0x3854B0's delayed input (P74) and add the
   0x15 executor.
7. New steps: tag out (0x14, P6E) before the executors; delayed low guard (0x17) inside `pool_guard`; 4.L falls
   through when nothing is marked; in-string guard uses the guard pool first.
8. Handlers: the routines above (`call_handler`), with King's three sequences and the pad tables (`dump_ai5.py`).

Confidence: high for 21.1-21.4 and the handler table; medium for the life fields of 21.5 and player +0xF7 / +0x11A.

## 22. The look: neck, head and eyes (read in code, checked in PCSX2)

0x351E98(player, target, view) runs in the drawing (0x3478F0, after the node world matrices, player +0xDB0 + 0xC0 x
node): it changes only what is drawn, never the bone slots (+0x630) the strikes and spheres use. Nodes: neck 5 (on
2), head 6 (on 5), eyes 21 and 22 (on 6); the target is the opponent's (0x351E78) node 5 world position, +0x11A0.

- On (fp): player +0xB132 (set by the fighter's setup at 0x3731D0, cleared by the scene setups 0x3AEB34 /
  0x3B53DC), the round machine gp-0x7268 at 1 or 2 (0x36E160: the fight and the K.O. time), the move's +0x24 bit 22
  (the engine's dialect maps it to Tekken 3's bit 30) and, for an attack (+0x2C != 0), frame >= max(+0x2D, +0x18 -
  17). Off: the easing is 1 (0x352238) and the angles are aimed at once at the point (1000, 0, 80) of node 6's
  frame (so turning on starts where the face looks), but the copy-back of the neck, head and eyes (worlds and
  locals +0x8910 / +0x8950 / +0x8D10 / +0x8D50, 0x352DA4..0x352E94) runs only while on (`beqz fp` at 0x352D9C):
  off, the pose solver's neck and head are drawn (below). gp-0x7210 / gp-0x720C / 0x3A9698 freeze the angles (the
  saved ones are written).
- Before the look: the drawing's node pass (jump table 0x522DF0, node → slot 0x437E70) gives node 5 slot 2's turn
  (0x34A3B8, rotation only) and node 6 its bind (0, 0, 0); then the pose solver 0x347E28 (called at 0x347B70, before
  the world matrices) splits it (0x3480A0..0x3484B0): the neck's local over its base B (0x349D70 of 90°, 0, 90°),
  M = B^-1 · L5, is taken as Rz(c) · Ry(b) · Rx(a) (a = atan2(m21, m22), b = atan2(-m20, |m00, m10|), c =
  atan2(m10, m00); of it and the other solution (a ± 180°, 180° - b, c ± 180°) the one with the smaller 8|a| + |b| +
  |c|, each wrapped to ±180° by 0x34A570; |sin b| within 1e-5 of 1 takes the gimbal branch), the neck becomes
  B · Rz(0.4 c) · Ry(0.6 b) · Rx(0.2 a) (gp-0x7990 / 0x7994 / 0x7998) and the head node5'^-1 · L5, so the head
  looks where the motion's neck did. PCSX2 (Paul's 1+2, True Ogre's fire; and with 0x351E98 patched to return): the
  locals +0x8910 / +0x8950 equal this to 1/1000 on every off frame, and the saved angles on every on frame.
- Neck: aimed from the solver's neck (the +0x8910 copy at 0x3521CC) with its origin raised by eye A's height (z in
  the neck's frame): yaw = atan2(d.y, d.x), pitch = atan2(-d.z, |d.xy|); new local rotation Rx(yaw) · Rz(pitch) ·
  base (0x349EE0 of 90°, 90°, 0: x→y, y→z, z→x, the model's rest; sceVu0 RotMatrixZ 0x34AB68, Y 0x34ACB8, X
  0x34AC10, left-multiplied), the motion's offset kept. The head the same from the turned neck: local Rz(yaw) · Ry(pitch) (0x349D70 of 0, pitch,
  yaw). Each: yaw within ±40°, pitch within -32.5°..15° (gp-0x7A88..0x7A3C); past ±160° the yaw stays at ±40° on the
  side it went round (state 3 / 4) and the pitch is 0; eased by 1/8 a frame (gp-0x6108: 2 while off, so 1 while on;
  0.125 / it); while on the pitch moves only when the eased yaw moved 5° or less and the aim is 5° or more off.
  State per side: yaw / pitch gp-0x6100 / 0x60F8 (neck), 0x60E8 / 0x60E0 (head), side 0x60F0 / 0x60D8, reset by
  0x351E10 with the fighter.
- Eyes (while on, no easing): from where the turned head puts them, up = -atan2(d.y, d.x) within -7°..9°, sideways =
  atan2(-d.z, |d.xy|) within ±35°; local Rz(sideways) · Ry(up) · Rx(90°).
- PCSX2: the drawn head's x axis points at the opponent's neck (dot 0.9999, Xiaoyu and Anna); Paul's side-on stance
  holds the neck at its 40°.

## Not decoded

- Camera (section 18): the camera-motion channel format in file 12 in full; director states 8-15 in detail; the
  replay's per-frame follow (0x3D0A38) and pass bounds beyond the end checks; the demo orbit cameras (modes 5-12).
- Hit sparks and their table (section 17).
- Ids 0xA8-0xB5 (group 21) and kind-1 group 22's use; the type-8 script semantics in
  full; what 0x376578 (type 9) does.
- Which pad bits pick the intro and win pose (as buttons).
- How costume bodies 34-38 index the per-body tables (the dump script assumes they use their host body's).
- Wall / ring-out rules; the ground game was not compared with Tekken 3 in this pass.
- CPU AI (section 21): player +0xF7 and +0x11A's exact meaning; the life table
  0x440940's fields; gp-0x6FAC; the wake-up and first-attack bodies were not re-read against Tekken 3 line by line.
- Player +0x12E (counter-hit cause), +0x3E8 / +0x3F0 (conditions 0x36 / 0x37), transition bit 0x4000's effect.

Risk: the hit segments have no radius and use raw bone origins, so the engine must pose Tag's skeleton exactly to
reproduce hits.

## Scripts (`docs/re/`)

Research aids, kept as they were used; they read the executable `disc/SLUS_200.01` beside them (`extract_elf.py`
takes it from your disc image) and the pack's `raw/` folder (`TTT_RAW`). Plain `python3`; `mdis.py` and
`fulldis.py` need `capstone`. `ttt.py` (loaders, table addresses), `dump_fighter.py` (`python3 dump_fighter.py
PAUL`), `dump_tables.py` (reactions, curves, shared cancels, health tables), `t3match.py` (pairing with Tekken 3's
converted game data), `xref.py`, `fn.py`, `dump_rules.py` (the tables of sections 7-12 as JSON, and the usage
counts of conditions and transition types), `condmap.py` (conditions and transitions of paired Tekken 3 / Tag
cancel lists), `dump_round_sound.py` (the tables of sections 13-14 as JSON), `dump_camera.py` (section 18's tables as JSON),
`dump_ai.py` (section 19's AI tables), `dump_ai5.py` (section 21's named profile fields, handler sequences, pad
tables), `aifn.py` (`python3 aifn.py 387DA8`: an AI function to the next function start, calls named), `gpx.sh` (gp-relative references in `out/code.asm`, the `fulldis.py` output).

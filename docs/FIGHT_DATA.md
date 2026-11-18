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
- The test (0x376CF8) is Tekken 3's: box reject, clip to the sphere's height band, horizontal distance against
  radius.

Timed properties: ops 1-10 kept their numbers on paired moves; damage to the opponent is 0x0C (was 0x0B), damage
to self 0x0D; hand poses moved up (0x0E -> 0x0F, 0x15 -> 0x18). 221 of 233 paired entries have equal frame and
argument.

Sound lists keep the type nibbles (6, 7, 2, 8, 9; a few entries have 0, 1 and 3); the indices are renumbered.

Over the whole data: 4174 hit records (3676 chains; the gates used are 0x4E, 0x51, 0x53, 0x56, 0x57, 0x5C: the
opponent's moveset is 11, 14, 16, 19, 20 or 25), 2283 timed property lists (ops up to 0x58, and 10 entries with op 0), 4064 sound
lists, 100 input sequences, 54 reversal motion lists.

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
| 0x50 / 0x51 / 0x52 / 0x53 | self +0x54 == 1 / 2 / 0 / 3: which way I face on screen (quadrant against the camera's axis) | 0 |
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
  frame 135: done. In one mode (gp-0x7434 set, 0x3ABCC0 false) there is no replay: done at frame 120.
- Time up: frame 50 as above, done at frame 160.
- Otherwise done at frame 600.
- No slow motion was found in the K.O. path itself; the replay's speed is in 0x364B50 (not read).

**Win poses** (state 3, 0x36E77C): both players get +0x127 = 1; 0x3B5DB0(p0, p1, 0) replaces the loser's player with
the winner's partner (team entry +2 of the winner's side; body, aliases and volumes reloaded), placed at
x 0x400, z -0xC00 from the winner; in a solo match the loser is hidden (+0x127 = 0). 0x368528(winner, other,
frame-counter parity) picks the pose from the winner's pad: 0x80 -> 0, 0x10 -> 1, 0x40 -> 2, 0x20 -> 3, nothing ->
gp-0x74D8 & 1. Both play `alias 0x12A2 + pose` (4770-4773) of their own moveset (0x3685B8), so a tag team poses as
a pair. Body 33 never uses pose 0: it takes 0x12A3 + gp-0x74D8 % 3. Camera mode 0xE (side 0 won) or 0xF.
State 4 then lasts 300 frames, or until START (pad bit 0x800, from frame 2).

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

## 15. Tag (third pass)

- No tag invulnerability exists in code. The only hit immunity is +0x14E: set to 4 frames on a hit that dealt
  damage or hit an airborne victim (0x37202C), counted down each frame (0x3707F4); while it runs the hurt-sphere loop
  (0x377198) skips the victim. Posture bit 24 (the tag state) is tested only at 0x36AA70 (section 7). So the leaving
  fighter can be hit until the swap, and the entering one from its first frame.
- The benched partner is not simulated: it exists only as its team entry (health regeneration, anger; section 10).
- Tag throws and tag combos: nothing beyond the partner gates and the juggle counter reset (section 10) was found.
- Camera during a tag: not decoded (section 16).

## 16. Fight camera (entry points only)

0x38D518(p0, p1, &mode) runs the camera every frame on mode gp-0x7200 (jump table 0x507F70, 20 modes; state at
0x433330). Mode 0 is the round-start fly-in (0x395400), then mode 1, the fight camera (0x390E08 resets it with an
offset of y -1200, z -8000 and the field of view from state +0xA4; framing 0x38EC08 per fighter, 0x38F768, 0x396CF8,
0x391048); mode 1 switches to 0xD when 0x38CFC0 says so. 3 = replay, 0xD-0x12 the end shots of section 13.
0x35B7A8 / 0x35B858 copy a 36-byte eye / target block into 0x415808 and build the view (0x35B4B0); 0x35B8B8 turns a
segment into yaw and pitch. The per-move camera script byte (flags top byte) is not traced yet.

## 17. Hit effects

Not located. The damage function's only effect calls are on chip damage: 0x35BFF8(2, attacker) when the victim
survives, 0x35C0D8 when chip K.O.s (both lead to 0x35BF10 / 0x3A2FF8, likely vibration and an effect).

## Not decoded

- The fight camera's framing constants and per-move camera scripts; the camera during a tag (section 16).
- Hit sparks and their table (section 17).
- Ids 0xA8-0xB5 (group 21) and kind-1 group 22's use; the type-8 script semantics in
  full; what 0x376578 (type 9) does.
- The K.O. replay's speed and window (0x364B50); the default round-time and rounds options.
- Which pad bits pick the intro and win pose (as buttons).
- How costume bodies 34-38 index the per-body tables (the dump script assumes they use their host body's).
- Wall / ring-out rules; the ground game was not compared with Tekken 3 in this pass.
- The CPU AI's structure.
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
cancel lists), `dump_round_sound.py` (the tables of sections 13-14 as JSON), `gpx.sh` (gp-relative references in `out/code.asm`, the `fulldis.py` output).

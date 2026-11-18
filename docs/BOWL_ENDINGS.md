# Tekken Tag Tournament (PS2, USA v2.00, SLUS-20001): Tekken Bowl, the endings, Theater and Gallery

Seventh pass, worked out on 2026-11-18 from `SLUS_200.01` (static analysis) and the disc's raw files. Memory address
= file offset + 0x33F000, gp = 0x5FD0F0. Confidence: [H] read in code, [M] read in code but the meaning is inferred,
[L] a guess. "FE n" is a section of [FRONTEND.md](FRONTEND.md), "FD n" one of [FIGHT_DATA.md](FIGHT_DATA.md).
Characters are character ids (FE: 0 Paul ... 33 Unknown). Units are the game's: 60 frames a second, y is down,
angles in 1/65536 turns (0x800 = 11.25 degrees). The VU0 macro code (`lqc2` / `vadd` ...) that capstone misreads
was read with `docs/re/vdis.py`.

## 1. Tekken Bowl: structure

**Memory** [H]

| Address | What |
|---|---|
| 0x1FC51B0 | camera `{f32 x, y, z, .., +0x14 yaw, +0x20, +0x24 mode}` |
| 0x1FC51E0 | ball, 0x80 bytes: +0x00 position, +0x10 velocity, +0x20 spin vector, +0x30 / +0x40 previous position / velocity, +0x50 roll angles, +0x60 roll speeds (vec4 each), +0x70 rolling, +0x74 shown, +0x78 has landed |
| 0x1FC5260 | 10 pins x 0x180: +0x00 position, +0x10 velocity, +0x40 angles, +0x50 angular velocity, +0xC0-0xF0 four contact points, +0x100-0x130 their velocities, +0x150 base anchor, +0x170 standing, +0x174 on the deck (not yet swept), +0x178 moving (0 = at rest), +0x17C hit this frame, +0x17E clatter timer |
| 0x1FC61E0 | 2 players x 0x5E: the score card (section 5) |
| 0x1FC62A0 | throw state: +0 phase, +2 prompt, +4 spin meter, +8 spin meter speed, +0xC power, +0x10 power speed, +0x14 stat S |
| 0x1FC6350 | bowler: +0 / +4 / +8 three hand-joint matrices (ball attach), +0xC-0x11 motion slots, +0x12 left-handed |
| gp-0x6048 | main state; gp-0x6044 current player; gp-0x6046 frames since release; gp-0x603C pins standing |
| gp-0x6768, gp-0x601C | bowler motion slot (file 128) and its frame |

**Entry and loop** [H]. Mode setup 0x3F0C00: arena 20, scene 42 (gp-0x63BE = 0x2A) whose motion pack is **file
128** (0x363FB8 -> 0x35CFD0: scene 42 -> file 128; its camera file would be 127), racks the pins (0x3EA140), player 0.
Per frame the fight loop calls **0x3F0008** (from 0x35ACA0 / 0x35B454), a state machine on gp-0x6048 (table 0x523A50):

| State | Code | Does |
|---|---|---|
| 0 | 0x3F00FC | new turn: bowler set-up 0x3E9C58, the other fighter parked at z -15000, ball reset 0x3E9FD8 |
| 1 | 0x3F01B0 | the throw (0x3EAF78, section 2), ball in hand (0x3EBB80) or rolling (0x3EC768); goes to 3 when the ball passes z 4000, to 4 when it hits a spectator |
| 2 | 0x3F04C0 | replay start (0x3EF290), then 3 with REPLAY shown [M] |
| 3 | 0x3F04DC | the roll: physics step 0x3EED88, decision 0x3EFB00 -> 2 (replay), 3 (go on), 5 (strike), 8 |
| 4 | 0x3F05E4 | spectator hit: 180 frames, "K.O." (0x523A30) and voice 0x1059 at frame 90, then 8 |
| 5-7 | 0x3F0670-0x3F083C | strike: both fighters stand at z -6500 / -9500, take their pose (0x5237D8 by moveset) and play their reaction (0x523590), START skips |
| 8 | 0x3F0898 | score the ball (0x3EA2B0), next ball / player; 10 when both cards are done |
| 9 | 0x3F0950 | the pause / quit menu (START or TRIANGLE in the throw) |
| 10 | 0x3F0AE0 | winner (higher total; draw), records, results 0x3F1570 |

There is **no CPU bowler**: a player whose pad slot gp-0x7440[p] is empty is skipped, so one or two humans take
turns [M].

## 2. The throw (0x3EAF78, phases in 0x1FC62A0 +0, table 0x523930) [H]

Pad: held bits for the d-pad, pressed bits for buttons; 0x2000 right, 0x8000 left, 0x40 CROSS, 0x20 CIRCLE, 0x10
TRIANGLE, 0x100 SELECT, 0x800 START.

| Phase | Name | Rule |
|---|---|---|
| 0 | start | slot +0xC (stand), prompt 1 |
| 1 | **position** | left / right: bowler x += -/+ 8 a frame, clamped to +-1000 (start x random in [-64, 64), z -11000); step-left / step-right / stand slots; SELECT = bowl-style menu (0x3EAE08); START quit menu; CROSS -> 2 |
| 2 | **aim + spin** | left / right turn the bowler +-0x10 a frame, clamped +-0x800 (+-11.25 degrees); the spin meter swings: m += v each frame, v reverses outside [-2, 1] (right-handers), [-1, 2] (left-handers), [-1.2, 1.2] (movesets 0x10 Gun Jack, 0x13 Jack-2); CIRCLE back to 1; CROSS locks it -> 3 |
| 3 | **power** | p += power speed a frame while p < 100, else p = 0 (it wraps). CROSS: if p > 0 -> 4; if p == 0 (the frame it wrapped): "OVER CHARGED" with probability S / 2000, else 4 |
| 4 | run-up | throw motion (slot +0xF, 150 frames) from frame 1; left / right still steer the bowler by 1280 / S a frame (+-1000); the ball leaves the hand at **motion frame 106** |
| 5 | message | "OVER CHARGED" (or "TIME UP") for 60 frames -> 6 |
| 6 | charge | the bowler runs down the lane with slot +0x10 (0x6A-0x6D, 175 frames): distance 256 f for f < 25, 256 f - (f - 25)^2 to 50, a further ease to 75, then 14825; voice 3 at frame 25; the pins react to the body (0x3EEBF8); then slot +0x11 (0x6E-0x71) |
| 7 | done | back to stand |

**Turn set-up** 0x3E9C58 (bowler stat S from 0x523618; Mokujin and Unknown use the copied character): S' = S, or
400 + 0.3 (S - 400) above 400. Spin meter starts uniform in [-1, 1), speed `S' x 0.06 / 200 + 0.03` (Jacks 0.048)
with a random sign; power 0, speed `2 (S' + 180) / 340`. [H]

**Release** (phase 4 at frame 106) [H]:

```
spin  = m + v_spin  * U(-1, 1)        # the spin meter's own speed is the error
power = p + v_power * U(-1, 1)
speed = S * (power + 100) / 200       # units a frame, S 160-500
a     = bowler angle in radians
ball.vel  = (speed sin a, 0, speed cos a)      # +z is down the lane
ball.spin = (spin cos a, 0, -spin sin a)        # sideways
ball.roll = (-speed / 800, a, spin / 2)          # visual rotation speeds
```

Then one of two throw voices (random) from 0x5236A0, camera mode 2, phase 7. While held, the ball sits at the
mean of the three hand joints plus (150, 0, 0) (x mirrored for left-handers) (0x3EBB80).

**Speed shown** (0x3EAC88, "SPEED %3d.%02d MPH"): v = vz x 0.216; v > 20 ? v - 6 : v x 0.7; mph = v x 0.625. [H]

## 3. Ball and pins

**Ball** (0x3EC768, every frame while rolling) [H]:

1. pos += vel; roll angles += roll speeds (wrapped to +-pi).
2. In the air (y < -150): y += 2.45, vy += 4.9. First touch: voice 0x10AA if horizontal speed < 270, else 0x10AB.
   On the lane: y = -150; vy = 0 if vy < 4.9, else vy x -0.6 (bounce).
3. **Hook**: vel += spin x ((z + 11000) / 15000)^2, then (if vz >= 20) vel is rescaled to its old length: spin
   bends the path more and more down the lane, the speed never changes. There is **no friction** and **no gutter**.
4. Past z 9435 the pin-deck walls (0x3EE2F8): x = +-2550 with 106-unit chamfers, back wall near z 12691,
   restitution 0.4.

**Spectators** [H]: the lane has none of its own edges; 25 background people of arena 20 (FORMATS 12: x +-3900 to
+-8950, z 250-7500) stand beside it. Each frame 0x3EFDB8 tests the ball against them (horizontal distance < 300):
a hit knocks the person over (0x3DB860), ends the roll at once (state 4, "K.O." on that side), and the pins count as
they are. A wide ball otherwise rolls on past the deck: a miss.

**Pins** [H on the structure, M on the meaning]: a **real rigid-body simulation**, not rules. Rack (0x3EA050, pin
centre y -250.66, an equilateral triangle of side 455):

| Pin | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| x | 0 | -227.5 | 227.5 | -455 | 0 | 455 | -682.5 | -227.5 | 227.5 | 682.5 |
| z | 7211.9 | 7606 | 7606 | 8000 | 8000 | 8000 | 8394 | 8394 | 8394 | 8394 |

Step 0x3EED88 per frame: ball (or the charging bowler, 0x3EEBF8) against each pin on the deck (0x3EDC90), each pin
integrated (0x3EC9A0), every pin pair (0x3EDF88), each pin against the lane (0x3ED338) and the deck walls
(0x3EE598). A pin has four contact points (heights 0x523570: 80, 60, 100, 75); the ground step solves each point's
time of impact under gravity 9.8 (frame fractions), damps spin by 0.0008 / 0.005 terms and bounces with 0.4-0.8
(gp-0x7600-0x75D8). A pin **counts as down** (+0x170 = 0) when it touches the lane with anything but its base point,
or with its base more than ~71 units (sqrt 5000) from where it stood [M]. There is no random element in the physics:
the randomness is all in the release (section 2) [H].

**Pin sounds** (0x3EC200): each hit sets a 5-frame timer; the sum over pins picks 0x10AE (< 15, then 8 frames
quiet), 0x10AD (< 30, 14), 0x10AC (heavier, 30). [H]

**End of the roll** (0x3EFB00): the roll ends once every pin is at rest and the ball is past z 12000 (8000 for a
charge), after at least 150 frames, at most 1000 [M]. Then 0x3EA4C8 counts and re-racks (section 5).

## 4. Per character (tables 0x523618, 0x523590, 0x5236A0) [H; meanings M]

`0x523618`: `{s16 S, u8 left-handed, u8 female style}`. S scales ball speed and both meters; hand picks the joint set
and the motion slots (0-3 right, 4-7 left); the style flag swaps the charge motions (0x6C / 0x70 instead of
0x6A / 0x6E; +1 for left-handers). `0x523590`: four motion slots of file 128 (strike reactions; 0x46 is shared).
`0x5236A0`: two throw voices and the charge voice (the fighter's own voice bank). `docs/re/dump_bowl.py` prints this.

| Id | Character | S | Hand | Style | Spin speed | Power speed | Motions | Voices |
|---|---|---|---|---|---|---|---|---|
| 0 | Paul | 360 | R | | 0.138 | 3.18 | 0x8 0x27 0x46 0x47 | 0x2001 0x2000 0x4002 |
| 1 | Law | 260 | R | | 0.108 | 2.59 | 0x9 0x28 0x46 0x48 | 0x5001 0x2004 0x5000 |
| 2 | Lei | 280 | R | | 0.114 | 2.71 | 0xA 0x29 0x46 0x49 | 0x2007 0x2001 0x4000 |
| 3 | King | 320 | R | | 0.126 | 2.94 | 0xB 0x2A 0x46 0x4A | 0x2005 0x2002 0x5001 |
| 4 | Yoshimitsu | 280 | R | | 0.114 | 2.71 | 0xC 0x2B 0x46 0x4B | 0x2001 0x2002 0x2004 |
| 5 | Nina | 220 | R | F | 0.096 | 2.35 | 0xD 0x2C 0x46 0x4C | 0x2005 0x2003 0x5000 |
| 6 | Hwoarang | 280 | R | | 0.114 | 2.71 | 0xF 0x2E 0x46 0x4E | 0x2001 0x2005 0x4001 |
| 7 | Xiaoyu | 180 | R | F | 0.084 | 2.12 | 0x10 0x2F 0x46 0x4F | 0x2002 0x2006 0x4002 |
| 8 | Eddy | 280 | L | | 0.114 | 2.71 | 0x11 0x30 0x46 0x50 | 0x2000 0x2004 0x4000 |
| 9 | Jin | 280 | R | | 0.114 | 2.71 | 0x12 0x31 0x46 0x51 | 0x2003 0x2004 0x4000 |
| 10 | Julia | 240 | R | F | 0.102 | 2.47 | 0x13 0x32 0x46 0x52 | 0x2000 0x2003 0x4001 |
| 11 | Kuma | 400 | R | | 0.150 | 3.41 | 0x14 0x33 0x46 0x53 | 0x2001 0x2002 0x4000 |
| 12 | Bryan | 380 | R | | 0.144 | 3.29 | 0x15 0x34 0x46 0x54 | 0x2004 0x2006 0x5001 |
| 13 | Heihachi | 340 | R | | 0.132 | 3.06 | 0x16 0x35 0x46 0x55 | 0x2002 0x2008 0x4000 |
| 14 | Ogre | 320 | L | | 0.126 | 2.94 | 0x17 0x36 0x46 0x56 | 0x2001 0x2003 0x4001 |
| 15 | Mokujin | (copied) | | | | | none (0x72) | 0x2000 0x2000 0x4000 |
| 16 | Gun Jack | 500 | R | | 0.133 | 3.59 | 0x18 0x37 0x46 0x57 | 0x2001 0x2002 0x4000 |
| 17 | Roger | 160 | R | | 0.078 | 2.00 | 0x26 0x45 0x46 0x65 | 0x2000 0x3001 0x5000 |
| 18 | Anna | 200 | L | F | 0.090 | 2.24 | 0xE 0x2D 0x46 0x4D | 0x2005 0x2001 0x4000 |
| 19 | Kazuya | 280 | R | | 0.114 | 2.71 | 0x19 0x38 0x46 0x58 | 0x2002 0x2003 0x4000 |
| 20 | True Ogre | 400 | L | | 0.150 | 3.41 | 0x17 0x36 0x46 0x66 | 0x2001 0x2003 0x4001 |
| 21 | Jack-2 | 500 | R | | 0.133 | 3.59 | 0x1A 0x39 0x46 0x59 | 0x2005 0x2004 0x4000 |
| 22 | P. Jack | 460 | L | | 0.155 | 3.52 | 0x1B 0x3A 0x46 0x5A | 0x2005 0x2004 0x4000 |
| 23 | Michelle | 220 | R | F | 0.096 | 2.35 | 0x1C 0x3B 0x46 0x5B | 0x2005 0x2001 0x4000 |
| 24 | Wang | 160 | R | | 0.078 | 2.00 | 0x1D 0x3C 0x46 0x5C | 0x2000 0x2003 0x5000 |
| 25 | Lee | 240 | L | | 0.102 | 2.47 | 0x1E 0x3D 0x46 0x5D | 0x2000 0x2006 0x5001 |
| 26 | Armor King | 360 | R | | 0.138 | 3.18 | 0x1F 0x3E 0x46 0x5E | 0x2004 0x2001 0x3000 |
| 27 | Ganryu | 420 | R | | 0.152 | 3.45 | 0x20 0x3F 0x46 0x5F | 0x2000 0x5000 0x4000 |
| 28 | Kunimitsu | 180 | L | F | 0.084 | 2.12 | 0x21 0x40 0x46 0x60 | 0x2001 0x2003 0x4000 |
| 29 | Devil | 280 | R | | 0.114 | 2.71 | 0x22 0x41 0x46 0x61 | 0x2000 0x2002 0x4000 |
| 30 | Jun | 200 | R | F | 0.090 | 2.24 | 0x23 0x42 0x46 0x62 | 0x2001 0x2002 0x5000 |
| 31 | Bruce | 320 | R | | 0.126 | 2.94 | 0x24 0x43 0x46 0x63 | 0x2000 0x2003 0x4000 |
| 32 | Baek | 280 | L | | 0.114 | 2.71 | 0x25 0x44 0x46 0x64 | 0x2000 0x2006 0x5000 |
| 33 | Unknown | (copied) | L* | F | | | 0x67 0x68 0x46 0x69 | 0x2002 0x2005 0x4000 |

Unknown is left-handed unless she copies Yoshimitsu (0x3E9E90). Strike pose by moveset (0x5237D8, 33 x s16): 4770-4773,
Bryan's moveset 115, Mokujin's 0. **File 128** (scene 42) part 0 = the 114 bowl motions: 0-7 stand / step right /
step left / throw (150 frames) for each hand, 8-0x45 the strike reactions, 0x46 shared, 0x47-0x69 more reactions,
0x6A-0x71 the charges (175, then 60 / 75 frames); parts 1 and 2 (1 and 18 tracks) are presumably the face and hands
as in the endings (section 7.3) [M].

## 5. Scoring (0x3EA4C8, 0x3EA2B0) [H]

Score card (0x5E bytes): +0 ball index b (frame = b / 2, odd = second ball, 20+ = the tenth frame's extra ball),
+1-0xC recent ball indices, +0xD + b pins knocked by ball b, +0x27 + b its mark, +0x42 u16 x 11 running frame totals,
+0x56 total (s16, -1 = not playing), +0x58 pending bonus list.

- After a roll every standing pin is put back exactly on its spot; the fallen ones are swept (+0x174 = 0).
  knocked = standing before - standing after.
- **Marks**: 1 STRIKE (all down, first ball), 2 SPARE, 3 / 4 MISS (nothing down, first / second ball), 5 **FOUL**
  (an OVER CHARGED charge: counts 0 and re-racks), 6 **SPLIT** (first ball, head pin down and two standing pins more
  than 500 apart across x with no standing pin between, 0x3EA3A0).
- **Standard ten-pin scoring**: a strike scores 10 + the next two balls, a spare 10 + the next ball; a strike skips
  the frame's second ball; the tenth frame gives the extra balls; 300 max.
- Next ball (0x3EA2B0): after a strike, a second ball or the last ball, the other player bowls on a fresh rack;
  in the tenth frame a strike or spare re-racks for the same player.

**Callouts** (0x3EF8B0): after a strike, the run n of strikes picks 0x523A18: 1 STRIKE!, 2 DOUBLE!!, 3 TURKEY!!!,
4+ !!GREAT!!, 12 PERFECT; PUNCH OUT for the tenth frame's strikes [M]. Announcer: 0x10B2 (n = 1), 0x1046 (3),
0x1058 (12), else 0x1057, then 0x10B5. MISS / FOUL (0x5F69D8), SPARE / NICE COVER (0x5F6A00, NICE COVER = a split
converted [M]). The results summary counts STRIKES, SPARES, MISSES, FOULS, SPLITS.

**Results** (0x3F1570): "THE KING OF IRONBOWL" top 5 (save block 0x46BC70, FE 6); a 200+ game sets gp-0x5FFC
(JUKEBOX unlock, "BONUS AT 200 PTS"), 300 sets gp-0x5FFA and plays 0x1058. "THANKS FOR PLAYING MISHIMA LANE!" on
leaving. Music: FE 6.

## 6. Camera and HUD [M]

Camera modes (0x3EBC98, table 0x523970, set through 0x1FC51B0 +0x24): 0 fixed behind the lane (0, -2300, -12000),
1 behind the bowler (bowler x, y -2350, bowler z, bowler yaw) during the throw, 2 follows the ball, easing to
(0, -1500, 4000) as the ball nears z 4000, 3-6 the replay's random angles (pi/12, -0.88, pi/6, -pi/4 ... rad), 7 hold.
The aiming guide is a 7-triangle overlay (0x3EA978). HUD texts: SPEED / MPH, BOWL STYLE, SWITCH HANDS (swaps the
hand, 0x3EAE28), REPLAY, OVER CHARGED / TIME UP, the callouts; the score card is drawn by 0x3E4BC0. Gauge art: FE 1
(file 130) [L]. Strings at 0x5232F8 (DISTANCE, ANGLE, FRICTION, WEIGHT, POWER, ENERGY, DR.BOSKONOVITCH) look like
an unused debug display [L].

## 7. Endings

### 7.1 Choosing the ending [H]

`ending = 0x51B678[character * 8 + costume]` (34 x 8 bytes, signed) for the front member; if it was seen and the
partner's was not, the partner's (0x3AA9D4). Seen bits: config +0x10 (0x3BFD68). Only these depend on the costume:

| Character | Costume -> ending |
|---|---|
| Xiaoyu (7) | 0-3, 5 -> 9; 4, 6, 7 -> 10 |
| Eddy (8) | 0-3, 5 -> 12; 4, 6, 7 -> 33 (Tiger) |
| Kuma (11) | even -> 29 (Kuma); odd -> 30 (Panda) |
| Devil (29) | 2, 4 -> 14 (Angel); others -> 32 |

### 7.2 Descriptors 0x51B488 (38 x 12: `{script*, u16 arena, u16 0, cast*}`) [H]

Arena = the stage id (gp-0x743A), loaded as for a fight. Cast = `{s16 character, s16 costume}` up to a negative
character; costume -1 = the player's (selection & 7; for character 2 & 6 [M]). Two actor slots; the script swaps cast
members into them. Frames = the camera path's length = the scene's length.

| E | Arena | Cast (P = player's costume) | Frames | E | Arena | Cast | Frames |
|---|---|---|---|---|---|---|---|
| 0 | 8 | Jin P, Kazuya 1 | 1200 | 19 | 3 | Michelle 0, Ganryu 4 | 1020 |
| 1 | 6 | Paul P, Kuma 2, Kazuya 0 | 1260 | 20 | 5 | Lei 3, Bryan P | 1080 |
| 2 | 1 | Paul 2, Law P | 1080 | 21 | 11 | Gun Jack P, Jack-2 0 | 1440 |
| 3 | 15 | Lei P | 1560 | 22 | 8 | Jin 2, Heihachi P, Kazuya 0 | 720 |
| 4 | 11 | King P, Armor King 0, Mokujin 6 | 1800 | 23 | 12 | Bruce P | 1140 |
| 5 | 9 | Nina P, Anna 0 | 1260 | 24 | 10 | Jack-2 P, P. Jack 0 | 1200 |
| 6 | 9 | Nina 2, Anna P | 1260 | 25 | 8 | Heihachi 2, Kazuya 0, Lee P | 720 |
| 7 | 22 | Yoshimitsu P, Kunimitsu 0 | 1500 | 26 | 15 | Xiaoyu 3, Wang P | 1260 |
| 8 | 19 | Yoshimitsu 2, Kunimitsu P | 1440 | 27 | 0 | Yoshimitsu 2, Roger P | 1140 |
| 9 | 10 | Xiaoyu P, Kuma 3 | 1140 | 28 | 6 | Jin 2, Kazuya P | 1242 |
| 10 | 7 | Xiaoyu 4, Jin 4, Kuma 2 | 900 | 29 | 17 | Kuma P, Heihachi 2 | 1140 |
| 11 | 0 | Hwoarang P, Jin 3 | 960 | 30 | 7 | Kuma P, Kuma 2 | 840 |
| 12 | 13 | Eddy P, Eddy 4 | 1080 | 31 | 0 | Gun Jack 2, P. Jack P | 840 |
| 13 | 14 | Jin 2, Jun P | 1380 | 32 | 14 | Devil P, Jun 0 | 1200 |
| 14 | 14 | Kazuya 0, Devil 0, Devil 4 | 1380 | 33 | 3 | Eddy 4 | 960 |
| 15 | 13 | Julia 2, Michelle P | 1200 | 34 | 6 | Jin 2, Ogre P, Unknown 0 | 1500 |
| 16 | 10 | Julia P, Michelle 0 | 840 | 35 | 14 | True Ogre P, Devil 0 | 1350 |
| 17 | 4 | Hwoarang 2, Baek P | 1320 | 36 | 21 | Mokujin P | 1620 |
| 18 | 11 | King 2, Armor King 4 | 1320 | 37 | 1 | (unused: Unknown plays a movie) | 600 |

Arenas 21 and 22 appear only in endings 36 and 7 [M].

### 7.3 Scene files [H]

Ending e is scene n = e + 2 (gp-0x63BE). **Camera**: file 2n + 43 = **2e + 47** (0x392E00 -> 0x35D238): `u32
frames`, then 7 f32 a frame (eye xyz, target xyz, fov; the arena camera path format). **Motion pack**: file 2n + 44
= **2e + 48** (0x363FB8 -> 0x35CFD0; scene 1 -> file 42, scene 42 -> file 128), a pack of 3 parts, each a pack of
tracks `u16 frames, u16 triples`, then frames x triples x 3 s16, uncompressed; track i belongs to actor slot i.

| Part | Triples | Content | |
|---|---|---|---|
| 0 (gp-0x657C) | 19 | the 57 motion channels of FORMATS 8 (root heading, height, distance, 18 joints x 3), replacing the motion decoder's output (0x3639F0 -> 0x3640B0) | [H] |
| 1 (gp-0x6578) | 33 | the **face**: like fighter-pack part 5; a 3-frame dummy for masked actors (King, Armor King, Yoshimitsu, Kunimitsu, the Jacks, Kuma, Mokujin, Ogre); used when fighter +0x3088 >= 50 (track = id - 50) | [M] |
| 2 (gp-0x6574) | 15 | two tracks per actor (2a, 2a + 1): the **hands**, 5 fingers x 3 joints (fighter +0x3074 / +0x3078 >= 50) | [M] |

Every track is sampled at the camera's frame (gp-0x63A0), clamped at its end: all frame-locked. Ending files are
47-122. Scene 0 (file 41) is a set of 22 camera paths. **Scene 1** (files 41 / 42) is the **EMBU** exhibition
(0x3A97B0, title loop and Theater): two 6-actor casts at 0x437CE0 (Paul, Hwoarang, Xiaoyu, Eddy, Jun 7, Jin 6; or
Kuma 0 / 2, Panda, Roger, Alex); once config +0x37 is set the two alternate (gp-0x6E94). This corrects FE 7's
"+0x37 changes the ending scene" [M]. 123-126 are unreached 600-frame one-actor scenes; 43 is a near-empty pack [M].

### 7.4 Script (10-byte `{s16 frame, s16 op, s16 a, b, c}`, interpreter 0x3AAFF4, table 0x51B7F0)

Every record whose frame <= the scene timer (gp-0x63B4, +1 a frame) runs; op -1 or the camera's end ends the
scene; hard cap 7200 frames. [H]

| Op | Meaning | |
|---|---|---|
| 0 / 1 | put cast[a] into actor 0 / 1 (model, costume rule, position reset) | [H] |
| 2 / 3 | actor 0 / 1 plays track a (body 0[a], face 1[a], hands 2[2a, 2a + 1]) | [H] |
| 4 | camera = path a from frame b (c = 1800, unused) | [H] |
| 19 / 20 | fade in / out from / to black over a frames (0x3AC5D0) | [H] |
| 5 / 6 | flash (alpha b to 0 over a frames) / fade to colour 10 | [M] |
| 10, 17, 18 | effects at an actor's joint b (0x3A2BC0, 0x3A2758) | [L] |
| 12, 16, 26-28, 31 | show / hide, position offset (0x378368), flags, clear objects, Kuma / Panda +0xB052, 0x34CB38 | [L] |
| 21-25, 29, 30, 32 | spawn 2D / 3D effect objects (0x3AC7C0 ... 0x3AA410) | [L] |
| 7-9, 11, 13, 14 | no-op | [H] |

Typical: ops 0-4 at frame 0, fade in over 45, effects, fade out over 60-120, end at camera frames - 1. No op plays
sound: the only audio is the music [M]. Thirteen endings swap the arena's background-character set (0x3D85D8,
table 0x521C30): 2, 4, 7, 9, 10, 12, 15, 16, 23, 24, 30, 33, 36 [M].

### 7.5 Playback and staff roll (0x3AA8A8, table 0x51B7B0) [H unless marked]

Arcade clear: pick and mark the ending. **Unknown** plays movie 2 (file 16; table 0x518EA8: 0 = file 14 opening,
1 = file 15 arcade), then the staff roll on BGM 6. Everyone else: load arena, cast, camera, pack, BGM 6 (STAFF
ROLL), then the staff roll runs 4620 frames (77 s) first; START skips it, the music jumps to BGM 7 (file 20, 0xCF7
sectors in), and the scene plays (START skips, 4-frame fade); the game then waits for the staff roll to end or
START [M on credits over the scene].

Staff roll 0x3AC018 (12 states, table 0x51DCB0): cards "TEKKEN TAG TOURNAMENT", "NAMCO PRODUCTION STAFF", logos
0xAB / 0xAC (170 frames each), then list 0x51B880 (434 `{u32 kind, char *text, u32 font}`: 0 heading (font 7), 1
name (5), 2 blank, 3 centred final page, 4 last lines; flag 0x10000 = alternative: "SECRET MODE PROGRAMMER" until
Tekken Bowl is unlocked, then "TEKKEN BOWL PROGRAMMER"); scroll 1.5 px a frame from y 480, a line every 32 px, up to
64 on screen; ends "TM AND (C)1994 1995 1996 1999 / NAMCO LTD. / ALL RIGHTS RESERVED".

## 8. Theater and Gallery

**Theater** (screen 0xC, shown when config +0x10 != 0) [H]: one row per character 0-33, labels 0x455E50 (8 per row,
by costume: "(1P)PAUL".."(4P)PAUL", "(5P)TIGER", "(2P)PANDA", "(3P)ANGEL" ...). A cell (row x 8 + costume) is
available when labelled, `0x51B678[cell] != -1` and that ending seen (0x3544C8; 2 when it differs from the row's
first-costume ending [M]); a row shows if any of costumes 0-5 is (0x3545A8). Choosing plays that ending (screen 0xD:
no staff roll, BGM 7, ends with the scene); Unknown's row plays movie 2. Then:

| Cell | Entry | Plays / gate |
|---|---|---|
| 0x1C0 | OPENING MOVIE | movie 0 (file 14) |
| 0x1C1 | ARCADE MOVIE | movie 1 (file 15) |
| 0x1C2 | EMBU | scene 1, cast 0 |
| 0x1C3 | EMBU2 | the animal cast; only when config +0x37 |
| 0x1C8 | MUSIC PLAYER | screen 0xE: table 0x455D80 `{s32 BGM, char *name}`: SELECT 0, EMBU 1, RESULT 2, ENDING REPLAY 4, NAME ENTRY 5, STAFF ROLL 6, CONTINUE? 8, PAUL..UNKNOWN 9-22, EXIT; no unlocks [M] |
| 0x1D0 | EXIT THEATER MENU | |

**Gallery** (screen 0x10, 0x3E5910; unlocked at stage 14, config +0x23) [H]: up to 12 of the player's own
screenshots, `SCRN-01.CPT`..`SCRN-12.CPT` on the memory card (100 KB each), VIEW / DELETE / EXIT; captures come from
the pause menu's SCREEN CAPTURE (needs a save). The first visit (config +0x24 bit 0) opens a help page [M]. Nothing
to convert.

## 9. Not decoded

Ranked by how much a player would notice:

1. The pin simulation's inner details: ball-pin and pin-pin impulse response (0x3EDC90, 0x3EDF88), the pin's mass
   and inertia, the exact down test. A port needs a rigid-body step with the constants of section 3; it will not
   match pin for pin without decoding these.
2. Ending effect ops (10, 17, 18, 21-25, 29, 30, 32): the visible flourishes in many endings.
3. The face part's joint mapping (33 triples).
4. The replay: when it is offered (gp-0x603A / -0x602E) and its camera angles.
5. Whether the staff roll draws over the ending scene; the flash / fade colours (table 0x43FD10, 9 and 10).
6. The HUD layout (0x3E4BC0) and the bowl gauges' sprite ids.
7. The BOWL STYLE menu's other entries; HELP MESSAGES / SPECIAL EFFECTS behaviour (FE 6).
8. File 128 parts 1-2 and file 127 (the bowl's camera path, 600 frames) use.

## Scripts (`docs/re/`)

`dump_bowl.py` (section 4's table, the strike poses, the pin rack and the physics constants), `vdis.py`
(`python3 vdis.py 3EC768`: like `fn.py`, with the EE's VU0 macro instructions, `lq` / `sq` and `sqrt.s` decoded),
`dump_endings.py` (`python3 dump_endings.py SLUS_200.01 [RAW_DIR]`: the ending table, the 38 descriptors with
casts, each script as text and the scene files' frame counts).

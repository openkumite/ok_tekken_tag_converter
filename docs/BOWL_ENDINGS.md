# Tekken Tag Tournament (PS2, USA v2.00, SLUS-20001): Tekken Bowl, the endings, Theater and Gallery

Seventh pass, worked out on 2026-11-18 from `SLUS_200.01` (static analysis) and the disc's raw files. Memory address
= file offset + 0x33F000, gp = 0x5FD0F0. Confidence: [H] read in code, [M] read in code but the meaning is inferred,
[L] a guess. "FE n" is a section of [FRONTEND.md](FRONTEND.md), "FD n" one of [FIGHT_DATA.md](FIGHT_DATA.md).
Characters are character ids (FE: 0 Paul ... 33 Unknown). Units are the game's: 60 frames a second, y is down,
angles in 1/65536 turns (0x800 = 11.25 degrees). The VU0 macro code (`lqc2` / `vadd` ...) that capstone misreads
was read with `docs/re/vdis.py`. Sections 1-6 are exported for OpenKumite as `assets/gamedata/bowl.json`, the motions
`assets/motions/bowl_NNN.glb` and the ball, pin and pin deck `assets/stages/bowl_*.glb` (GAMEDATA.md).

## 1. Tekken Bowl: structure

**Memory** [H]

| Address | What |
|---|---|
| 0x1FC51B0 | camera `{f32 x, y, z, .., +0x14 yaw, +0x20, +0x24 mode}` |
| 0x1FC51E0 | ball, 0x80 bytes: +0x00 position, +0x10 velocity, +0x20 spin vector, +0x30 / +0x40 previous position / velocity, +0x50 roll angles, +0x60 roll speeds (vec4 each), +0x70 rolling, +0x74 shown, +0x78 has landed |
| 0x1FC5260 | 10 pins x 0x180: +0x00 centre, +0x10 velocity (a frame), +0x20 the frame's start, +0x30 the frame's move, +0x40 angles (x tilt, y heading, +0x4C twist), +0x50 their speeds, +0x60 / +0x70 at the frame's start / the frame's change, +0x80-0xB0 the four spheres' centres, +0xC0-0xF0 at the frame's start, +0x100-0x130 their moves, +0x140 / +0x150 / +0x160 the base's centre, at the start, its move, +0x170 standing on its base, +0x174 on the deck (not yet swept), +0x178 not touched yet, +0x17C touched this frame, +0x17E clatter timer |
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
| 9 | 0x3F0950 | in the throw's first step START opens Tekken Tag's pause menu with Bowl's entries (gp-0x71E8), TRIANGLE the whole score card (gp-0x6042, any face button closes it) |
| 10 | 0x3F0AE0 | winner (higher total; draw), records, results 0x3F1570 |

**Replay** [H] (format 23). As the ball passes z 4000 (state 1 -> 3) 0x3EF1D0 keeps the ball (0x1FC62C0), the
bowler's motion frame and two random orders of 0, 1, 2 (0x1FC6340, by 0x3E92E0), and zeroes the frame count
gp-0x6046. State 0 allows gp-0x603A = 3 replays on a frame's first ball (an even ball index), none on its second.
0x3EFB00, each frame of state 3: the first frame no pin moves is kept (gp-0x6036); once every pin is down (+0x170
clear) the end is set halfway between that frame and now; before frame 91 a replay starts at once (state 2), else
gp-0x602E and one replay are set for when the roll is over, played with the physics stepped every other frame
(gp-0x602C). A replay (0x3EF290) puts the ball back, re-racks the pins (the physics has no randomness: the same
roll again), zeroes the count, sets gp-0x6034 and draws its camera (mode 3: around (0, -290, 8000) at 4000, pitch
(order1[k % 3] + U) pi / 12 - 0.8796, yaw (order2[k'] + U) pi / 6 - pi / 4, k the replays left); playing, it starts
again past the kept end while any are left. START in a replay knocks every pin down and counts (a strike). An
OVER CHARGED charge that strikes gets 9 replays of 60 frames.

**The strike's pose** [H] (states 5-7, 0x3F0670): the other player's fighter is loaded as the bowler's team's second
member (0x3E99C0 slot 1; a Mokujin there takes the bowler's moveset), the bowler stands at z -6500, the partner at
-9500 and 1000 to the side (-x for player 1), both take their strike pose move (0x5237D8) and play the second of
their reactions (0x523590 + 1, 0x3E9550: the copied character's row for Mokujin) frame by frame until both are over
or START (0x3E9628); camera mode 4: 3500 in front of the bowler, 500 up, looking back (yaw pi), then mode 5 eases
after the body (0.97 / 0.03 a frame). State 7 loads the other player's own fighter again. So Bowl's select takes a
team of two [M: from the slot-1 loads and the records' two bodies].

There is **no CPU bowler**: a player whose pad slot gp-0x7440[p] is empty is skipped, so one or two humans take
turns [M].

## 2. The throw (0x3EAF78, phases in 0x1FC62A0 +0, table 0x523930) [H]

Pad: held bits for the d-pad, pressed bits for buttons; 0x2000 right, 0x8000 left, 0x40 CROSS, 0x20 CIRCLE, 0x10
TRIANGLE, 0x100 SELECT, 0x800 START.

| Phase | Name | Rule |
|---|---|---|
| 0 | start | slot +0xC (stand), prompt 1 |
| 1 | **position** | left / right: bowler x += -/+ 8 a frame, clamped to +-1000 (start x random in [-64, 64), z -11000); step-left / step-right / stand slots; SELECT switches the hand (0x3EAE08, below); START the pause, TRIANGLE the score card (state 9); CROSS -> 2 |
| 2 | **aim + spin** | left / right turn the bowler +-0x10 a frame, clamped +-0x800 (+-11.25 degrees); the spin meter swings: m += v each frame, v reverses outside [-2, 1] (right-handers), [-1, 2] (left-handers), [-1.2, 1.2] (movesets 0x10 Gun Jack, 0x13 Jack-2); CIRCLE back to 1; CROSS locks it -> 3 |
| 3 | **power** | p += power speed a frame while p < 100, else p = 0 (it wraps). CROSS: if p > 0 -> 4; if p == 0 (the frame it wrapped): "OVER CHARGED" with probability S / 2000, else 4 |
| 4 | run-up | throw motion (slot +0xF, 150 frames) from frame 1; left / right still steer the bowler by 1280 / S a frame (+-1000); the ball leaves the hand at **motion frame 106** |
| 5 | message | "OVER CHARGED" (or "TIME UP") for 60 frames -> 6 |
| 6 | charge | the bowler runs down the lane with slot +0x10 (0x6A-0x6D, 175 frames): distance 256 f for f < 25, 256 f - (f - 25)^2 to 50, a further ease to 75, then 14825; voice 3 at frame 25; the pins react to the body (0x3EEBF8); then slot +0x11 (0x6E-0x71) |
| 7 | done | back to stand |

**SWITCH HANDS** (0x3EAE08 -> 0x3EAE28) [H]: SELECT in the position step switches the hand of characters 2, 6, 11,
22 and 30 (Lei, Hwoarang, Kuma, P. Jack, Jun; never Unknown, moveset 0x21): the motion slots (0-3 / 4-7), the
charge's (0x6A / 0x6E, +2 the female style, +1 left), the hand joints and +0x12, the step back to 0; gp-0x6032 = 120
frames of "BOWL STYLE" (239, 364; size 4, colour 15) and "SWITCH HANDS:%C" (174, 384; size 5, colour 15), %C R or L
(0x5F69B0). Nobody else's hand changes: SELECT does nothing.

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

Step 0x3EED88 per frame: the ball's frame (0x3EC768) or the runner's (0x3EBB80); for each pin on the deck (+0x174)
its frame (0x3EC9A0) and the ball against it (0x3EDC90; the runner: 0x3EEBF8), then the ball's walls (0x3EE2F8); then
for each pin on the deck, each later pin against it (0x3EDF88), the lane (0x3ED338) and the masking (0x3EE598). [H]

The pin's body [H]: four spheres along its axis (0x46BC20: y -249.34, -99.34, 100.66, 225.66 from the centre; radii
0x523570: 80, 60, 100, 75; their shares of the mass 0x523580: 1, 0.6, 1.5, 0.7) and the base's centre (0x46BC60: y
250.66); mass 3.8 (gp-0x7668), turning 6.258e-6 (gp-0x765C). Its matrix (0x3E9120 from the identity 0x1FC61A0) turns
the tilt about x, then the heading about y: the axis (centre to base) is (sin y sin x, cos x, cos y sin x). Its frame
(0x3EC9A0): the start kept, centre += velocity, the angles turned by their speeds (0x3EC388: heading += w.y; the tilt
by asin(sin w.x cos y - sin w.z sin y), plus asin(sin x cos x a^2) with a = sin w.z cos y + sin w.x sin y; the
heading by -atan2(a cos x, sin x') (x' the new tilt within +-pi; +atan2(a cos x, -sin x') below 0) unless cos x or a
is 0), then 0x3ECAD8 places the spheres and the base and their moves. Every touch is found within the frame:
0x3EDB98 gives the first time t in [0, 1) two spheres moving apart by d from a gap g meet: 0 if they already overlap,
else (-b - sqrt(b^2 - |d|^2 (|g|^2 - r^2))) / |d|^2 with b = d.g < 0. Both bodies go back to t (0x3ECA38: start +
t x move), take the push, then go on for 1 - t at their velocities (0x3ECA80).

- **Ball and pin** (0x3EDC90): the first of the four spheres to meet the ball (radius 150); the touch point on the
  sphere's side (r / (r + 150) of the way to the ball's centre n); the relative move v = the sphere's (0x3ECBF0: its
  own move plus the centre's move along the axis a by (a.move) (3.8 - w) / (|a|^2 w)) less the ball's; if v.n > 0:
  j = 2 (v.n) w / ((w + 30) |n|^2), the ball's velocity = its move + j n with y 0, the pin pushed by -30 j n
  (+0x178 = 0, +0x17C = 1; a vibration 5).
- **Pin and pin** (0x3EDF88): the first pair of spheres of the sixteen; the touch point r_q / (r_p + r_q) of the way
  from q's; if (v_q - v_p).n > 0: j = 2 (v.n) w_p w_q / ((w_p + w_q) |n|^2), p pushed by j n and q by -j n.
- **A push** (0x3ECDE8) J at a point r from the centre: its part along r, J (J.r)^2 / (|J|^2 |r|^2), sets the
  velocity to the frame's move plus that / 3.8; the rest x 6.258e-6 turns it: speeds += r x rest.
- **The lane** (0x3ED338, y 0 the floor): the first sphere to reach it under gravity (t = (sqrt(d^2 - 9.8 y) - d) / 4.9
  for a sphere y below at the start moving d; 0 when it already touches; the last of equal times). The belly (2) on a
  pin tilted past pi / 2 + asin(0.05714) - 0.005 either way is its whole body (4). The base sphere (3) touching within
  sqrt 5000 of the base's centre is the base: its rim's lowest point (sqrt(5000 / |axis|^2) of the axis' length out)
  meets the floor at its own time, else the pin falls freely. Back to t with gravity (y += 2.45 t^2, velocity +4.9 t);
  moving down: an upward push of the move's y x -(mass at that point) x 1.8 (0x3ECD00: w + (3.8 - w) |a.y| / |a|),
  the body's -6.84; still in the floor after the rest of the frame (0x3ED268): another, 4.9 x mass x rest x 0.8 (the
  body's 18.62 x rest x 0.8), the y velocity 0, and friction: lying (0x3ED118: the belly past the angle, or the head
  short of upside down) the slide slows by 0.25 x rest a frame and the turn by 0.0008 x rest (to a stop: all speeds 0),
  standing on its base within 0.005 of upright by 0.75 and 0.0008 (a stop sets the tilt 0). Then on for the rest with
  gravity, out of the floor by the deepest sphere (0x3ECF70; the base by its rim's lowest point), and a pin not on its
  base this frame no longer stands (+0x170 = 0). No touch: y += 2.45, velocity +4.9.
- **The masking and the pit** (0x3EE598, pins past z 9255.66): the first sphere to cross the masking's front (z 9585,
  y -3528 to -1176), its underside (y -1176, z 9585-12585) or the pit's sides (x +-2700, y -1176 to 0, z 9585-12585),
  within 0.707 r of their edges, is put back on it and bounces at 0.4. The pins have no other walls.
- **The runner** (0x3EEBF8): each of its 14 body spheres (+0x1F0: x, y, z, s16 radius; 0x14 each) overlapping a pin's
  sphere pushes the pin out of it and by 150 away from its centre.

A pin **counts as down** once off its base (+0x170 = 0): the lane clears it, and 0x3EF3A8 (the end of the roll's test)
clears it for a pin still moving farther than 3000 from (0, 8000). Moving: any of its velocity or angle speeds not 0.
There is no random element in the physics: the randomness is all in the release (section 2) [H].

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

**Records** [H] (format 23). Save block 0x46BC70: 5 entries of 0x66 bytes `{u8 1, u8, u8 body, u8 body (the
team's two), char name[4], score card 0x5E}`, then u16 +0x1FE / +0x200 (the BOWLING OPTIONS' two switches, 0x3F1F18),
+0x202 the JUKEBOX unlock and +0x204 the 300 flag (OR-ed in by 0x3F1F30). 0x3F1408 places each player's game
(0x3F2128): from the bottom up, the new score passes every entry it is not below (an equal one too), the last
falls out; the second player cannot pass the first's new entry. The new entry's name starts "A" (the name entry,
0x3F2EB8). gp-0x6000 / -0x5FFE are the two players' places (5: none). 0x3F1570 sets gp-0x5FFC for a placed game of
200 or more, gp-0x5FFA for a placed 300 (with 0x1058). Defaults (0x3F1F58, cards 0x523C10, bodies 0x523C00, the
name the first body's three letters): GAN (Ganryu, Michelle) 268, ALE (Alex, Kuma) 214, PAN (Panda, Xiaoyu) 157,
TIG (Tiger, Anna) 103, GUN (Gun Jack, Yoshimitsu) 30. Texts: SCORE SHEET, YOUR SCORE:%3d, RANK %s:%3d, NEW
RECORD!, PLAYER%d, WINNERS / LOSERS / DRAW, SUMMARY with STRIKES SPARES MISSES FOULS SPLITS, 1ST-5TH, TOP 5
BOWLERS, EXIT:START. The pause (state 9) also shows the HELP MESSAGES instructions first when gp-0x6042 is set
(any face button goes on).

## 6. Camera and HUD

**Camera** (0x3EBC98, table 0x523970; the struct 0x1FC51B0: pivot x, y, z, +0x10 pitch, +0x14 yaw, +0x20 distance,
+0x24 mode) [H]. The eye is the pivot less the view's direction (pitch about x, yaw about y) times the distance; each
turn starts (0x3E9C10) at pivot (0, -2300, -12000), pitch -0.2 rad, distance 2000. The throw's phases set the mode:

| Mode | Set by | Does |
|---|---|---|
| 0 | position (0x3EB05C) | pivot (0, -2300, -12000), distance 2000, yaw 0: the fixed view behind the lane; then 7 |
| 1 | aim (0x3EB290) | pivot (bowler x, -2350, bowler z), distance 0, yaw the bowler's: from the bowler's eye along its aim |
| 7 | power, run-up (0x3EB424, 0x3EB528) | hold |
| 2 | the release (0x3EB7B0), a charge (0x3EB888) | t = ((ball z - 4000) - pivot z) / (4000 - pivot z): while > 0 the pivot moves t of the way to (0, -1500, 4000) (0x523960) and yaw and distance shrink by 1 - t (the pivot's z ends 4000 behind the ball); at t >= 1 there, yaw and distance 0, then 7 |
| 3 | a replay (0x3EF2A4) | section 1 |
| 4 | the strike's pose (0x3F071C) | pivot = the bowler's body (+0x590) 500 up, yaw pi, distance 3500, pitch 0; then 5 |
| 5 | | the pivot follows the body: halfway a frame, a Hwoarang (moveset 6, a Mokujin copying him) 0.03 of the way |

The aiming guide is a 7-triangle overlay (0x3EA978).

**The throw's gauges** (0x3C65D0, every frame; the step's row of table 0x51FD30 gives each one's alpha: position
the rack, aim the spin meter and the power bar at half, power the rack, half the spin meter and the power bar, run-up
all three at half; none from the release) [H], pictures of file 130 (`130_NNN.png`):

- the rack (0x3C6280, only for a frame's second ball, 0x3E9450): `130_015` at (524, 20), a dot a pin at (+37, +66),
  (+26, +46), (+48, +46), (+15, +26), (+37, +26), (+59, +26), (+4, +6), (+26, +6), (+48, +6), (+70, +6) (0x51FCE0),
  `130_024` standing, `130_023` down;
- the spin meter (0x3C6410): the arc `130_019` at (50, 280); the needle `130_022` centred at (138, 369) + 94 (cos a,
  sin a), a = (spin / 4 - 0.5) x 3.14, turned spin / 1.3 (gp-0x7D94);
- the power bar (0x3C64E8): `130_020` at (550, 140), 64 x 246; its lowest power x 236 / 100 + 4 rows in `130_021`.

**Texts** (the printf 0x3C6C60 in the chrome font, `x` the left, `y` the foot) [H]: SPEED (464, 396; size 4), the mph
whole part `%3d` (448, 412; 5), `.%02d` (512, 417; 4), MPH (564, 422; 3), all colour 5, in states 3 and 4 (0x3EAD18;
" --" ".--" for a charge); REPLAY (48, 412; size 5, colour 14) blinking while the slow replay after the roll plays
(0x3EF348); OVER CHARGED (113, 240; size 8, colour 24 / 12 by frame & 2; TIME UP at 208); K.O. (state 4 from frame 90,
blinking: 270 + 224 x side, 200; size 5, colour 12; an arrow `{` / `}` at 304 + 274 x side, 204; side -1 for the 13
people on +x); the help band with help text 105 over a K.O. and a foul. **Callouts** at y 360, 150 frames (gp-0x6022):
the strikes' (0x3EF8B0: x 178, 144, 129, 125, 126, 170 and colours 2, 9, 18, 11, 18, PERFECT blinking 11 / 2 for
STRIKE! .. PERFECT; from a TURKEY on a chicken a strike, `003_016`, 36 apart from (644 - 36 n) / 2 at y 296); SPARE /
NICE COVER (0x3EF660: widths 214 / 420, colours 17 / 7, sliding 32 a frame from the bowler's side to the middle);
MISS / FOUL (0x3EF470: x 242 / 232, colours 1 / 10, dropping from y -20 at 5 a frame, 0.5 more each, rebounding from
360 at half the speed). HELP MESSAGES on: the help band and help text 93 + 4 x ball + step (the frame's first,
second or third ball; position, spin, power, the run-up's steering) during the throw (0x3F0330).

**The score card** (0x3C59D8; a row a player, 68 high, the second's 46, then a 4-high bottom; pieces of file 130 each
over a faint green rectangle, colour 0x20009050, 0x3DBE98) [H]: the left edge (`130_000`-`002`, 4 wide), frames
(`130_003`-`005`, 47 wide; the tenth `130_006`-`008`, 70) with their number at (+15, +25), the balls' marks at
(+5, +27, +50; foot +47, the second row's +26): `%d` the pins, a strike's `130_017` and a spare's `130_016` (21 up), G a
first ball's miss, - a second's, F a foul (colour 1), a split's count (colour 5), the next ball an arrow `130_018`
blinking (frame & 0x30); the running total centred on +24 (+34) at foot +69 (+48); the total cell (`130_009`-`011`,
54) when asked; the right edge (`130_012`-`014`). All in the small font (11), colour 6. In the throw's position step
four frames at (20, 20), the bowled one and three before it (0x3EA868 from 0x3EB248); the whole card at (63, 128)
for TRIANGLE (state 9).

**Pause** (state 9, START): Tekken Tag's pause (FRONTEND.md 8, 0x3B7B98) with entries CANCEL, RESET, TOP BOWLERS,
BOWLING OPTIONS, ADJUST DISPLAY, CONTROLLER SETUP (mode 8 skips 2 and 3, the others skip 4 and 5; window
`h_capture` high). TOP BOWLERS: TOP 5 BOWLERS (0x3F2D00, below). **BOWLING OPTIONS** (0x3F1968): the pause's window
at (135, 172, 370, 88); HELP MESSAGES:, SPECIAL EFFECTS: (155, 200 / 220; size 4; the cursor's colour 11 over the
bar to x 291 / 322 + 185 (0x3F1898), else 5; the value ON 16 / OFF 12 after it; the arrow at 450 / 483); JUKEBOX:
and its track (0x455D80) once a 200 game unlocked it, else BONUS AT 200 PTS (colour 17) at 240; EXIT at 260; the
cursor row's help text 106 + row. Both switches are ON at first (0x3F20E8), kept at save block +0x1FE / +0x200.
SPECIAL EFFECTS (0x3E9808, each frame of the roll): the first frame every pin has been touched (+0x178 all 0,
gp-0x6028), with the switch on (0x3F1EE8): the sound 0x10A8, the burst 0x3A2298 at 300 along the direction from (0,
-225, -8000) to the eye (x, -y, 8000 - z) (0x343F20(7): set 7, the hall's geometry pack part 1 entry 1 (EFFECTS.md 5), stages.json `weather.sets[1]`: 51 paths
of 40 frames; a particle 0x3A04B8 on each, 8.16 times its path (gp-0x7E88), 0x44 180, 0x48 75, added (+0x40 1)), and 0x3E97C8(1, player): the shake 0x35C2F8(1) (pattern 0x4FBB38,
added to the view's pitch +0 by 0x35C320 after the camera each frame: 3, 2, 0, -2, -3, 0, 3, 2, 0, -3, -2, 0, 2, 1, 0,
-2, -1, 0, 1, 0, 0, -1, 0, 0 in 4096ths of a turn) and a vibration (0x3BB218). The pause's TOP BOWLERS (0x3F3118,
the cursor reset by 0x3F3108): over the hall as it stands (no shade of its own; 0x3CD868 sets the blend), the
cursor's row lit by the band 0x3F2A38 (the screen's width, 63 above the row's y, grey 128 at alpha 0x40), then 0x3F2D00
and THE KING OF IRONBOWL (112, 40); up / down move it round the five, a button on a row opens that record's card
(0x3F25D8; a 300 game's sound 0x1058, else 4), START leaves; the help line 109 or EXIT:START (72, 440).
The pin's twist (+0x4C, 0x3EC388's second half): the identity's z row turned about x by the new tilt, about z by
asin(a), about y back by the heading's step, about x back by the final tilt; the twist += atan2(its x, its z). Drawn
(0x3E91D0 from 0x1FC6160) turned about y by -twist, x by the tilt, y by -heading.
**SCORE SHEET** (0x3F25D8, a record's card; a button closes it): SCORE SHEET (203, 80; size 5, colour 10); RANK
1ST..5TH:%3d (106, 212; size 8) with the score, in colour 11 / 2 / 9 / 21 for 230+ / 170+ / 100+ / less (a 300 game
blinking 11 and 2, frame & 2); the card (0x3C59D8) at (36, 100), frames 1-10 and the total; the team's two bodies'
portraits (0x3CF1C0) at (204, 296) and (204, 376); SUMMARY (365, 259; size 5, colour 19); STRIKES, SPARES, MISSES
(both kinds), FOULS, SPLITS (336, y; size 5; colours 2, 17, 1, 10, 22) and `:%2d` their counts (476, y) at y 259,
285, 311, 389, 415; the rank's pin (0x3F2480: arena object 1 for 300, 4 / 5 / 6 by the rank, turning with the frame
counter). The records' cards are the defaults' at 0x523C10 (section 5's layout; frame k at ball 2k).

**TOP 5 BOWLERS** (0x3F2D00 / 0x3F2AC0): the title (183, 64; size 5, colour 10); a record every 71 from y 135: its
place `%2d` (16, y - 27; size 10) and ST / ND / RD / TH (y - 7; size 5); the team's faces (`003_110` frames, 51 x 52
faces) at (176, y - 57), 52 apart; the score (size 9) ending on 364 (x 364, 326 or 288 by its digits) at y - 23 and
PTS (408, y - 3; size 4), colour 21; the name (468, y - 27; size 10). The new record's row: a band (0x3F2A38) and
colour 11. **Results** (state 10, 0x3F0FF0): a player every 176: its team's faces from (216, 64), 64 apart; PLAYERn
(48, 116; size 5, colour 14); WINNERS (blinking 2 / 11), LOSERS (12) or DRAW (10) right-aligned on 472 (widths 115,
101, 70) at 78 and the total `%3d` (415, 110), size 8, in that colour; the whole card at (36, 128); a placed game's NEW
(-26, -6) RECORD! (-60, +12) taking turns every 64 frames with 1ST..5TH (-32), blinking, from (123, 84) (0x3F0E28).
**THE KING OF IRONBOWL** (0x3F3108: 112, 40; size 5, colour 15) over TOP 5 BOWLERS: a placed player's name entered
with Tekken Tag's letters (0x3F2EB8 -> 0x3D0450), EXIT:START (72, 440; size 4, colour 5) on the help band.

**The ball and the pins drawn** [H]: the roll's draw 0x3EEF00 calls the arena-object drawer 0x3D10D0 with object 0 at
the ball (its matrix from 0x1FC61A0, its roll angles +0x50), object 2 its shadow, then for each pin on the deck
(+0x174) object 1 at the pin (matrix 0x1FC6160, angles +0x40) and object 3 its shadow. The objects are the arena's
(0x367938 -> 0x3D0CC8 / 0x3D0E18): table 0x520FC0 + 16 x arena `{u32 links, u32 *links, u32 count, u32 *objects}`,
an object 64 bytes `{s16 model, ..., +0x20 f32 x, y, z}`, a link `{s16 parent, s16 child}`; the models are the blocks
of the geometry pack's object part (file 329's part 3, a grid of one cell and 13 blocks). Arena 20: object 0 = model
1 (the ball, radius 158), 1 = model 2 (the pin, 661 tall: the hall's gold pin), 2 / 3 = models 3 / 4 (the shadows),
4-6 = models 5-7 (three more pins, not drawn by the bowl), 10 = model 12 at z 10000 (the pin deck's masking unit)
with 7-9 (models 8, 10, 11) linked to it at (0, 0, 3000), (0, 2500, 2000), (0, 2500, 1000): its back, its round
sign and the sign's glow; models 0 and 9 are empty blocks (0x130 bytes, no batch). 0x3D2280 (objects 8, 9's script)
is not read. Strings at 0x5232F8 (DISTANCE, ANGLE, FRICTION, WEIGHT, POWER, ENERGY, DR.BOSKONOVITCH) look like
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
| 2 (gp-0x6574) | 15 | two tracks per actor (2a, 2a + 1): the **hands**, 5 fingers x 3 joints (fighter +0x3074 / +0x3078 >= 50; below) | [H] |

**The hands** (0x3DE0D8, track by 0x364390): track 2a turns hand_A's 15 nodes 39-53 (the nodes under node 38),
2a + 1 hand_B's 64-78. Each triple, 65536 a turn, makes a matrix as a joint's (0x349D70) that replaces the node's
local turn, its bind position kept (0x34A180 blends the rows at weight +0x3080 / +0x307C, 1 in a scene (0x3634F4),
then 0x34A710 squares them up). hand_A takes (-x, -y, z) as a motion's joints do (0x3DD788); hand_B mirrors by joint
(table 0x522CC0): node 64 (x - pi, -y, -z), 65-66 (-x, -y, z), then per finger (x, -y, -z), (x, -y, -z), (-x, -y,
-z). A fighter's own hands (+0x3074 = 0) come from its character's track (0x3672D0, frame +0x306C); 1-49 none.
**The face** (0x3DD918, 0x364298): nodes 7-28 under the head, by table 0x522C60 a mix of turns, scales (0x3DD800,
1/4096) and offsets (0x3DD890, x fighter +0x500 / 40960) from the 33 triples, over a per-character rest table
(0x3B87F8, 12 bytes a node), the eyes aimed through VU0 code (nodes 13, 14) [M]: not exported further.

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
| 5 / 6 | from white (b x 2 to 0 over a frames) / to white over a frames (task 0x3AC5D0, kinds 2 / 3: entry 10 of the full-screen quads 0x43FD10, white; 19 / 20 use entry 9, black) | [H] |
| 17 | a burst 0x3A2758(kind (a >> 1) & 3, at, 0, 0) at actor a & 1's node b (its matrix +0x30, fighter 0x8796C0 + 0xB8A0 x side + 0xDB0 + 0xC0 x node; y, z negated), turned away from the other actor's node c (at +0x10: the difference); b = 0x4002: the matrix 0x444CD0 instead (0x3D2270) | [H] |
| 18 | the dust 0x3A2758(3, at, flag 1, c x 2 pi / 4096) at actor a & 1's node b: spinning c x 2 pi / 4096 a frame (+0x84 into +0x74 each frame, 0x3A06F0); b = 0x62: the actor's place on the floor (fighter +0x510 x, z; y 0), flag 0; 0x4000: twenty objects 0x3ACCD0 (0x3ACEE0) instead; 0x4001: the point gp-struct 0x415808 +0x14 (y - 800), flag 0; 0x4002: as op 17's. With the flag, dust more than 500 mm up is 3 particles at the set's own size and colour 0x808080, not the arena's row | [H] |
| 10 | the character effect 0x3A2BC0(side c & 1, 0, at) at actor c & 1's node b (0x4003: the point 0x5F6280 through the actor's +0x974 matrix, plus +0x988): particles of the side's own set (4 + side, the character's effect file by 0x35C970 -> table 0x519438) [M] | [M] |
| 12 | actor a & 1's texture channel 0 (0x8D5E20, 4 players x 2 x 16 bytes) to frame 1 (b = 0) or 0 (0x3A86D8): Jin's mark (below) | [H] |
| 15 | True Ogre's fire (0x3A6440 -> 0x3A4EE8, character 20 only) from actor a & 1 for b frames (task 0x3AC558) | [H] |
| 16 | 0x378368: table 0x8A7B70[a & 1] = {b, ?, c} (6 bytes; its reader not followed) | [L] |
| 21 | full-screen quad 0x43FD10 entry 9 (black, a 0 / 1) or 10 (white, a 2 / 3) at alpha b / 128 for c frames (0x3AC7C0) | [H] |
| 22 | an iris (0x3AC8B0 -> 0x3CF850(x a, y b, r)): black outside a circle (15-step fans and two bands), r from 640 down by (640 - (c & 0xFF)) / (c >> 8) a frame to c & 0xFF, then held to the scene's end | [H] |
| 23 | an oval (0x3AC978 -> 0x3CFAD8(220, 133, 0xC0)): black outside the ellipse about (320, 224) of half-height 220, half-width 220 x 1.33, for a frames | [H] |
| 24 | a screen fill (0x3ACAB0 -> 0x3AC9D8(a)) for b frames from table 0x437D10 (16 bytes: r, g, b, a, ALPHA's A B C D): 0 (255, 255, 255) and 1 (205, 157, 200) with (Cs - Cd) x Ad (the colour less the picture, the frame's alpha taken as 1 [M]), 2 (162, 51, 50) at 64 / 128 over | [H] |
| 25 | an arena prop (0x3ACB08 -> 0x3D10D0(b & 0x3FFF, matrix)) on arenas 3, 5 and 7 by b, at actor a's node matrix (fighter +0x2A30 or +0x3CF0; flag 0x20 the other actor) for c frames; its matrix 0x444CD0 is ops 17 / 18's node 0x4002 [M] | [M] |
| 26 / 31 | gp-0x6E84 = a & 1 (cleared with the arena's people, 0x390E74); 0x34CB38(0 / 1) by bits of a, its result unused: nothing drawn | [H] |
| 27 | 0x3465E0: gp-0x6420 \|= 3 (also op 4's: a camera cut) | [H] |
| 28 | actor a & 1's +0xB052 = b when it is Kuma or Panda (characters 20 / 29 at +0x2A): the face state 0x363330 reads | [M] |
| 29 / 30 / 32 | 0x3AA4B8 for a frames; every other frame for a frames an object 0x3ACF50 (0x3AD1D8); gp-0x6E78 = 1 for a frames (0x3AA410, read by 0x3ABD1C) | [L] |
| 7-9, 11, 13, 14 | no-op | [H] |

**Texture channels** (0x3A82C8 at a fighter's set-up, 0x3A80E8 each frame; records 0x8D5E20, 2 a player: +0 state,
+4 frame << 8, +8 speed, +0xC frames): a fighter pack's part 1 holds channel k's frames at [0][k][f] (an image and its
palette); the model part's table after its nodes (u32 count, then at its +0x14 16 bytes `{s16 channel, u16 dbp, x, y,
w, h, dbw, psm}`) says where a frame goes, from the fighter's first texture block. Only Jin's packs have one: channel 0,
his face with the mark (frame 0, the face the textures load with) and without (frame 1). His set-up shows frame 1
(0x3A85E4, character 9; not on screen 5 when 0x43B800 is set); in a fight the mark shows while the move's +0x24 bit 17
is set (0x370E20), in a scene by op 12. The converter bakes frame 1 into his textures. [H]

Typical: ops 0-4 at frame 0, fade in over 45, effects, fade out over 60-120, end at camera frames - 1. No op plays
sound: the only audio is the music [M]. The background characters are a set of 0x114FC8 by the scene, not
the arena's (0x3D85D8, the people set gp-0x6170): ending e takes case e - 2 of jump table 0x521C30: 2 -> 23, 4 -> 27,
7 -> 22, 9 / 16 / 24 -> 29, 10 -> 25, 12 -> 13, 15 -> 30, 23 -> 28, 30 -> 26, 33 -> 24, 36 -> 21; every other
ending shows none. On the EMBU's screen arena 13 takes set 30, any other none. Sets 23-30 use the arena's models and
motions (their motion table is an arena's); arenas 7 and 21 still draw each person with the next model. [H]

### 7.5 Playback and staff roll (0x3AA8A8, table 0x51B7B0) [H unless marked]

Arcade clear: pick and mark the ending. **Unknown** plays movie 2 (file 16; table 0x518EA8: 0 = file 14 opening,
1 = file 15 arcade), then the staff roll on BGM 6. Everyone else: load arena, cast, camera, pack, BGM 6 (STAFF
ROLL), then the staff roll runs 4620 frames (77 s) first; START skips it, the music jumps to BGM 7 (file 20, 0xCF7
sectors in), and the scene plays (START skips, 4-frame fade); the game then waits for the staff roll to end or
START [M on credits over the scene].

Staff roll 0x3AC018 (12 states, table 0x51DCB0, record 0x8D7DE0): state 1 fades from black over 171 frames
(0x3A67D0(0xAB, 0, 0, 0)); states 2-4 show the card "TEKKEN TAG TOURNAMENT" (0x3ABEC0), 5-7 "NAMCO PRODUCTION STAFF"
(0x3ABF18), 171 frames a state (a 170 timer to -1): each card in the chrome font (`%F` 10) at size 5, colour 11,
centred on 640, at y 200; state 3's end fades to black over 171 (0x3A6770(0xAB)), state 4's from black over 171,
state 6's to black over 172 (0xAC). There are no logos: 0xAB / 0xAC are the fades' lengths. State 8 (frame 1027)
starts list 0x51B880 (434 `{u32 kind, char *text, u32 font}`: 0 heading (font 7), 1
name (5), 2 blank, 3 centred final page, 4 last lines; flag 0x10000 = alternative: "SECRET MODE PROGRAMMER" until
Tekken Bowl is unlocked, then "TEKKEN BOWL PROGRAMMER"); each line `%F%S%H%V%c%s` (0x51DC98) at size 4, its `font` the `%c` colour (7 headings, 5 names, 8 notes), x by kind
(0x3ABF70: 0 at 48, 1 at 304, 2 ending at 608, 3 and 4 centred; so kind 2 is a right-aligned line, often empty);
scroll 1.5 px a frame (y << 8 less 0x180) from y 480, freed above 0, up to 64 on screen; a new line when a timer of
0x180 a frame passes 32 px, then from 0 (22 frames: 33 px apart); a negative `font` is the end mark: 139 frames more
(state 10, to 0xD000), then the lines stand for 210 frames (state 11) and gp-0x6E56 = 1. It ends "TM AND (C)1994 1995 1996 1999 / NAMCO LTD. / ALL RIGHTS RESERVED".

### 7.6 The EMBU (scene 1, 0x3A97B0; state machine 0x3A9AB8, table 0x519B30) [H unless marked]

Reached as screen 11 (gp-0x74D0; jump table 0x4FB970 -> 0x3BB310, gp-0x74CE its state) by 0x3BB4C0(a): gp-0x62BE
= a. The title's attract (0x3BCC88, state 0 at 0x3BCD14) takes a variant from gp-0x6BE8 +5 (stored to +4; +5 counts
0, 1, 2 and wraps): 0 runs 0x3C9358(1) then 0x3BB4C0(1), 1 0x3C9358(0), 2 0x3C9358(2). Variant 0 is the EMBU: in
PCSX2, with 0 kept in +5, every attract went title -> screen 11 (state 2, scene 1) for about 2290 frames, then the
title; no unlock is needed. The EMBU's own frame counter is gp-0x63CE (s16, 0x3A9E48 compares the script's frames to
it; not the endings' gp-0x63B4). An earlier note here said variant 0 played a demo fight: it read the wrong counter. The menu's entry for screen 11 is looked up in the main menu's list 0x51EFE0 (8
bytes an entry, +4 the screen; 0x3BD240); state 0 treats gp-0x62BE 0xD apart (0x3BB35C) [M].

Set-up 0x3A97B0: gp-0x6E94 counts the plays; once config +0x37 is set (0x3BFDC8) odd plays take cast 1, else cast
0 (gp-0x6E98 = 0x437CE0 + 0x18 x cast); the arena gp-0x6E90 is 4 for cast 0, 13 for cast 1 (stored to gp-0x743A);
scene 1: camera file 41 (0x392E00(0): `u32 count` 22, `u32 offsets[22]`, each an ending's path), motion pack file 42
(30 body tracks, 30 face, 60 hands). States: 0 (0x3A9B34) clears the two slot records (0x8D78B0, 12 bytes: +0 state,
+6 track, +8 last frame, +0xA frame) and points the script gp-0x63D4 at **0x519B90**; 1-2 fade to white over 44
frames; 3 (0x3A9E48) runs the script, then 0x3A9898; 4 the end (op -1, START, or 7200 frames).

The script: 81 records `{s16 frame, op, a, b, c}`, ops by table 0x519B50:

| Op | Does |
|---|---|
| 0 / 1 | cast member a (0-5; past it 0) into slot 0 / 1 (0x35C8D0), its record cleared (state 0: not drawn) |
| 14 | the same for slot 0, its record kept (a member swapped in mid-track) |
| 2 / 3 | slot 0 / 1 plays track a from frame b to c (state 2); 0x3A9898 shows frame +0xA, then +1, and past c the slot goes back to state 0 and is not drawn (+0x127 = 0) [M on +0x127] |
| 4 | camera path a from frame b (c 999 unused); the path's frame +1 a frame (0x392F80) |
| 5 / 6 | from / to white over a frames (0x3A67D0 / 0x3A6770) |
| 7 | arena 4 only: 0x3A7FA8(a, b) [L: its lightning] |
| 8, 9, 11 | 0x363628(a); 0x3A6BC0(); 0x3A68D0(a, white) [L] |
| 12 | slot a & 1's Jin: the mark off (b = 0, 0x3A88A0) or on (0x3A8890) |
| 10 | an effect at slot a & 1's joint b (0x3A2BC0) [M] |
| 13 | the music: 0x39A0C0(1), BGM 1 (EMBU) |

It runs 2289 frames: white fades in over 128 while camera path 0 shows the empty arena (frames 0-487); then the
two actors' katas on 22 camera paths; white out over 106 frames from 2184. Exported as endings.json `embu`.

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

1. Ending effect ops 10, 16, 25, 28-30, 32 (the props, the character effects, Kuma's face), op 15's fire.
2. The face part's joint mapping (33 triples; section 7.3's outline).
3. What a spectator the ball hits plays (0x3DB860).
4. Whether the staff roll draws over the ending scene (its texts are 7.5's); the flash / fade colours (table 0x43FD10, 9 and 10).
5. What 0x3E4BC0 draws in the throw (a panel by the bowler's motion).
6. The black band behind the pin deck: the object part's models 0 and 9 are empty blocks (0x130 bytes, no batch),
   no fill; how the PS2 shows that place is to be compared.
7. File 128 parts 1-2 and file 127 (the bowl's camera path, 600 frames) use.

## Scripts (`docs/re/`)

`dump_bowl.py` (section 4's table, the strike poses, the pin rack and the physics constants), `vdis.py`
(`python3 vdis.py 3EC768`: like `fn.py`, with the EE's VU0 macro instructions, `lq` / `sq` and `sqrt.s` decoded),
`dump_endings.py` (`python3 dump_endings.py SLUS_200.01 [RAW_DIR]`: the ending table, the 38 descriptors with
casts, each script as text and the scene files' frame counts).

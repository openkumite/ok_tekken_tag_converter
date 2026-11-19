# Tekken Tag Tournament (PS2, USA v2.00): what a fight draws besides the fighters and the arena

Read in the executable (memory = file offset + 0x33F000) on 2026-11-19. Confidence: **[H]** read in code and data,
**[M]** read in code, the meaning inferred, **[L]** a guess. "Exported" says what the pack gives OpenKumite
(GAMEDATA.md).

## 1. The frame's draw order (0x3577F0)

0x357A50 (0x35DB90), 0x357C88: the arena (0x3DE6C0, arena gp-0x743A), 0x3D11D8, **0x340E28 the fighters' shadows**
(below), 0x3DB758, 0x3621F8, 0x3A6C80, 0x341F68(0x433440, 0x433400), 0x3DB7B0; 0x357A78: each fighter (0x357640
its lights, 0x3A8090 its face textures, 0x343340 pass 0); 0x357D38: arenas 2, 0x13, 0x16 add 0x3E7270(0x4334C0)
[L: a second floor pass]; 0x357DB8 (0x35DE68, 0x399918); 0x357DD8: the fighters' pass 1 (blended cloth and hair).

## 2. Fighter shadows (0x340E28 -> 0x342818, 0x342128, 0x342988) [H]

A projected-texture shadow, one per fighter, from fighter-pack **part 8** (FORMATS.md section 11's "low-detail
model": 18 joints in the arcade slot order, two-matrix vertices; nothing else draws it):

- 0x342390 points the frame at a 256 x 256 PSMCT16S picture at FBP 0x1CA (packet 0x46B180) and clears it.
- 0x3424A8(fighter, offset) draws part 8 there with the fighter's joint matrices (fighter +0xDB0 + 0xC0 x slot),
  through a view along the fighter's light: 0x353C88(side) = the side's light 0 direction (block 0x1FC4290 +
  0x120 x side, +0x10), turned about y in mode 4. 0x342818 draws the fighter, then the opponent too when its
  squared distance is under gp-0x78F8 (it shadows the same picture).
- 0x342400 sends packet 0x46B240: TEXA with TA1 = **table 0x46B280[arena]** (u16, 23 arenas; set by 0x342360,
  0x40 past the table): the shadow's alpha (of 0x80) where the picture holds the silhouette, 0 elsewhere.
- 0x342128 makes the projection matrix 0x1FC49F0 (that view, scale 1 / gp-0x67A4, + 0.5); the floor and the
  objects flagged +0x148 & 1 / 2 (0x3422C0) are drawn again with it: the silhouette falls along the light onto
  them. That pass (0x356C28, pass 5) sets ALPHA **0x46** (Cv = Cd - Cd x As) and TEX0 0x0E_20A13940 (the picture,
  TFX decal, its alpha): the floor is **multiplied** by 1 - TA1 / 0x80 under the silhouette (arena 0: x 0.625).
  Arena 14 (UNKNOWN) takes 0x342A38 instead (not read).

The shadow's length is not limited [H]: 0x34A5D0 makes the view's axes from the light alone (normalised; across
it, `(z, 0, -x)` normalised), 0x342128 scales them by 1 / 5800 and adds 0.5: an orthographic projection with no
clamp on the light's height, the shadow's length or a fade (none in 0x340E28's callers either). A floor point
takes the silhouette's texel wherever its line along the light meets the fighter, so a low light (arena 10, Eddy-A:
y 0.073) lays a shadow some 13 times the fighter's height across the floor, bounded only by the picture's 5.8 m
square in the light's axes (CLAMP). In a fight (gp-0x74D0 = 4) 0x353C88 turns the light about y by the camera's
turn (0x366EC8) [M]; its height stays.

Alpha (x / 0x80) by arena 0-22: 30 48 30 40 40 40 50 40 40 40 48 40 40 48 20 40 40 40 40 20 40 40 40.

Lights (the shadow's direction, and the fighters' light): the arena's table **0x416288 + 0x54 x arena**: three
directions (towards the light, y up, the arenas' axes), three colours (0-255, 128 = 1) and the ambient, floats [H];
0x363630 points gp-0x72E8 / -0x72E4 / -0x72E0 at its directions, colours and ambient (blending in table 0x4161E0
while gp-0x72EC counts down). 0x353878(side) fills the side's light block 0x1FC4290 + 0x120 x side: +0x10, +0x20,
+0x30 the directions, +0x40, +0x50, +0x60 the colours times gp-0x60C8 (1.0), +0x70 the ambient; 0x353758 lights 0
and 1 again, x and z negated while flag gp-0x60B8[side] is set [H]. In a fight (gp-0x74D0 not 5) 0x353448 instead
turns light 2 towards a place (0x415808 +0x14..+0x1C, ints) and blends it and its colour (0x4550C0[player +0x2A], grey)
with the base copy at +0xB0 / +0xE0 by a distance weight (0x352F50 with 0x4550B0, 0x4550B4) when 0x4550B8 is set
[M]. The fighter's draw (0x357640) sends the block to VU1; that it lights as ambient + each colour x max(0, n.L),
the GS modulating the texture by it (128 = 1, saturating at 255), is read in the data, not in the microcode [M].

Exported: `gamedata/shadows.json` (part 8 as the Tekken 3 converter's shadow meshes), `gamedata/stages.json` (each
arena's shadow: light 0's direction, the alpha; since format 13 its `lights`, the table as it stands). Not: the
shadow on objects, light 2's turn towards a place, the negated lights, the
picture's 256-texel softness.

## 3. Effect files 4-9 (loaded at boot, 0x3B96C0; 0x345410 keeps their parts) [H]

| File | Part 0: picture (texture slot) | Part 1: particle paths | Set |
|---|---|---|---|
| 4 | red-orange glow, 64 x 64 (slot 0) | 61 x 756 bytes, 35 frames | 0 |
| 5 | violet sparkle, 128 x 128 (slot 1) | 25 x 656 bytes, 30 frames | 3 |
| 6 | white-blue sparkle, 128 x 128 (slot 2) | 45 x 856 bytes, 40 frames | 1 |
| 7 | grey-green smoke, 64 x 64 (slot 3) | 45 x 3056 bytes, 150 frames | 2 |
| 8 | 20 fireball frames, 32 x 32: 10 orange, 10 blue (slot 4) | - | - |
| 9 | two soft grey discs, round and hexagonal (slot 5) | - | - |

A path: `u32 frames`, a 100 x 100 mm quad (four corners), u16 x 2, then from +0x38 20 bytes a frame: position (mm,
from where the effect starts), size and fade [H: the particle task 0x3A05F8 copies them to +0x60..+0x68 times the
row's scale, +0x54, +0x58; the draw 0x345A00 sizes the quad 0x90 x size / 0x48 and sets the brightness 0x44 x
(1 - fade)]. Rows of table 0x436688 (0x343F38): 0x24 the hit spark {scale 0.5, 0x44 128, 0x48 400}, 0x25 the guard
{1.0, **32**, 400}, 0x26 kind 1 {1.0, 128, 400}: the guard's burst is a quarter as bright. Sets: table 0x8BF350 `{u32 paths, u16 count}` (0x343F20(k)); texture slots: 0x8BF3A0 + 0x18 x
slot (0x343F08). Effect objects are tasks (0x3464F8 / 0x345780 spawn, 0x3465C8 end; +0x1A texture slot).

## 4. Hit, guard and move effects [H unless marked]

- **Every hit** (0x35C0D8 from the hit code 0x372408 / 0x3726F0): 0x35BFF8(0, at) = effect kind 0 (set 0, file 4).
  An attack from bone 0x18 (fire) adds big effect 0x0A instead of the rest. Damage >= 21: the attacker's own effect
  0x35BF70(side) -> 0x3A2BC0(side) [M], and big effects 0x81 + 0x80 (normal) or 0x8C + 0x8B (attacker +0x16A or
  +0x400 set: the charged / angry attack), 0x3A6110: file 8's fireball frames (slot 4), orange or blue. Tekken
  3's rule, as OpenKumite's `vfx.rs`.
- **Guard** (chip damage the victim survives, 0x3723E0): kind 2 (set 1, file 6).
- **Move effects** (timed properties, 0x372C40, jump table 0x500620): op 6 / 7 the own / other side's effect at
  a bone, op 8 kind 0, op 9 kind 1 (set 3, file 5), op 0xA kind 3 (set 2, file 7: dust, coloured by arena table
  0x437AC0, 12 bytes an arena), op 0xB kind 2. As Tekken 3's ops 6-0xA.
- Landing / ground impact (0x35C240 from 0x36DAF8, 0x36DB50, 0x372DF8): kind 3 at the feet.
- 0x3A2758(kind, at, flag, f12) picks the set (kind 0 -> 0, 1 -> 3, 2 -> 1, 3 -> 2) and spawns its particles [H]:
  one on each of the set's paths (its count, +4), with row 0x24 / 0x25 / 0x26 (kinds 0, 2, 1); set 2 (the dust)
  instead on paths drawn at random (rand() % count), as many as the arena's row of table **0x437AC0** says (12
  bytes: u32 particles, f32 scale, the size's divisor 400 / scale at +0x48, RGB at +0x50, 128 = 1; 3 particles, and
  the row's colour only below 500 mm, when flag is set). Each particle's +0x74 is atan2(at +0x10, -(at +0x14)), the
  strike's heading, growing by f12 a frame (0x3A05F8); the draw (0x345A00; the task's defaults 0x3456B0: +0x44 128, +0x48 400, colour 0x80) turns the particle's path offset (+0x60)
  about y by it (0x34ACB8: rows (cos, 0, -sin), (0, 1, 0), (sin, 0, cos)) before placing it: the path's +z runs along
  the strike. The ending scripts' op (0x3AB504) passes the bone's position and the vector from the other bone.
  The blend (0x345D3C, ALPHA at 0x70000308): +0x40 = 1 Cs x FIX + Cd, FIX (1 - fade) x +0x44 (128 = 1), added;
  another non-zero value Cd - Cs x FIX; 0 (the dust's) (Cs - Cd) x As + Cd, As the texture's alpha times the vertex
  alpha (1 - fade) x +0x44; the colour +0x50.
- The **SPECIAL EFFECTS** option (OFF: "Cancels Explosion Effects") [L: gates the 0x3A6110 explosions].
- **Ogre's fire** and the other projectile attacks (0x3A6390 each frame from 0x36D99C): attack bone 0x18 with body
  20 (True Ogre) in the active frames -> 0x3A4EE8; bone 0x19 -> 0x3A5810 [M, not read].

Exported: `pictures/effects/` (atlas.png, effects.json in the Tekken 3 converter's layout): sets 0-3's pictures
and file 8's frames as OpenKumite's spark, zero, block, dust and heavy strips, and `bursts`: each set's paths (the
row's scale applied), its brightness and quad size. Since format 13 stages.json
has each arena's `dust`, and the dust's paths (not the guard's) are drawn at random. Not: the character effect
(0x3A2BC0); what the hit code puts in at +0x10..+0x14 (OpenKumite turns the burst with the strike) [M].

## 5. Arena effects

0x3A2640 by arena (jump table 0x519770, arenas 4-20) starts the arena's effect tasks [H]: 4 Hwoarang and 15 Law-B
0x3A0A90; 5 Lei 0x3A0CE8; 8 Jin-A 0x3A1068 (+0x1A 0), 17 Jin-B and 20 the bowl 0x3A1068 (+0x1A 1); 9, 18 Nina 0x3A1378;
11 King 0x3A1510 and 0x3A1378; 14 Unknown 0x3A1B48 (twenty objects 0x3A16C0 and thirty 0x3A18F8); and 0x3A1C78 on
every arena.

**Their pictures and paths** [H]: the arena's geometry pack part 1 (0x367938 keeps it at 0x8795E4; empty on most
arenas) is a pack of up to three entries, each a pack of a texture pack and a pack of particle paths (as files 4-7's
part 1); 0x345070 (from the arena's load, 0x367BD4) puts entry k's picture in texture slot 6 + 2k (table 0x8BF3A0 +
0x18 x slot) and its paths in set 6 + k (0x8BF350 + 8 x set). Hwoarang and Law-B: a grey puff, 16 paths of 300
frames; Lei: a faint grey haze (alpha 0), 16 of 800; Jin-A, Jin-B, the bowl: a grey puff, 16 of 400; Nina: a red
glow; King: the red glow, a puff with 19 paths of 800 (entry 1), a white light (entry 2); Unknown: a grey puff.
Eddy's and Yoshimitsu's entries (a picture, no paths) are not used by these tasks.

**The tasks** [H; a particle's fields: 0x3456B0 makes it, 0x345A00 draws it (EFFECTS.md 3), +0x40 the blend: 0
ALPHA 0x44 with the vertex alpha the brightness x (1 - the fade), 1 Cs x FIX + Cd with FIX that; the vertex colour
+0x50..0x52]. Each counts its frames (not while paused) from its start; its first frame starts nothing:
- **0x3A0A90** (Hwoarang, Law-B: smoke): six rows (0x5194E0 / 0x5195A0, 32 bytes: `{x, y, z, 1, u32 every}`), each
  every so many frames a particle 0x3A08D8 on a path of set 6 drawn at random (at most 15), the path 4 times its
  size, 0x48 450, colour (160, 160, 170), blended; 0x3A08D8 moves it by the path's step and then by the wind at it
  (0x3D3D08) times rand01 x 6 (arena 4) or 25 (+0x120).
- **The wind** [H] (0x3D3630 at the arena's load from table 0x447EC0, 12 bytes an arena: `u32 waves, *waves, u32`;
  0x3D37E0 each frame): up to three waves (32 bytes: `i32 step, f32 speed, f32 strength, u32, f32 direction[4]`),
  each a Lorenz series (0x3D42F8: Euler, dt gp-0x7D30 0.01, sigma 10, rho 28, beta gp-0x7D2C 8/3, from (1, 1, 1);
  the value (x + 40) / 80) kept 1024 frames back (0x3D4178: `step` values a frame, or one every -step frames and the
  others drawn on from the last two; 1024 run at the start). A point (0x3D3F48) takes the value d / speed frames on
  from the oldest, d its distance along the wave from the line across it through (sqrt(g / (x^2 / z^2 + 1)), 0, z /
  x of that), g gp-0x7D4C (1e8): the wind blows the series across the arena at its speed; times the strength along
  the direction. (Every arena has waves; only these particles read them here. 0x3D3B80's four impulses, 0x450000 -
  0x4F00, are not read.)
- **0x3A0CE8** (Lei: steam): three rows (0x519660, 32 bytes: `{x, y, z, 1, heading}`), every 34 frames a particle
  0x3A04B8 from each, the path turned by the heading (+0x74) and 1.2 times as long along x (gp-0x7F44), 0x48 400 x
  1.43 (gp-0x7F40), added; on its first frame each row's stream as it would stand a path's length on (path 0's
  frames / 34 particles, k 34 x k frames along).
- **0x3A1068** (Jin-A; Jin-B and the bowl with +0x1A 1): every 32 frames a particle 0x3A04B8 from each of three rows
  (0x436A30 / 0x436AC0, 48 bytes: `{x, y, z, 1}`, the scale, colour u8 x 4, brightness, 0x48), turned pi / 2,
  blended (Jin-A's blue (106, 118, 255) at 128 and 246; Jin-B's grey at 230).
- **0x3A1510** (King: smoke): every 32 frames one particle of set 7 (slot 8) from (-2680, -4240, -20240) mm
  (gp-0x7F24..), scaled (20, 7.29, 12.79), 0x48 140, blended.
- **0x3A1B48** (Unknown: wisps): twenty particles 0x3A16C0 and thirty 0x3A18F8 (slot 6, colour (64, 64, 128),
  blended): particle k from angle k x 2pi / 20 (30) at radius 12000 + rand x 5000 (6800 + rand x 2000) mm, x sin, z
  cos of its angle, turning (2 rand - 1) x 0.002 +- 0.001 a frame; height rand x 3500 (1340 + rand x 500) + sin(f) x
  3000 (500), f from rand x 2pi growing rand x 0.01 a frame; fade (sin(g) + 1) / 2, g from rand x pi growing a
  constant rand x 0.01; size 10 + 8 fade (5 + 3 fade).
- 0x3A04B8 places its particle at the row + the path's frame (turned by +0x74, scaled) and steps through the slot's
  cells while its path's +0x34 is set (none here); gone at the path's end.

Exported (format 26): stages.json `weather` (GAMEDATA.md), the pictures in the effects' atlas from cell 24. Not:
the particles' depth bias (+0x4C), the offscreen count that ends one (+0x96), the paused frames.

**The lens flare** (0x3A1C78) [H]: on the arenas its jump table 0x519730 (by arena - 1, 16) names: 1 Law-A, 7
School-A, 10 Eddy-A, 11 King, 12 Heihachi, 13 Eddy-B, 16 School-B, each with a sun (0x437A50 + 0x10 x row: x, y, z,
1, mm, the arena's axes) and a strength (gp-0x7EB8 + 4 x row: 0.6, 0.3, 0.6, 0.6, 0.6, 0.7, 0.6). It makes nine
screen-space objects 0x3A1BC8 (file 9's discs, slot 5, flag 0x80: placed in GS units, ALPHA (Cs - 0) x FIX + Cd,
FIX the brightness) and each frame: the sun projected (0x34B2A0; none behind the eye); (dx, dy) the screen's centre
(0x8000, 0x8000) less it, d = 1.5 x |(dx, dy)|; none when d > 8000; the level 1, or (8000 - d) / 2000 from 6000
(gp-0x7E9C); the size x 1 + (2000 - |dy|) x 0.00025 (gp-0x7E98) within 2000 of the centre's height. Each fighter's
two points (player +0x590, +0x11A0, not read) in the axes of a view from the eye towards the sun: the least x^2 + y^2
below 250000 (500 mm, gp-0x7E90) dims the level by (it / 250000)^2 (gp-0x7E8C). Disc k (0x5196C0 + 12 k: u32
picture, the word before its size; f32 size; f32 brightness: hexagon 0.1 192, hexagon 0.3 170, round 0.5 148, round
1.3 100, hexagon 0.6 132, hexagon 0.5 148, round 1.8 96, hexagon 0.3 170, round 0.8 128) lies at the centre + (k - 4)
x 0.1875 x (dx, dy), half 0x90 (0x400) x size GS units across (0x345A00: a sixteenth of a pixel), its FIX the
brightness x the level x the arena's strength. Exported: stages.json `flare`, `pictures/effects/flare_0.png` (round),
`flare_1.png` (hexagon), opaque.

**The beacons** (0x3A1378) [H]: Nina's arenas (table 0x436B50) and King's (0x436EE0), 48 bytes a row up to one
whose +0x20 < 0 (Nina 18, King 60): u16 kind, +0x10 the place (m), +0x20 the period (frames), +0x24 the size. Each a
3D object 0x3A1298 of texture slot 6 (kinds 0, 1: brightness 160) or 10 (kind 2: 240), added, +0x4C 10 (a depth bias,
not a cell); its fade (cos(phase) + 1) / 2, the phase growing 2 pi / period a frame from pi (kind 1, and a period of
0: always lit) or rand x pi (kind 0). Slots 6 and 10: the arena's entries 0 and 2 (above). Exported (format 26):
stages.json `weather.beacons`.
The arenas' additive strips (ALPHA 0x48, Cs x As + Cd: lights, neon, floor gloss) and darkening strips (0x89,
Cd x As: painted shadows, dirt) are in the stages since format 14, in materials named `additive` and `shade`
(GAMEDATA.md).

## 6. Not found or not read

Fog / depth cue, screen flashes and the K.O. look, the tag-in / tag-out effects, ki-charge glow, motion blur:
not located. The ending scripts' effect ops (BOWL_ENDINGS.md 7.4) use the same tasks.

## 7. Arena objects (0x3D0E18, table 0x520FC0) [H unless marked]

The row of arena n (16 bytes): `{*table, u32, *update, *init}`; the table `{u32 links, *links, u32 objects, *objects}`;
a link `{s16 parent, s16 child}`; an object (64 bytes): `s16 model` (a block of geometry part 3), +2 the view
(projection 0x433480 or 0x433440: near and far planes only), +6 the draw bucket, +8 the most children, +0xA / +0xC
its int / float parameters, +0x10 the turn (radians), +0x20 the place (mm), +0x30 the routine run as it is drawn,
+0x34 the one run once made. 0x3D1920 sets a runtime object's defaults (drawn, turn order z-y-x); 0x3D1468 draws
`parent x T x Rx x Ry x Rz` (0x34AD60), `Rz x Ry x Rx` when its +8 is set. Only what the row's update draws
(0x3D10D0, 0x3D1160) is seen:

| Arena | Update | Drawn and how it moves | Exported |
|---|---|---|---|
| 0 Paul | 0x3D5798 | the train (0-3, four 18 m cars): from (0, 14000, 200000) turned 5-10 degrees, z negated (0x3D56F0), each frame headed along its circle and moved 278 mm; back to a new start after 1800-2400 frames | the middle start, 2100 frames [M: not seen on screen yet] |
| 1 Law-A | 0x3D6FD8 | lights 4-13 in frames of their own x/z scale (0.77-0.96, y 1.1), 14-18 swayed on a curve (0x3D7C38); a flock of 40 copies of 0-3 on splines with their shadows (19) | 4-18 standing; not the sway nor the flock |
| 3 Xiaoyu | 0x3D52B0 | the carousel 0 turning 0.25 degree a frame (0x3D54F8), its horses 1-8 rising and falling 300 mm (phase 0.03 a frame, the second four half a turn behind), lamps 9-12 turned to face the camera (0x3D5640), 13, 14 | all; the lamps facing the camera (since format 22) |
| 9, 18 Nina | 0x3D5E80 | the helicopter 1 on its pad, 100 mm up and down (0.0316 a frame), its rotors (2: y, 3: x, 1.03 a frame); after 600 frames out of sight it takes off and comes back (states 2, 3); 40 copies of 0, 8, 9 in 10 lanes moving 15-30 mm a frame, wrapped about the camera | the helicopter waiting and its rotors; not the take-off nor the lanes |
| 11 King | 0x3D4B18 | three helicopters (12, 15, 18 under frame 11) round an ellipse of 100 x 80 m, 0.1 degree a frame, the wingmen turned to their own bearing (0x3D5190), rotors 1.03 a frame; ten groups of four of objects 1-10 drifting with a fade (not read) | the helicopters from angle 0; not the groups |
| 14 Unknown | 0x3D2168 | 0 and 1 standing | both |
| 5, 7, 8 | 0x3D2160 (nothing) | none drawn | none |
| 20 Tekken Bowl | 0x3D21B8 | object 10 (the bowl draws the rest) | the bowl's own files |

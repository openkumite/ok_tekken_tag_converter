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

**Yoshimitsu's snow** [H] (arenas 2, 19, 22; not a task): the arena's load (0x367A84) builds a VU1 packet
(0x3E6DA0, gp-0x6528) and 0x357D38 sends it each frame after the arena (0x3E7270, view-projection 0x4334C0, texture
slot 6: entry 0's picture). Two layers of flakes, 8 a batch (0x3E6B10: x and z (rand - 0.5) x the box, y 12288 or
6144 mm, a move of (r cos a, -385 / 16, r sin a) mm a frame, a from -pi to pi and r 0-12 at random, a first frame
rand & 0x1FF or & 0xFF): arena 19 0x82 batches of a 35 m box and 0x46 of a 6 m one, arenas 2 and 22 5 and 2. Each
frame the eye's heading (0x415808: +4 the yaw, +0x14 / +0x1C the eye) puts each box's corner (gp-0x6078, gp-0x6074)
half a box past a point 30 m (6 m) ahead (gp-0x77C4, gp-0x77C0). VU1 program 0x708 (MPG at 0x4FA38C) moves a flake's
start by whole boxes to within the box, adds its move times (its first frame + the count gp-0x6084) & 0x1FF (0xFF),
and draws it at four places 7 m (3 m) aside in +x, +z, -x, -z (no flake at its own place): a screen square of half
24000 / w GS pixels (0.0312 m at 768.75 pixels a unit at w), colour 128, alpha 0x60 - 9.6e-5 x w (w the depth, mm;
gp-0x7808), ALPHA 0x44, TEST without z writes. Checked in a PCSX2 arena-19 fight: the boxes' corners against the eye
and the yaw. Exported (format 61): stages.json `weather.snow`.

**Their size** [H]: 0x345C10 makes a particle's half 0x90 x size x z / 4 / 0x48 GS units (a sixteenth of a pixel), z
its depth: the arena effects (flag 4) through the arena's view 0x879420 (0x346A24, projection 0x433480: z =
12586753 / w - 64), the others through 0x433440 x the view (0x346A40: z = 16782424 / w - 85). At the screen's 768.75
pixels a unit at w, an arena effect's quad is 0x90 / 0x48 x size x 0.5117 m across (0.6822 m for the others: the
`bursts` since format 60, 1.47 times smaller than before; a PCSX2 hit's sparks are as small).
Lei's steam (0x90 0x400, 0x48 572) drawn 1.95 times too wide had covered the street in haze (format 58).

Exported (format 28): stages.json `weather` (GAMEDATA.md), the pictures in the effects' atlas from cell 24. Not:
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
0: always lit) or rand x pi (kind 0). Slots 6 and 10: the arena's entries 0 and 2 (above). Exported (format 28):
stages.json `weather.beacons`.
The arenas' additive strips (ALPHA 0x48, Cs x As + Cd: lights, neon, floor gloss) and darkening strips (0x89,
Cd x As: painted shadows, dirt) are in the stages since format 14, in materials named `additive` and `shade`
(GAMEDATA.md).

## 5b. Devil's beam and the character effects (read 2026-11-21; exported, format 65)

- **Devil's beam** [H unless marked]: in a fight 0x378150 (each frame, Devil only: character 0x1D at fighter +0x2A,
  drawn +0x127): while the move (+0xC0) has attack bone 0x18 in one of its slots +0x28..+0x2B and its frame +0xC4 is
  within +0x2C..+0x2D, state 0 picks the beam's type by the move (0x368248 of entries 0x104F -> 1, 0x1051 and 0x1053
  -> 2, 0x1A48 -> 3, 0x1054 -> 4); a type starts it (0x3780C8: the record 0x8A7AD0 + 0x50 x side cleared, start and
  end at fighter +0x708), plays sound 0x45 and 0x3A8C50 (18 random jitters 0..7 at 0x8D7820 + 0x48 x side, and a point
  light at the head matrix +0x1BF0 x 0x437CD0, 0x3531F0). State 1 runs the type's routine (table 0x506210: 0x378810,
  0x378A28, 0x378C48, 0x378E70). Type 1: heading +0x3C, the beam's step 200 mm a frame along it (0x357F18 / 0x357F58,
  sin / cos x 200 >> 15) times the frames since the active start; its start offset by table 0x506248 (0x50624C when
  the costume table's value is 0x6B): `s16 forward, s16 height`; 0x378500 writes start (+0x38) and end (+0x44), and
  the hit line fighter +0x190..+0x1A4; an end below the floor is put on it (y 0) and every 8 frames 0x379090 throws a
  spark there. Ending op 16 (0x378368 -> 0x378398) sets table 0x8A7B70[side] = {type b, count, frames c}: the same
  routines with the scene's count and the heading from 0x378760 (the head's two points 0x42F120 / 0x42F130), for c
  frames.
- **Its draw** (0x378600 -> 0x3A8D08): width 0x64 x t + 0x32 (type 0) or 0x82 x t + 0x46, t fading after the active
  end (+0x1E frames); colour gp-0x7198 (3 bytes) x t. For the first 15 frames a flare at the eye (0x3A8BC0, slot 0x14
  / 0x17 by side, colour 0x5F6250). The line from start to end is cut into |end - start| / 500 segments (at most 36),
  each projected (0x34B2A0) and drawn as a screen-space quad (0x3A8BC0, queue 0x8D6AA0, 36 entries of 0x60) of half
  width 7200 x width / w across the segment's screen direction, its texture slot 0x14 + 3 x side + 2 (the first) or
  + 1 (the others), its cell the segment's jitter % 7 (the jitter + 1 each frame). Skipped off screen (0x6C00..0x9400,
  0x7200..0x8E00) or when w > 0xFDE8.
- **The character effects** (0x3A2BC0(side, 0, at)): the model's effect index (0x35C970: model table 0x415840, the
  record's byte 8) selects 0x347730's row (0x51B3B8 + 8 x index, 36 rows: `u32 the particle routine, light`); a
  particle on each path of set 4 + side, texture slot 14 + 3 x side + the path's +0x36, its +0x34 cells. Characters
  0x13, 0x1D, 0x0F, 0x21 on their moves 0x25 / 0x26 take set 9 instead (0x3A2E20; 0x344BE0 points set 9 and slot 12
  at True Ogre's or Devil's own part, 0x8BF2C0 / 0x8BF308). **Where they come from** [H]: the fighter's set-up 0x367600 keeps its
  pack's parts in record 0x8794C0 + 0x2C x n (n the loaded fighter, fighter +0x22; 0x367250 reads part 0) and calls
  0x344EA8: fighter-pack **part 0** is a pack of its pictures (texture slots, each a pack of frames: one GS upload,
  image and palette, a frame) and its particle paths (section 3's; `+0x34` 1 when the particle's frame runs, `+0x36`
  its slot); True Ogre's, Devil's and Unknown's (+0x2A 0x14, 0x1D, 0x21) a pack of that and their special part
  (part 0: more slots). 0x344EA8 keeps them by n at 0x8BF2C0 + 0x18 (paths), 0x8BF308 + 0x18 (pictures) and 0x8BF308 +
  0x38 (the special slots); a fight's set-up (0x344318) points set 4 + side and slots 14 + 3 x side (17-19), 20 + 3 x
  side at the side's fighter's, sending every frame of a slot (0x354120 packets at 0x1688A30 + 0x8A0 x slot + 0x30 x
  frame); the draw (0x3469D8) sends the frame a particle's +0x1E names (0x344030, clamped to the slot's last) before
  the particles of that frame (46 lists a slot).
- **The rows' routines** (0x5193B8, 0x347730) [H]: 0x346BB8 (most rows): the row 0x436688[row] (scale x 3, blend,
  brightness, the size's divisor), the frame +0x1E on by 1 while the path's frame runs and its fade is under 1, held
  on the slot's last; 0x346DA8 (rows 5, 16, 20) the same, round the slot; 0x346FA0 (Paul) scale 1.5, added,
  brightness 128, divisor 800 (a running path) or 1600, held; 0x347178 (Xiaoyu) 2.0, 300, still; 0x3472B8 (Eddy) 1.5,
  400, still; 0x3473F0 (Jin) 1.0, divisor gp-0x7F50 (533), round the slot every frame; 0x347568 (Jun) 1.5, 400 or 800,
  held. The row's colour lights the fighters from the effect for 54 frames (0x3531F0: gp-0x60B6 = 0x36, strength
  frames / 54). Exported (format 65): effects.json `characters` (GAMEDATA.md). **New** (format 70) each sheet's frames
  magnified to its cell bilinearly: the beam's queue (0x3A8BC0 -> 0x3A88B0) draws in context 2 (TEX0_2, ALPHA_2),
  whose TEX1_2 filters (0x61), and PCSX2 shows Devil's 32-texel eye flare smooth at some 80 pixels (was: nearest).
- **The beam's draw, completed** [H]: 0x378600 passes 0x3A8D08 the width only (the colour it makes is not used: the
  quads are drawn at 0x80), 100 x t + 50, or 130 x t + 70 when the record's +0xC is 1 (types 2 and 4: 0x378A28,
  0x378E70); t is 1 in a scene (0x3ABCD0). The segments run from the end back (the first, the end's, slot + 2). The
  flare's frame (gp-0x63D8 + 2 x side) and the jitters (18 of 36 set to rand() & 7) start at 0 with the beam
  (0x3A8C50), which also lights the fighters pink (gp-0x6EA8, ff 80 ff) from the head. The half widths are GS units
  over w as the hit effects' (0x433440). In a scene 0x378398 starts the beam the frame after op 16 and runs the type's
  routine (table 0x506230) with the op's count as its frame and the head's heading (0x378760: the head matrix's
  point (-100, 0, 0) from its origin) for the op's frames.

## 6. Not found or not read

Fog / depth cue: the arenas' strips with FGE (FORMATS.md 10), its colour from table 0x456540. The tag-in / tag-out effects, ki-charge glow, motion blur: not located. The ending scripts' effect
ops (BOWL_ENDINGS.md 7.4) use the same tasks.

Searched again (format 32) [H]:
- **Screen flashes**: the one full-screen fade (0x3A67D0: from a colour over n frames) is called only by the
  ending / attract script ops (0x3A9AB8's white flash), the transitions 0x3AC018 and 0x3BB310 and the title
  (0x3BCC88); nothing in a fight (rounds, K.O.) calls it, and the K.O. path has no slow motion (FIGHT_DATA.md).
- **Animated textures** (texture channels, as a fighter's, BOWL_ENDINGS.md 7.4) [H]: an arena's table of up to 8
  channels `{s16 channel, u16 dbp, x, y, w, h, dbw, psm}` closes geometry part 4 (after its light records; 0x356D40
  keeps it at gp-0x60A8), their frames are geometry part 2's pack [0][channel][frame] (an image and its palette; the
  image alone goes, 0x3A6EB0 makes a packet a frame). Records 0x8C9D30 (16 bytes a channel: +0 state, +4 frame << 8,
  +8 speed, +0xC frames); each frame 0x3A6C80 sends the frame shown of every channel in state 2 or more, then state 3
  goes on by +8 and stops on frame 0 past the last (0x3A73F8 starts it), state 4 loops (0x3A7430: from a start, back to
  0 past the last, the fraction dropped), state 2 holds (0x3A7490). By the arena (0x3A7198, table 0x519940): 1 channel
  0 at 0x40 (256 = a frame a frame); 2 0 at 0xC0; 3 0-1 at 0xC0, 2 at 0xC0 from 16; 4 the lightning (0-3 held on
  frame 1, whose texels the set-up clears (0x3A70C0: frame 0 is the sky's glow), 4-6 on frame 3 (frames 0-2 three
  bolts); task 0x3A76C0 below, not started on screen 11); 6 0 at 0x80, 1-2 at 0x100; 8, 17, 20 0 at 0x100; 9, 18 0 at
  0x100, 1 at 0x100 from 8; 11 0 at 0x80; 12 three tasks 0x3A7590 (+0x1A the channel, 0-2: state 0 holds frame 0;
  state 1 at rand() % 1000 < 7 plays it once at 0x80 (0x3A73F8); state 2 counts +0x80 a frame and, past the
  channel's frames, holds frame 0 again); 14 0-1 at 0x80. Exported (format 59): the looping ones as the arena's scene
  extras `channels`; **new** (format 60) the still ones arena 4's and 12's tasks play too, with their `channel`,
  speed 0, resting on `start` (arena 4's bolts' frame 1 cleared), and their tasks' data as stages.json `weather`
  `lightning` and `signs`.
- **Arena 4's lightning** (task 0x3A76C0, a frame each while not paused) [H]: state 0 sets the four bolts' records
  (0x8D5DE0, 16 bytes: +0 channel 0-3, +2 a count from rand() % 1000, never read, +4 on, +8 the list) and the three
  flickers' (0x8D5DB0: channels 4-6, +2 a count from rand() % 240). Each frame, each point (bolts 0x437C80, flickers
  0x437C50; 16 bytes, mm, the arena's axes) is projected (0x366EA8, 0x34B2A0): "near" when in front and within 10000
  (bolts, gp-0x7DF0) or 5000 (flickers, gp-0x7DEC) GS units (1/16 pixel) of the screen's centre. A bolt at rest
  starts at rand() % 1000 < 6: list rand() % 3 (0x519A20, 0x519A68, 0x519AB0, the same 35 frames of 0 and 1), with
  thunder 0x1072, 0x1074 or 0x1075 (0x374D30) when near. A flicker at rest counts +1: at 240 back to 0 and at rand()
  % 100 < 30 the long list 0x519990 with the roll 0x1073 when near; from 168, at rand() % 1000 < 20 a list of rand()
  % 3 (0x5199A8, 0x5199C0, 0x5199D8); below, at rand() % 1000 < 10 one of rand() & 1 (0x5199F0, 0x519A08). A list
  playing holds the channel on its frame (state 2), then steps (s16 a frame, to a negative); its end in the same
  frame rests the channel (a bolt's frame 1, a flicker's 3), so a list's last frame is not shown. A flicker's frame
  0 also shows its bolt's frame 0 (channel 4 -> 0, 5 -> 3, 6 -> 1; that bolt's own flash ended), another its bolt's
  frame 1. While near, a bolt's frame 0 (and a flicker's) sets 0x353C78(side, 1) for both sides: the fighters' lights
  0 and 1 with x and z negated (0x353758); any other frame of a near bolt and every list's end set it back. The
  EMBU's op 7 (0x3A7FA8(a, b)) starts task 0x3A7EB0: channel a plays list b of table 0x519B00 (0x519990, 0x5199A8,
  0x5199C0, 0x5199D8, 0x5199F0, 0x519A08, 0x519A20, 0x519A68, 0x519AB0), the same way, without the lights.
  Exported (format 60): stages.json `weather.lightning`.
  rows' +0x30 / +0x34 of table 0x520FC0) moves matrices only; no UV scroll or palette cycling in a fight. Paul's
  train's routine 0x3D5998 only sets the pad's rumble and the train's sound by its distance (0x3758F0).
- **The particles' depth**: arena effects (flag 4) are drawn only behind the farther fighter (0x346830 passes
  max(player +0x620) to 0x345A00, which skips a nearer particle; +0x620, 0x347DF0, a fighter's depth in view).

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
| 1 Law-A | 0x3D6FD8 | lights 4-13 in frames of their own x/z scale (0.77-0.96, y 1.1), 14-18 swayed: 0x3D7AA8 shears an object's matrix by the arena's wind at it (0x3D3D08, EFFECTS.md 5) times gp-0x7B68, 0x3D7378 by the fighters' impacts (0x3D3B80's four, 0x450000 - 0x4F00) too; a flock of 40 copies of 0-3 (0x444CD8) with their shadows (19, flattened along light 0: 0x3D7110) | 4-18; since format 32 14, 15, 17, 18 pitching on 0x3D7C38's curve (0x3D6BC0: a Newton polynomial over the times 0, 90, 350, 449 each taken twice, pitches 0, 0.1309, -0.0873, 0, flat at each; 450 frames). Since format 36 for OpenKumite to move (stages.json `objects.lanterns`, `objects.leaves`; 40 leaves and 40 shadows as objects 30-109, after the lanterns' frames 20-29): 4-13 (0x3D7AB0) are drawn through an 8-corner lattice (0x3E13C8, 0x46AF50: x, z +-s, y 0..s, s the float parameter) whose top four corners 0x3D7AB0 moves by the wind at the object x gp-0x7B68 (5000) turned into its axes; 0-3 (0x3D7378) are 40 birds (0x3D6BC0: copies of 0-3, k & 3, within 8 m at random, a random heading) that walk, are startled by an impulse (0x3D3B80, a fighter's landing) within 5 m and under 800 mm (pushed for strength x 60 frames), fly about and land below 50 mm, put back within reach past 8 m |
| 3 Xiaoyu | 0x3D52B0 | the carousel 0 turning 0.25 degree a frame (0x3D54F8), its horses 1-8 rising and falling 300 mm (phase 0.03 a frame, the second four half a turn behind), lamps 9-12 turned to face the camera (0x3D5640), 13, 14 | all; the lamps facing the camera (since format 22) |
| 9, 18 Nina | 0x3D5E80 | the helicopter 1 on its pad, 100 mm up and down (0.0316 a frame), its rotors (2: y, 3: x, 1.03 a frame); state 0 counts only the frames its centre is off the screen (projected outside 0x6C0-0x940, 0x720-0x8E0, or behind) and after 600 of them, in a fight (gp-0x74D0 4, gp-0x7434 0, or 0x11), takes off (state 2: rising t / 450 x 20 mm more each frame, its nose swung by a cosine, to 50 m) and comes back elsewhere at random (state 3); 0x3D5B58 makes 40 copies (0x3D1A80) of 0, 9 and 8 (by k & 7: 0-3, 4-6, 7) at y -2000 in 10 lanes (0x444CD0, 0x5C bytes each: x = k x 4000 - 17500, its speed (rand + 1) x 15 mm a frame either way, at most 20 cars at least 4 m apart), each frame moved along z and x and z kept within 20 m of the camera (gp-0x7C6C, a period of 40 m) | the helicopter waiting and its rotors; since format 32 stages.json `objects` (`helicopter`, `lanes`) and 40 copies of 0, 9, 8 as objects 10-49 (0 and 9: 2 x 2 x 4 m boxes, 8: a soldier) for OpenKumite to move: off the screen meaning the centre projected outside 640 x 448 in front of the eye; state 3 counts down from where state 2 stopped (the descent mirrors the climb), state 1 settles 2 mm a frame to y 0, bounces 120 frames (30 mm x (k / 120)^2 x sin(k pi / 60 + pi)), then waits 600 frames out of sight again (pitch gp-0x7C00, veer gp-0x7BF8 - rand x gp-0x7BFC) |
| 11 King | 0x3D4B18 | three helicopters (12, 15, 18 under frame 11) round an ellipse of 100 x 80 m, 0.1 degree a frame, the wingmen turned to their own bearing (0x3D5190), rotors 1.03 a frame; ten groups (0x444D20, 0xC0 bytes each) moving along z by their speed (+0x18) to an end (+0x48) where 0x3D47A8 puts them back at a random start, drawn (0x3D1160) with a fade within gp-0x7D0C of either end and turned to the camera (0x366EC8 less a turn of the frame counter 0x41580C & 0xFFF), their four objects 1-10 each drifting on its own random walk of two angles (+0, +4, by rand x +8, +0xC a frame; x and z sin of them times +0x10, +0x14) | the helicopters from angle 0; since format 32 stages.json `objects.clouds` and ten copies of 1-10 for each group (objects 21-120): a group on a route of table 0x44B1C0 (two, 32 bytes: x, y, z from, x, y, z to) at random, x and y at random on it, 20-40 mm a frame along z, back at the start past the end; its four members' angles from rand x 2pi, gaining rand x (rand x -0.0017 + 0.0052) a frame, reaches rand x 5000, at (reach sin a, reach sin b, 0); the group turned (frame & 0xFFF) / 2048 x pi less the camera's turn, its brightness the distance to the nearer end / 10000 within 10 m |
| 10, 13 Eddy | | their floors' grass (the floor row's +4 1): 0x3415D4 lays the window's cells through a lattice (0x3E13C8, x, z +-1000, y 0..1000, the row's +0x14) whose top corner at each corner of the window's cells (0x3D43A8: from -(n + 1) / 2 cells, the window's centre added, 0x34A9C0 a plain sum: the wind taken half a cell before each corner) moves by the wind there (0x3D3E00: the waves and the four impulses) x 120 and drops by that move's length + 200 mm. An impulse (0x44B100, 0x30 bytes: +0 on, +4 frames, +0x10 strength, +0x20 place) is a landing's (0x3D3C68 from 0x36DB30, 0x36DB88, 0x372E38: the fighter's x, y, -z; strength 30 x min(1, player +0x122 / 300)^2) where 0x447EC0's third word is set (arenas 1, 10, 13) and a slot is free (gp-0x6960); it lasts 300 frames (0x3D42A8) and pushes a point within 1500 mm (0x3D3FF8) away from it, strength x 2^(-frames / 10) x (1 - the distance / 1500) | since format 47 stages.json `objects.grass` (OpenKumite adds the impulses) |
| 14 Unknown | 0x3D2168 | 0 and 1 standing | both |
| 5, 7, 8 | 0x3D2160 (nothing) | none drawn | none |
| 20 Tekken Bowl | 0x3D21B8 | object 10 (the bowl draws the rest) | the bowl's own files |

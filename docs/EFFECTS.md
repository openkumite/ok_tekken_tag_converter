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
  them. Arena 14 (UNKNOWN) takes 0x342A38 instead (not read).

Alpha (x / 0x80) by arena 0-22: 30 48 30 40 40 40 50 40 40 40 48 40 40 48 20 40 40 40 40 20 40 40 40.

Lights (the shadow's direction): the arena's table **0x416288 + 0x54 x arena**: three directions (towards the
light, y up, the arenas' axes), three colours (0-255) and the ambient, floats [H]. In a fight 0x353448 moves the
lights with the fighter's place (table 0x4550B0) [M, not read]; 0x3537xx falls back to the arena table.

Exported: `gamedata/shadows.json` (part 8 as the Tekken 3 converter's shadow meshes), `gamedata/stages.json` (each
arena's shadow: light 0's direction, the alpha). Not: the shadow on objects, the place-dependent lights, the
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

A path: `u32 frames`, a 100 x 100 mm quad (four corners), then per frame position (mm), alpha (1 -> 0) and a
growing value [M]. Sets: table 0x8BF350 `{u32 paths, u16 count}` (0x343F20(k)); texture slots: 0x8BF3A0 + 0x18 x
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
- 0x3A2758(kind, at, flag) picks the set (kind 0 -> 0, 1 -> 3, 2 -> 1, 3 -> 2) and spawns its particles [M].
- The **SPECIAL EFFECTS** option (OFF: "Cancels Explosion Effects") [L: gates the 0x3A6110 explosions].
- **Ogre's fire** and the other projectile attacks (0x3A6390 each frame from 0x36D99C): attack bone 0x18 with body
  20 (True Ogre) in the active frames -> 0x3A4EE8; bone 0x19 -> 0x3A5810 [M, not read].

Exported: `pictures/effects/` (atlas.png, effects.json in the Tekken 3 converter's layout): sets 0-3's pictures
and file 8's frames as OpenKumite's spark, zero, block, dust and heavy strips. shortcut: each effect is one
camera-facing picture (Tekken 3's sprite), not the set's particle paths; the dust is not tinted by arena; no
character effect.

## 5. Arena effects [M]

0x3A2640 by arena (jump table 0x519770, arenas 4-20) starts the arena's effect tasks: 0x3A0A90, 0x3A0CE8,
0x3A1510 (+ 0x3A1378), 0x3A1068 (two variants), 0x3A1B48 (twenty objects 0x3A16C0 and thirty 0x3A18F8; 0x3A1BC8
draws file 9's discs, slot 5: a lens flare [L]), 0x3A1C78 for all. Not decoded further; not exported.
The converter leaves the arenas' additive strips (lights, floor gloss) and darkening strips (painted shadows,
dirt) out of the stages (GAMEDATA.md): not exported.

## 6. Not found or not read

Fog / depth cue, screen flashes and the K.O. look, the tag-in / tag-out effects, ki-charge glow, motion blur:
not located. The ending scripts' effect ops (BOWL_ENDINGS.md 7.4) use the same tasks.

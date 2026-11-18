# Tekken Tag Tournament (PS2, USA v2.00, SLUS-20001) — data formats

Everything here was worked out from the disc itself. Each section says what is verified and what
is still unknown. All values are little-endian unless marked. Offsets are hexadecimal.

## 1. Disc

A Mode 2 CD (2352-byte sectors, user data at +24). ISO9660 holds ten files:

| File | Size | What |
|---|---|---|
| `SLUS_200.01` | 2 873 728 | the game (EE, MIPS R5900 ELF) |
| `TEKKEN.BIN` | 630 474 752 | every resource |
| `IRX/RSPU2DRV.IRX` | 66 077 | Namco's IOP driver: sound and all disc reads (it looks up `\TEKKEN.BIN;1`) |
| `IRX/*.IRX`, `IOPRP165.IMG`, `SYSTEM.CNF` | | Sony modules and boot file |

## 2. TEKKEN.BIN file table (verified: all 748 entries check out)

The table is not in TEKKEN.BIN. It is in the executable at file offset `0x1CAE40`
(address `0x509E40`): 748 entries of 16 bytes.

| Offset | Field |
|---|---|
| +0 | `sector + 0x4820` — the file starts at `sector * 2048` in TEKKEN.BIN |
| +4 | `size + 0x350A` in bytes. A stored value of `0x3509` (size −1) marks an absent file |
| +8 | `index << 18 \| (sectors + 0x11F3C)` — repeats the index and the size in sectors |
| +12 | the index again for scrambled files, `0xFFFF` for plain ones |

Files 0 (48 MiB) and 747 (256 KiB) are random filler around the real data.

### First-sector scrambling (verified)

Files whose last word is not `0xFFFF` (and that are not file 44) have their first 2048 bytes
XORed, one 32-bit word at a time, with the low half of a 64-bit generator
(code at `0x398070`, `0x3980D8`, `0x398250`):

```
key  = 218319 * (index + 0xF3C) + (index - 0x348F) * 0x1234567      (64-bit, wrapping)
word ^= low32(key);  key = key * 0x71C3AB9 + 0x3941                  (per word)
```

Sound banks and streams are plain.

## 3. Packs (verified)

Most scrambled files are packs, and packs nest.

```
u32 count
u32 offset of part 0            (= header size, 8 + 8*count rounded up to 16)
count x { u32 size, u32 offset of the next part }   — the last "next" is 0xFFFFFFFF
```

Padding between parts is `0xAB`.

## 4. What the 748 files are

| Files | Content | Status |
|---|---|---|
| 1, 2, 45, 187 + 7k, 344 + 3k | sound banks (`VAGp` files back to back) | exported |
| 14, 15, 16 | movies: MPEG-2 video with stereo sound | exported |
| 17–40, 44 | music and jingles (stereo stream) | exported |
| 3–9, 43, 46, 129–136, 141–182 | 2D art: fonts, name plates, portraits, menus, logos | exported |
| 10, 133 | motion banks | exported |
| 345 + 3k (134 files) | fighters: pack of 11 parts | model, textures exported |
| 188 + 7k, 189 + 7k | arenas: pack of 5 (textures) and pack of 7 (geometry) | exported |
| 190–193 + 7k | 4 small float tables per stage | raw |
| 41–128 | cutscenes: camera path (`u32 frames`, then 7 floats a frame) + pack of 3 | raw |
| 12, 13, 137–140 | not identified | raw |

## 5. Sound (verified by decoding every file)

- **Bank**: `VAGp` headers (big-endian size at +0x0C, sample rate at +0x10, name at +0x20,
  data at +0x30) one after another. 2 350 sounds in 156 banks.
- **Stream**: no header. 2048-byte sectors, left channel in the first 1024 bytes and right in the
  second, 16-byte PS-ADPCM frames, 44 100 Hz. A frame with flag byte 7 ends the sound.
- **Movie**: 16 KiB MPEG-2 program-stream packs (`00 00 01 BA`, video stream `E0`, 640x448, 30 fps),
  with sound sectors of the stream format between them (7 before the first pack).

One stream file can hold several pieces: the endings' music is sectors 3319–4495 of file 20 (BGM id 7 starts
there, a constant of 0x397AB8; FIGHT_DATA.md section 14). Each file is one WAV (and one FLAC under `assets/`).
Movie 15's sound is 3 s shorter than its pictures, so it probably holds more than one piece too.

## 6. Textures (verified on fighters and 2D art)

A texture is a GS upload: a GIF packet setting `BITBLTBUF` (0x50), `TRXPOS` (0x51),
`TRXREG` (0x52), `TRXDIR` (0x53), then an IMAGE packet with the pixels in scan order.
Formats seen: PSMCT32 (0), PSMCT24 (1), PSMCT16 (2), PSMT8 (0x13), PSMT4 (0x14).
Palettes are 16x16 PSMCT32 uploads (8-bit) in CSM1 order: swap bits 3 and 4 of the index.
Alpha is 0x80 = opaque.

A fighter has one 256-wide PSMT8 sheet at block 0 and about 25 palettes from block 0x300.
A `TEX0` register picks a texture by block pointer: with 16x16-pixel blocks and a 256-wide
sheet, row `y = TBP0`. `TW`/`TH` often exceed what was uploaded; UVs are relative to `TW`x`TH`.

## 7. Fighter pack (11 parts)

| Part | Content | Status |
|---|---|---|
| 0 | pack: 30 uploads of 128x128 4-bit images, and 81 records of 0x3BC bytes (floats) — probably face animation | not decoded |
| 2 | pack of 6 tables of about 0x6B0 bytes | not decoded |
| 3, 4, 5 | small tables (4 and 5 look like short motions, type words 0x13 and 0x21) | not decoded |
| 6 | **`PS2TagA` model** | decoded |
| 7 | texture uploads for part 6 | decoded |
| 8 | low-detail model, 18 joints, 403 triangles, VIF packets | understood, not exported |
| 9, 10 | small tables (10 has 0x20-byte records) | not decoded |

### PS2TagA model (verified: bind pose closes to 0.000 units on shared vertices)

Header:

| Offset | Field |
|---|---|
| 0 | `"PS2TagA\0"` |
| 08 | 98: the nodes every fighter shares |
| 0C | float 100.0 |
| 14 | end of the node table |
| 24 | number of strips |
| 2C | 2 x 2 x 5 u32 section offsets (0 = none): pass, half, section |
| 7C | nodes, 0x20 bytes each |

Node: `i32 parent` (−1 = root), `float pos[3]`, `float rot[3]` (radians), `i32 extra`
(−1, or an id on cloth/hair nodes). Local matrix = `T(pos) · Rz(rot.z) · Ry(rot.y) · Rx(rot.x)`.
This is the bind pose. Units are millimetres, Y is up.

Section: `u32 ?`, `u32 size`, 8 bytes of zero, then records of 16-byte words until `size`:

- **Matrix palette** `{n ≤ 20, 0, 0, 0}` + 20 x u64 node indices. Vertices index it from 1.
- **Blend state** `{1, 0x20000000, 0x9E, 1}` + 2 quadwords (ALPHA, CLAMP).
- **Texture** `{1, 0x10000000, 7, 2}` + the TEX0 value twice.
- **Strip**: a GIF tag (`NLOOP` vertices, regs `0x412`; PRIM has ABE set for blended strips)
  followed by the vertices. One triangle strip per tag.

Vertex (variable size):

```
float s, t, 1.0;  u8 n (influences, 1..4) + 3 bytes
u32 palette index [4]
n x { float normal[3], 0;  float position[3] * weight, weight }
```

Positions and normals are in the node's local space and already multiplied by the weight,
so the skinned point is `Σ M_node · (p·w, w)`.

Sections (verified in the executable): the offsets are `[pass][half][k]`, at `0x2C + 0x28·pass + 0x14·half + 4·k`.
The loader (0x3AF624) relocates all 20. The fighter draw (0x343340, `a1` = pass) draws every section of the pass,
both halves, k 0 to 4: 0x357A78 calls it for pass 0 and 0x357DE0 for pass 1, for each fighter. Only the
lower half is skipped, when the game mode (gp−0x74D0) is 5. No section is an alternative: every one is drawn.
Across the 119 fighter models (files 345 + 3·n):

| Offset | Section | What it holds |
|---|---|---|
| 0x34 | pass 0, upper, k 2 | the upper body: head, torso, arms, hands (3 800–9 900 vertices; True Ogre's first costume has its legs here too) |
| 0x48 | pass 0, lower, k 2 | the legs, on the thighs, shins and feet (1 000–4 300 vertices): every model but True Ogre's first costume |
| 0x2C, 0x38, 0x3C | pass 0, upper, k 0, 3, 4 | small opaque pieces: Hwoarang's and Heihachi's (4 models), Ogre's head (2) |
| 0x54–0x64 | pass 1, upper, k 0–4 | blended only (14–1 100 vertices): hair (Paul's crest), fringes, manes, see-through cloth; 101 models have 0x5C |
| 0x70 | pass 1, lower, k 2 | blended only (8–430 vertices): cloth on the hips and legs, 14 models |

The research GLB keeps each section as `part_N` (N: the order of the non-empty offsets); OpenKumite's character
merges them all into one mesh.

### Joints

The 18 animated joints are these nodes (others are twist copies with the same rotation,
fingers, face, cloth):

`root 0, chest 1, head 5, clavicle 29, upper arm 31, forearm 34, hand 38` (side A),
`54, 56, 59, 63` (side B), `pelvis 79, thigh 81, shin 84, foot 87` (A), `90, 93, 96` (B).

## 8. Motions (verified: every bit of every block of all 4 291 motions is accounted for)

File 133 starts with 7 351 `{u32 offset, u32 size}` pairs. Offsets below `0x4000000` are in
file 133, the others are `0x4000000 +` an offset in file 10. 4 291 motions are distinct.

The format is the arcade's (System 12), repacked with 5-bit fields and 32-frame blocks:

```
+00  u8 frames
+02  frame 0: 57 x u16
+74  last frame: 57 x u16
+E6  u16 per 32-frame block: start of its bit stream, in 16-bit words from +00
```

57 channels = heading, height, distance of the root, then (x, y, z) angles for the 18 joints in
the order of section 7 (Tekken 3's slot order; the arcade uses another). 65536 = 360°.
60 frames a second.

Frame f (0 < f < frames−1) is in block `(f−1)/32`. A block restarts from frame 0's values and
holds one bit stream per channel (LSB first, 16-bit words):

```
5 bits   span: this channel's length in words
then per frame: 4-bit code; if code >= 8: code -= 8 and 5 more bits give (r + 1) & 31 repeats
  0      no change
  1..6   value += (bits + bias) << shift   widths 6,4,2,2,4,6   biases -84,-20,-4,1,5,21
  7      value  = 12 bits << shift
shift is 2 for height and distance, 4 for everything else
```

Joint rotation = `Rz(z) · Ry(−y) · Rx(−x)`, replacing the node's bind rotation.

Which motions belong to which fighter: section 9.

## 9. Fighters, movesets and moves (in the executable)

| File offset | Table |
|---|---|
| `0x1C0690` | 34 bodies x 8 costumes, `i16 slot` (−1 = none). The fighter's pack is file `345 + 3*slot`, its voice bank `344 + 3*slot` (code at `0x3674B4`: body at player +0x22, slot at +0x2E) |
| `0x1F2EA0` | 7 351 move records of 52 bytes (13 x u32). Word 0 is the move's own number, which is also its motion's number in file 133 |
| after the moves, +4 | 33 movesets x 5 532 `u16`: the move for each alias, 7350 = missing |

The roster is at `0x1BCC20`: 50 rows of 16 bytes, `u32 name pointer` (memory address = file
offset + `0x33F000`), `u8 body`, `u8`, `u8`, `u8 moveset`, `u8`; a name longer than 7 letters is
stored inline in the following row. Bodies 0-13 follow Tekken 3 (Paul ... Heihachi), then 14 Ogre,
15 Mokujin, 16 Gun Jack, 17 Roger, 18 Anna, 19 Kazuya, 20 True Ogre, 21 Jack-2, 22 P.Jack,
23 Michelle, 24 Wang, 25 Lee, 26 Armor King, 27 Ganryu, 28 Kunimitsu, 29 Devil, 30 Jun, 31 Bruce,
32 Baek, 33 Unknown. Tiger, Panda, Tetsujin, Alex and Angel have body numbers 34-38 and the moveset
of Eddy, Kuma, Mokujin, Roger and Devil, whose bodies they are costumes of. The moveset is not the
body number: True Ogre uses Ogre's (14), Mokujin and Unknown use 15, a stub of 323 moves.

Arena names are at `0x101770`: 20 rows of `u32 name pointer`, `u32 arena number`, in stage-select
order (Paul 0, Law-A 1, Law-B 15, Yoshimitsu-A 2, ...). Arena `n` is files `188 + 7n` (textures)
and `189 + 7n` (geometry); arenas 20-22 are on the disc but not in that list (20 is the Tekken Bowl hall).

A fighter's animations are the distinct moves of its moveset (about 1 400, shared moves included).
The other 12 words of a move record (posture, attack level, cancels, hit rows, sounds, flags,
as in the arcade) are in [FIGHT_DATA.md](FIGHT_DATA.md); the pack's `gamedata/` holds them ([GAMEDATA.md](GAMEDATA.md)).

## 10. Arenas (verified: all 2 605 materials of 23 arenas find their texture)

Files `188 + 7k` (pack of 5: textures) and `189 + 7k` (pack of 7: geometry), k = 0..22.

Each geometry part is a grid (millimetres, Y up):

```
u32 levels of detail   u32 size   u32 cells x   u32 cells z
f32 cell width   f32 cell depth   f32 height   ...
then levels x cells blocks, each opened by  STCYCL 4,4  FLUSH  BASE 0  OFFSET 0x200
```

Vertices are relative to the centre of their cell: cell `i` sits at
`((i % nx) - (nx-1)/2) * width`, `((i / nx) - (nz-1)/2) * depth`. The levels hold the same cells
at different densities; the exporter keeps the densest.

- A grid of several cells, or a flat one (seen: 4x4 cells of 2 m, and 1 cell of 8 m), is the
  **floor** around the fighters. The game repeats it in every direction as they move; the far
  ground has a hole there. The exporter repeats it to 1.5x the hole's radius (16–40 m).
- A tall grid of one cell is a fixed piece, or an **object the game moves** (the subway car of
  arena 1). The exporter puts those standing over the centre in a separate `objects` mesh.
- Parts that are not grids are scenery in arena space.

One batch is one triangle strip:

```
STCYCL 4,4   UNPACK S1-32 x1    flags
             UNPACK V4-32 x5    GIF tag (A+D, CLAMP, TEX0), ALPHA, CLAMP, TEX0, the strip's GIF tag
STCYCL 4,1   UNPACK V4-8  xN    vertex colours, 0x80 = 1.0
STCYCL 4,1   UNPACK V2-32 xN    texture coordinates
STCYCL 4,2   UNPACK V3-32 x2N   normal, position
```

Blending, when the strip's PRIM has ABE: ALPHA `0x44` ordinary, `0x48` additive (lights, floor
gloss; uses the vertex alpha), `0x89` a darkening decal. `TEX0.TCC` says whether the texture's
alpha counts.

Textures are 4-bit and 8-bit sheets (for example 256x2048 PSMT4 at block 0) with 16-colour
palettes of 8x2 pixels; `TEX0.TBP0` points inside a sheet at a page boundary.

**Texture banks.** Part n of the geometry pack uses part n of the texture pack (empty parts
aside). Every bank loads at the same GS addresses, so a `TEX0` only means something inside its
own bank. (Mixing the banks gave decals the wrong palette: they came out black.)

Blend modes in the GLB: glTF has no additive or multiply mode, so those materials are named
`additive` and `shade` (drawn correctly by the research tool's gallery, as plain blending elsewhere), ordinary
blending is `blend`, and repeated floor is `floor`.

Not done: objects standing where the fighters are (helicopter, boat, carousel, subway car) are
found by position and put in the `objects` mesh, hidden by default; how the game places or moves
them is not decoded. Arena part 0 holds low-detail skinned models (the fighter low-detail format)
that are not exported. Collision. One sky quad in arena 14 shows a palm-shadow texture.

## 11. Not decoded

- What [FIGHT_DATA.md](FIGHT_DATA.md) lists as not decoded.
- **Low-detail fighter model** (part 8): header `{0, 100.0, 0, data offset, 1, size in quadwords}`,
  18 joints of `{i32 parent, float offset[3], 16 bytes}` in Tekken 3 slot order, then VIF packets.
  Each strip: a 4-quadword material packet and `1 + 5n` quadwords of
  `{s, t, matrix A, matrix B}, normal A, normal B, position A·w + w, position B·w + w`
  where a matrix number is `1 + 7 * index`.
- **Cloth and hair movement**, face and hand poses.

## 12. Arena background characters

Part 0 of an arena's geometry pack is a pack: chunk 0 holds the motions, each further chunk one
character; texture part 0 holds one load per character, in the same order (every load uses the
same addresses).

**Character** (no magic): `u32 0, f32, u32 0, u32 0x2B0` (mesh offset), then at `0x70` 18 nodes of
32 bytes (`i32 parent, f32 x, y, z`, 16 bytes unused) - the fighters' 18 joints in the same order.
The mesh is VIF data for VU1: `UNPACK V4-32 x4` with a material (GIF tag, ALPHA, CLAMP, TEX0), then
strips as `UNPACK V4-32` of a GIF tag and five quadwords per vertex:
`s, t, matrix A, matrix B | normal A | normal B | position A, weight A | position B, weight B`.
A matrix number is `7 * node - 6` (0 = none); positions are in the node's space, multiplied by
their weight. There is no rest pose in the file.

**Motions**: either (offset, size) pairs followed by motions in the arcade packing (section 6 with
16-frame blocks, 4-bit fields and a `u16` frame count), or a pack of unpacked motions:
`u16 frames, u16 19`, then 57 `u16` per frame.

**Who stands where** is in the executable at `0x114FC8`: per arena three pointers - two lists of
people and a table of motion data. A list is 32-byte records up to one at the origin:
`u16, u16 start frame, u16, u16, f32 x, y, z, f32 heading (degrees, 0 = +z), u8, u8 character,
u8 motion (255 = scripted), u8, u32 script`. A script is 16-byte steps up to a null pointer, with
the motion number in byte 6 (255 = none; a few name a motion one past the arena's last, meaning
unknown). Not decoded: the script's conditions, the other record fields, and
Law's junks (stored as characters, but they come out wrongly posed).

The four small files after an arena's geometry (`190 + 7n` ...) are camera paths:
`u32 count`, then `f32 eye x, y, z, target x, y, z, field of view` per frame (y is down).


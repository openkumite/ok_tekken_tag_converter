# ok_tekken_tag_converter

Converts your own Tekken Tag Tournament disc (PlayStation 2, USA v2.00, SLUS-20001) into a pack of its fighters,
arenas, motions, pictures and sounds in modern formats, for OpenKumite (an open-source 3D fighting game engine, a
separate project). The pack's `assets/` folder is laid out as OpenKumite reads Tekken 3's converted data (below);
the rest keeps the layout of the research tool that worked the formats out, to look at in any glTF viewer.

> **Unofficial.** ok_tekken_tag_converter is not affiliated with, endorsed or sponsored by Bandai Namco
> Entertainment. Tekken is a trademark of Bandai Namco Entertainment Inc. This repository contains no game data:
> you need your own legally obtained disc, which the converter reads on your computer. Do not ask for, or share,
> disc images or converted files.

## Input

An image of the disc in any format `ok_converter_common` reads: `.cue`/`.bin`, `.iso`, `.chd`, `.ccd`/`.img`,
`.mds`/`.mdf`, `.ecm`, or one of those in an archive (`.zip`, `.7z`, `.tar`, `.gz`, `.xz`, `.zst`). It reads the
executable `SLUS_200.01` (the file table and the tables that name fighters and arenas) and `TEKKEN.BIN` (every
resource), one file of its table at a time. Another disc, or another version of this one, is an error.

## The library

- `pack::convert(ctx, disc, research)`: the disc's data track (an `ok_converter_common::io::DiscFile`) into a pack
  (any `Vfs`, emptied first); returns its manifest. It runs no other program and reads no file but the disc's.
  Without `research` it writes `assets/` and a manifest of the source alone: what OpenKumite plays.
- `pack::manifest(pack)`: a pack's manifest, checked.

A pack (about 2.2 GB, 15 234 files):

| Folder | Content | Format |
|---|---|---|
| `models/` | 133 fighters: mesh, 4-bone skinning, bind pose, textures | GLB (glTF 2.0) |
| `stages/` | 23 arenas with textures and vertex colours; their background characters (`NNN_pK.glb`) and those characters' motions (`NNN_mK.glb`) | GLB |
| `anims/` | 4 291 motions | GLB, one animation each |
| `textures/` | pictures: fonts, portraits, menus, arena textures | PNG |
| `audio/` | 2 350 sounds, 25 music tracks, the 3 movies' sound | WAV |
| `video/` | the 3 movies' pictures, **not converted yet**: as on the disc (`NNN.pss`, an MPEG-2 program stream) | PSS |
| `raw/` | files and packs not (fully) decoded | as on the disc, unscrambled |
| `gamedata/` | the fight data: 7 351 moves, cancel lists, hit records, reactions, hurt and push volumes, health, the fight camera, the 33 movesets' aliases | JSON ([docs/GAMEDATA.md](docs/GAMEDATA.md)) |
| `assets/` | what OpenKumite reads (518 MB, 7 087 files): below | GLB, JSON, WAV, FLAC |

`manifest.json`: `{"pack": "ttt", "format": 48, "source": {"image", "disc": "SLUS-20001", "version": "2.00"}}` and
the index of the files above: `models` (each fighter's `name`, `body`, `moveset`, `file`), `movesets` (the motions
of each moveset, a fighter's own first) and `own` (how many of them are its own), `anims`, `stages` (each arena's
`name`, `file`, `people`, `motions`, and `cast`: who stands where), `textures`, `sounds`, `music`, `video` (each
with `"converted": false` and its `sound`), `raw`, `gamedata`. `NNN` is a file's number in TEKKEN.BIN.

A motion file holds no mesh. Its channels target joints by name (`root`, `chest`, `thigh_A`...), which are the same
in every fighter. The disc has no move names, only numbers. Converting the same disc again gives the same bytes.

### `assets/`: OpenKumite's

The files OpenKumite's loaders read, in the Tekken 3 converter's layout (its `characters/`, `stages/`, `gamedata/`,
`audio/`, `pictures/`, `manifest.json`), for a fight of two Tekken Tag fighters on a Tekken Tag arena, with its sound and
music, and the front end around it:

| Path | Content | Size |
|---|---|---|
| `manifest.json` | `{"game": "ttt", "format": 8, "fingerprint"}`: the Tekken 3 converter's fingerprint (the same hash of the same canonical forms) over `characters/`, `motions/`, `gamedata/` | |
| `characters/model_NNN.glb` | the 127 fighters of the roster (body and costume), NNN as `models/`: the main mesh skinned on the 98 nodes every fighter shares, the 18 animated joints named as Tekken 3's, one RGBA picture, no animation | 95 MB |
| `motions/motion_NNNN.glb` | the 4 231 motions the characters' movesets play, each once: one clip `motion_NNNN`, no mesh; `bowl_NNN.glb` Tekken Bowl's 114 bowler motions (file 128), clips `bowl_NNN` | 82 MB |
| `stages/stage_NN.glb` | the 23 arenas by their number (`stages/index.json`: names): the scenery as a model around the fighters, the floor repeated; `bowl_ball.glb`, `bowl_pin.glb`, `bowl_deck.glb` the bowling hall's ball, pin and pin deck, apart (its object table's) | 44 MB |
| `gamedata/` | the fight data again (a move's `anim` is its clip), `movesets/motions_NN.json` (the clips of each moveset), `roster.json` (with the front end's tables, `front_end`: the main menu, the select grid, the HUD's pictures and places, the arcade ladder's rules, the unlock schedule, the options); `sound.json` (sound ids -> banks, voices, hit sounds, footsteps, scripts, announcer), `music.json` (the BGM table, each arena's), `rounds.json` (the round flow's frames, start motions, win poses), `bowl.json` (Tekken Bowl: its tables and constants) | 17 MB |
| `pictures/` | the front end's 252 pictures (title, fonts, faces, name plates, health-bar strips, portraits), named as `textures/`, in the palettes the code draws them with | 5 MB |
| `audio/` | `sfx/NNN/KKK.wav`: the 2 350 samples of the 156 sound banks (NNN: the bank's file); `music/NN.flac`: the 25 music streams (NN: their file); `index.json` | 45 MB WAV, 197 MB FLAC |
| `endings/` | the 38 ending scenes (`gamedata/endings.json`: their arena, cast and script, the ending table, the staff roll, Theater's lists): each one's camera path as on the disc, its two actor slots' body tracks as clips `ending_EE_K`, its face and hand tracks as on the disc | 39 MB |

The details: [docs/GAMEDATA.md](docs/GAMEDATA.md) ("assets/").

The formats are described in [docs/FORMATS.md](docs/FORMATS.md), the fight data in the executable in
[docs/FIGHT_DATA.md](docs/FIGHT_DATA.md), and the JSON written from it in [docs/GAMEDATA.md](docs/GAMEDATA.md).
The pack is the same bytes on every platform: float math goes through a pinned `libm`.

## The developer tool, `ttt_convert`

```sh
ttt_convert pack DISC PACK      # the disc image -> the pack (under a minute); PACK is emptied first
ttt_convert list DISC           # the 748 files of TEKKEN.BIN: sector, size, stream or not
```

## Not done yet

- In `assets/`: the arenas' additive and darkening strips (lights, gloss, decals), their background characters and
  moving objects; the select screen's layout (`roster.json` lists the fighters in body order).
- The movies: their pictures are written as on the disc.
- Move data (damage, frames, cancels), hit and hurt volumes.
- Names of the two unlisted arenas after Tekken Bowl (the stage-select menu does not list them); how the game
  places an arena's moving objects.
- Cloth, hair, face and finger movement: those nodes stay in their bind pose. The endings' face and hand tracks are
  written as on the disc (a hand track's 15 triples look like the 15 nodes under `hand_A` / `hand_B`, thumb first,
  but how they compose with the bind pose is not known).
- The endings' effect ops (sparks, objects, show / hide) and the arenas' background-character swaps; the EMBU
  exhibition's sequencing (files 41-42).
- The same output on every platform: the float maths is the standard library's (the sibling converters pin `libm`).

## Checks

`sh check.sh` runs formatting, clippy, the tests and `cargo deny`; the git hooks and CI run it. The library has no
`unsafe`, and clippy denies `unwrap`, `expect`, `panic!` and unchecked indexing in it: the disc is untrusted data.
The test that needs the disc is skipped without it:

```sh
TTT_DISC=/path/to/disc.zip sh check.sh      # also: the whole disc converted twice, the same bytes (most of a minute)
```

## Releases

A tag `vX.Y.Z` (Cargo.toml's version; only organisation admins push `v*` tags) publishes a release: `ttt_convert`
for Linux (amd64, arm64), Windows and macOS (universal), with SHA256SUMS (`.github/workflows/release.yml`).

## License

MIT ([LICENSE](LICENSE)). The repository contains no data from Tekken Tag Tournament.

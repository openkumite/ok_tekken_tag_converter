# ok_tekken_tag_converter

Converts your own Tekken Tag Tournament disc (PlayStation 2, USA v2.00, SLUS-20001) into a pack of its fighters,
arenas, motions, pictures and sounds in modern formats, for OpenKumite (an open-source 3D fighting game engine, a
separate project). OpenKumite does not read this pack yet: this is the foundation, a library and a command-line
tool ported from the research tool that worked the formats out, and the pack keeps that tool's layout until the
engine's side of it is written.

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

- `pack::convert(ctx, disc)`: the disc's data track (an `ok_converter_common::io::DiscFile`) into a pack (any
  `Vfs`, emptied first); returns its manifest. It runs no other program and reads no file but the disc's.
- `pack::manifest(pack)`: a pack's manifest, checked.

A pack (about 1.7 GB, 8 361 files):

| Folder | Content | Format |
|---|---|---|
| `models/` | 133 fighters: mesh, 4-bone skinning, bind pose, textures | GLB (glTF 2.0) |
| `stages/` | 23 arenas with textures and vertex colours; their background characters (`NNN_pK.glb`) and those characters' motions (`NNN_mK.glb`) | GLB |
| `anims/` | 4 291 motions | GLB, one animation each |
| `textures/` | pictures: fonts, portraits, menus, arena textures | PNG |
| `audio/` | 2 350 sounds, 25 music tracks, the 3 movies' sound | WAV |
| `video/` | the 3 movies' pictures, **not converted yet**: as on the disc (`NNN.pss`, an MPEG-2 program stream) | PSS |
| `raw/` | files and packs not (fully) decoded | as on the disc, unscrambled |

`manifest.json`: `{"pack": "ttt", "format": 1, "source": {"image", "disc": "SLUS-20001", "version": "2.00"}}` and
the index of the files above: `models` (each fighter's `name`, `body`, `moveset`, `file`), `movesets` (the motions
of each moveset, a fighter's own first) and `own` (how many of them are its own), `anims`, `stages` (each arena's
`name`, `file`, `people`, `motions`, and `cast`: who stands where), `textures`, `sounds`, `music`, `video` (each
with `"converted": false` and its `sound`), `raw`. `NNN` is a file's number in TEKKEN.BIN.

A motion file holds no mesh. Its channels target joints by name (`root`, `chest`, `thigh_A`...), which are the same
in every fighter. The disc has no move names, only numbers. Converting the same disc again gives the same bytes.

The formats are described in [docs/FORMATS.md](docs/FORMATS.md).

## The developer tool, `ttt_convert`

```sh
ttt_convert pack DISC PACK      # the disc image -> the pack (under a minute); PACK is emptied first
ttt_convert list DISC           # the 748 files of TEKKEN.BIN: sector, size, stream or not
```

## Not done yet

- The pack's layout for OpenKumite (the engine reads none of it yet).
- The movies: their pictures are written as on the disc.
- Move data (damage, frames, cancels), hit and hurt volumes.
- Names of the two unlisted arenas after Tekken Bowl (the stage-select menu does not list them); how the game
  places an arena's moving objects.
- Cloth, hair, face and finger movement: those nodes stay in their bind pose.
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

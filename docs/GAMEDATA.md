# The pack's fight data: `gamedata/*.json`

Written by `src/fight.rs` from the executable's tables ([FIGHT_DATA.md](FIGHT_DATA.md) says where they are and how
they were found). The manifest lists the files under `gamedata`. The same files are in `assets/gamedata/`, with
the files OpenKumite reads beside them (section "assets/" at the end).

The shapes are the ones the Tekken 3 converter writes (`ok_tekken3_converter`, read by OpenKumite's
`kumite_sim::data`) wherever Tekken Tag kept the concept: the same key names, the same nesting. **The numbers are
Tekken Tag's own, as stored**: aliases, condition codes, transition types, req codes, flag bits, property ops and
sound indices are not translated to Tekken 3's (section "Numbering" below). What Tekken Tag has and Tekken 3 lacks
is in added keys, marked **new**.

Conventions: every number is an integer as the game stores it, but the two float constants of `rules.json`, written
with their bits beside the value. Game units are about 1 mm, y down; 60 frames a second; angles are 65536 a turn.
`address` strings are memory addresses (file offset + 0x33F000). The files are one line each, written with keys in
a fixed order: the same disc gives the same bytes on every platform.

| File | Size | Content |
|---|---|---|
| `moves.json` | 6.3 MB | the 7 351 move records, with their sound and timed property lists |
| `cancels.json` | 8.4 MB | the moves' cancel lists, the shared lists, the input sequences, the reversal motion lists |
| `hits.json` | 0.4 MB | the 4 174 hit records (**new**) |
| `reactions.json` | 0.2 MB | the 1 431 reactions, the 74 pushback curves, the 44 air reactions |
| `rules.json` | 8 KB | health, hurt and push spheres, damage constants |
| `movesets/index.json` | 20 KB | which fighter and which model uses which moveset |
| `movesets/moveset_NN.json` | 28 KB each | the 5 532 aliases of moveset NN (0-32) |

Not separate files: Tekken 3's `shared.json` (its content is in `cancels.json`), and the proposed `properties.json`
and `sounds.json`: the lists are reached only from moves, so each move carries its own (`sounds`, `properties`), as
in Tekken 3's data, with the list's index beside it.

## `moves.json`

`{"address", "moves": [...]}`; `moves[i]` is move record `i`. There is one table for the whole game and no common
bank: a moveset reaches its moves through its aliases, and shared moves are the same record.

| Key | Record | Meaning |
|---|---|---|
| `index` | +00 | the move's number = its motion's number |
| `anim` | | the motion's file in the pack (`anims/NNNN.glb`: several moves share one file), null when the motion did not decode. Tekken 3: an animation's name |
| `motion` | +00 | the motion's number |
| `frames` | motion byte 0 | the frame count (record +18 is 0 on the disc: `bytes_18.frame_count_ram`) |
| `posture` | +04 | `raw`, `base` (bits 0-2), `state_code` (bits 11-15), and **new** `tag_state` (bit 24). The bits are Tekken 3's; no names are written |
| `attack_mask` | +08 | `raw`, `mask` (low 16 bits), `level` (bits 8-15), `level_name` (Tekken 3's levels: 1 low, 2 mid, 3 special_mid, 4 high, 6 unblockable, 7 throw, 8 throw_link) |
| `cancel_start` | +0C | index into `cancels.json` `cancels.entries`; the list runs to its `end` entry |
| `next` | +10, +12 | `alias`: the move played at the animation's end; `hi`: the turn added to the facing at commit (0x7FFF = about half a turn) |
| `hit` | +14 | **new**: index of the move's first hit record in `hits.json` (a chain, see there). Tekken 3 keeps damage and reaction in the move |
| `damage`, `damage_type` | | the chain's ungated (last) record: what the move does to an opponent no gate names |
| `damage_runtime` | | `damage` plus the arguments of the move's timed property op 0x0C, which the game adds at boot |
| `air_drift` | +16 | signed: the drift along the facing during the jump frames |
| `bytes_18` | +18..+1B | `frame_count_ram` (0), `jump_start`, `jump_end`, `hold_frame` |
| `active` | +2C, +2D | first and last active frame |
| `hitstop` | +2E | signed |
| `air_reaction` | +2F | the move's own air reaction (an index of `reactions.json` `air_reactions`, 8-43; 0: none), taken when the move juggles; non-zero in 36 moves. Tekken 3 has none |
| `flags24` | +24 | `raw` (the whole word) and `camera_script`: **the top byte** (Tekken 3: the low byte). No bit names: see "Numbering" |
| `push_control` | +30 | `raw`, `mode` (bits 0-3), `disable_mask` (bits 4-11), `mask_only_near_active` (bit 12): Tekken 3's modes |
| `word_32` | +32 | always 0 |
| `sound_list`, `sounds` | +1C | the list's index (null: none) and its entries `{raw, type, index}`: `raw` = `type << 12 \| index`. The indices are Tekken Tag's; which sample each is, is not decoded |
| `property_list`, `properties` | +20 | the list's index (null: none) and its entries `{frame, op, arg}`; ops are Tekken Tag's |
| `attack` | +28..+2B | null for a move with no bones and no active frame; else **new shape**: `bones` `{end0, start0, end1, start1}`, `segment_count` (2 when `end1` is not 0), `object_driven` (`end0` >= 0x18: the segment comes from a projectile object) |
| `raw_hex` | | the record's 52 bytes |

**Attack segments are not baked.** Tekken 3's data has a list of segments for each active frame (`attack.frames`);
Tekken Tag has only the bone pair. On each active frame a segment ends at bone `end`'s position and starts at bone
`start`'s, or, when `start` is 0, at the end bone's position of the frame before. Bones are slot numbers
(`rules.json` uses the same); bone 0x13 is 6 for body 14. The engine has to pose the skeleton to get the segments.

Not written (Tekken 3 has them): `motion_tag` (the reversal lists hold motion numbers here), `reaction` and
`close_range` (in the hit records), `desc0`, `next.move` and `dest_move` (an alias resolves per moveset).

## `cancels.json`

```
{"cancels":        {"address", "count": 33487, "entries": [...]},
 "shared_cancels": {"address", "count": 782, "list_starts": [40 indices], "entries": [...]},
 "sequences":      {"address", "entries": [{"cmd", "address", "window", "input", "steps": [{"raw", "dir", "buttons", "tag"}]}]},
 "reversal_families": {"address", "families": [[motion, ...], ...], "offsets": [...]}}
```

`cancels.entries` is every move's list, one after another; a move's list starts at its `cancel_start` and ends in
exactly one `end` entry, whose `dest` is the move played at the animation's end and whose `win_start` is the first
frame another move can start. A `call` entry runs the shared list at `shared_cancels.entries[shared_list_start]`
up to its `return`; `list_starts` are the 40 lists' first entries.

An entry (14 bytes; Tekken 3's are 12):

| Key | Meaning |
|---|---|
| `index` | its place in its table |
| `cmd` | the command, raw |
| `kind` | `command` (`cmd` < 0x8000), `end` (0x8000), `dash` (0x8001), `backdash` (0x8002), `special_8003` (**new**, tied to Unknown), `call` (0x8004), `return` (0x8005), `sequence` (>= 0x8006) |
| `input` | the command as text, in the Tekken 3 data's notation: numpad directions `d/b d d/f b n f u/b u u/f`, buttons `1`-`4`, `tag` (**new**: bit 14), `a\|b` either direction, `(1\|2)` any of the buttons, `*` any direction, `(no button)`; a sequence is its steps joined by commas (motions are not folded into `qcf`) |
| `req` | the gate, raw (ranges below) |
| `cond`, `cond_tag` | the condition code (the byte's low 7 bits) and **new** its bit 0x80: a partner/tag test is added |
| `param` | the condition's parameter |
| `dest` | the alias of the move to start |
| `transition` | the 16-bit word (**8 bits in Tekken 3**); `transition_type` = bits 0-13, `turn_180` = bit 15 (Tekken 3: 0x80), `transition_bit14` (Tekken 3's `transition_bit6`) |
| `prefilter` | **new**: a mask of stick-direction classes, plus 0x20 for any button; a quick reject before the input test |
| `win_start`, `win_end`, `exec` | the frames the input is accepted on, and the frame the move starts (255: at the animation's end) |

`sequences.entries`: the motion inputs of the commands >= 0x8006 the lists use (100), by `cmd`. `window`: the
frames the steps must fit in. A step: `raw` = `buttons << 8 | numpad direction`; `dir`, `buttons` (1-4), `tag`
(bit 14).

`reversal_families.families`: the motion lists the reversal conditions test, in the table's order; `offsets[k]` is
where list `k` starts in the table, in 16-bit units. How a condition's parameter picks a list is not decoded (the
engine will need the offset or the order).

## `hits.json` (new)

`{"address", "count", "reaction_none": 1426, "records": [{"index", "gate", "damage", "damage_type", "reaction",
"close_reaction", "close_distance"}]}`

A move's `hit` starts a chain: the first record whose `gate` passes is used; the record with `gate` null (0xFFFF
on the disc) always passes and ends the chain. A gate is a req code (0x53: the opponent's moveset is 16, Gun
Jack's). `reaction` and `close_reaction` index `reactions.json` `entries`; the close reaction replaces the other
when the fighters are nearer than `close_distance` (0: no close reaction). `damage_type` is the damage word's top
2 bits, as in Tekken 3.

Tekken 3 has one damage and reaction per move, and a `close_range` table of 58 pairs; a chain by opponent does not
fit that shape.

## `reactions.json`

`{"reaction_table", "slot_names", "param_names", "none": 1426, "entries": [{"index", "victims", "params"}],
"pushback_curves": {"address", "count", "entries": [{"index", "constant_frames", "velocity", "impulses"}]}}`

- `victims`: 14 aliases, the victim's move by `slot_names` (Tekken 3's slot order).
- `params`: 7 values by `param_names`: three angles (65536 a turn) and four curves. A curve is **the index of an
  entry of `pushback_curves`**, or null (Tekken 3: an offset in 16-bit units, a multiple of 10).
- A curve has **10 impulses** (Tekken 3: 8).

`air_reactions`: `{"address", "entries": [{"index", "alias", "target_height", "rise", "words", "pushback"}]}`, the
juggle reactions (FIGHT_DATA.md section 9): 0 normal, 1 spike, 2 / 3 side hits (quadrant 3 / 1), 4 the launch of
transition 30, 5 / 6 / 7 the face-down victims (by height; 7 a spike), 8-43 a move's own (`moves.json`
`air_reaction`). `alias` is the victim's move, `target_height` the height the pop reaches at least, `rise` a fixed
rise speed (400 for the spikes; 0 none), `words` three values not decoded, `pushback` 8 per-frame samples away from
the attacker. Tekken 3's shape is `{index, target_height, alias}` with one `air_pushback_curve` for all; here each
entry has its own.

Tekken 3's throw victims and close-range tables are not here: not found as tables in Tekken Tag.

## `rules.json`

- `health`: `life_options` (8 words of the table, `life_options_raw` in 20.12 fixed point: 125, 130, 135, 140, 160
  and three more 135), `default_life_option` (3: every match's, set by 0x3B1488) and `default_max` (140, its
  value; Tekken 3's key), **new** `percent_per_character` (34 bodies: a body's
  health is the option times its percent), **new** `recoverable`: the part of a hit's damage the recoverable cap
  loses, `damage_factor` (0.6) and `damage_factor_flag_12a` (1.4), each `{bits, value}`.
- `dialect`: `"ttt"`, the game whose numbers the files hold (section "Numbering").
- `hurt_spheres`, `push_spheres`: `layout` (`index`, `bone_slot`, `offset_game`: the head's sphere is lifted 120,
  the root's 60; `gltf_joint`, the slot's joint name in the glbs, and `offset_gltf_m`, the offset in metres with y
  up, as Tekken 3's) and `per_character` (`character`: the body number, `name`, `radii`). Tekken 3's keys; a
  character here is a body (0-33). Costume bodies 34-38 are not in the tables; they are assumed to use their host
  body's (8, 11, 15, 17, 29).

- `counter_hit`: `multiplier` [12, 10] (D x 12 / 10) and `bonus_tenths` [3]: the bonus is the victim's move
  damage times each tenth in turn (truncating), then x its frame / its startup (Tekken 3's would be [3, 3]).
- `chip_damage`: `divisor` 10, `add` 0 (Tekken 3: D / 10 + 2), only under the match flag or a ki charge.
- `juggle`: `first_air_hit_percent` 80, `later_air_hits_percent` 50 (Tekken 3's keys and values).
- **new** `tag` (FIGHT_DATA.md section 10): `regen_divisor` (the benched partner regains max / 6000 a frame) and
  `anger`: `start` 100, `angry` 120, `hit` 4 (`hit_no_damage` 6) added to the partner per hit on the active member,
  `damage_tenths` 13 (x 1.3 while angry), `any_body` 0x22, and `relations` (the 76 entries of 0x5207F0:
  `partner_body`, `partner_mask`, `active_body`, `active_mask`, `effect`: -1 adds 3, 1 adds 5, -9 none).

These four are constants of the code (0x3720C8, 0x3C1350), written as data. There are no damage zones (the hurt
sphere does not scale damage). Throws and round time are not here.

## `movesets/`

`index.json`: `models` (each fighter model of the pack: `model` and `model_record` its file number, `glb`, `body`,
`moveset_id`, `moveset_file`), `fighters` (the roster's 39: `name`, `body`, `moveset_id`, `moveset_file`), `files`, `aliases`
(5532).

`moveset_NN.json`: `moveset_id`, `fighters`, `move_count` (distinct moves), `aliases`: 5 532 values, the move of
each alias, **a number of `moves.json`, or null** for a missing alias (7350 on the disc). Tekken 3: `{bank, move}`.

## Numbering: what differs from Tekken 3

The engine needs a table per game for these; nothing below is translated in the files.

- **Aliases**: other numbers (stand is 58, was 3; crouch 80, was 40); 5 532 of them, was 4 023.
- **Commands**: specials are 0x8000 + n, was 0xC000 + n; 0x8003 is new, call is 0x8004 (was 0xC00C), return
  0x8005; sequences start at 0x8006 and the command is the sequence's place in a table, not an index. Bit 14 of a
  plain command is the tag button.
- **req**:

| req | Passes when |
|---|---|
| 0x01-0x21 | own moveset = req - 1 |
| 0x22-0x42 | own moveset != req - 0x22 |
| 0x43-0x63 | opponent's moveset = req - 0x43 |
| 0x64-0x84 | partner's moveset = req - 0x64 |
| 0x85-0xA6 | own body = req - 0x85 |
| 0xA7-0xC8 | partner's body = req - 0xA7 (unused) |
| 0xC9-0xEA | own body != req - 0xC9 |
| 0xEB-0xED | flags; 0xEC / 0xED = Kazuya-Devil pair or not |

- **cond** (FIGHT_DATA.md section 7): 0-0x24 as Tekken 3; 0x25-0x27 are new (0x25 not back turned); Tekken 3's
  0x25-0x44 are + 3 (0x28-0x47); reversal codes start at 0x60 (Tekken 3's 0x45 + k is 0x60 + k); 0x48-0x5F are new
  camera, screen-side and round tests; bit 0x80 is `cond_tag`, a test on the opponent's tagging.
- **Transition types** (section 8): 46; 0-28 as Tekken 3, 29 / 30 are Tekken 3's 0x1D / 0x1E, 31 and 32 its 33
  (throw link), 33 a restart facing the opponent, 34 / 35 a turn to the screen axis / its perpendicular, 36 Tekken
  3's 0x22, 37-45 its 0x23-0x2B, 46 its 44 ("use the END entry").
- **Flag bits** (`flags24.raw`), Tekken 3 bit -> Tekken Tag bit: 8, 11, 12, 13 -> 0, 3, 4, 5; 18 -> 11; 28, 29,
  30 -> 19, 20, 22. By code: 0x04 unthrowable, 0x08 not parryable, 0x40 guard by input, 0x200 ki charge, 0x400
  drops push spheres, bit 19 commits the root. New: 0x100 the posture follows the stick. Unknown: 0x4000, 0x8000,
  bits 17 and 21. The camera script is bits 24-31.
- **Property ops**: 1-10 as Tekken 3; damage to the opponent is 0x0C (was 0x0B), damage to self 0x0D; hand poses
  moved up (0x0E -> 0x0F, 0x15 -> 0x18). Ops up to 0x58 occur; most are not decoded.
- **Sound entries**: the type nibbles are Tekken 3's (6, 7, 2, 8, 9; rarely 0, 1, 3); the indices are renumbered.
- **Reaction none**: 0x592.

## Raw or unknown

Posture and attack-mask bits beyond those named; flag bits 0x4000, 0x8000, 17, 21; the camera values cond
0x48-0x5B test; transition bit 0x4000's effect; req 0xEB; property ops past the ones listed; sound index -> sample;
camera scripts; the `prefilter`'s classes (table 0x4FF8D0); how a reversal condition picks its motion list; the
step bits of a sequence other than direction, buttons and tag; the air reactions' three `words`.

## `assets/`: what OpenKumite reads

`assets/` is a folder OpenKumite opens as it opens Tekken 3's converted data (`kumite_assets::Assets`): the Tekken 3
converter's layout and shapes. Written by `src/pack.rs` (`assets.rs`, `model::fighter_glb`, `anim::clip_glb`,
`stage::game_glb`).

| File | Content |
|---|---|
| `manifest.json` | `{"game": "ttt", "format": 8, "fingerprint"}`. `format` is the layout's (the Tekken 3 converter's FORMAT, which the engine asks for at least); `fingerprint` is the Tekken 3 converter's `convert_all::fingerprint` (SHA-256 over each `.glb` and `.json`, by path in sorted folders, of a JSON file with its keys sorted and a glb without its buffers, views and images then its accessors' data; the first 16 bytes in hex), over `characters/`, `motions/` and `gamedata/` |
| `characters/model_NNN.glb` | a fighter: body and costume of the roster's (127). NNN is its pack's file number in TEKKEN.BIN (345 + 3 x costume slot), as `models/NNN.glb` |
| `motions/motion_NNNN.glb` | a motion (4 231: those the characters' movesets play). NNNN is its number in file 133's table, the number of its first move (as `anims/`) |
| `stages/stage_NN.glb` | an arena (23). NN is its number (geometry file 189 + 7 x NN; 20 is Tekken Bowl, 21 and 22 are not in the stage list); `stages/index.json`: `[{"id", "name", "file"}]` |
| `gamedata/*.json` | the files above, but each move's `anim` is its clip's name (`motion_NNNN`), not a file |
| `gamedata/roster.json` | Tekken 3's shape: `characters` (a body each: `id`, `name`, `moveset_id`, `selections`), `selections.entries` (each character file: `selection` = body x 8 + costume, `character` = body, `costume`, `name`, `model_record`, `glb`, `moveset_file`, `moveset_id`), `select_grid.entries` (every character, in body order: the select screen's layout is not decoded) |
| `gamedata/movesets/index.json` | as above: its `models` are the characters (`model_record`, `glb` = `model_NNN.glb`), the shape of Tekken 3's |
| `gamedata/movesets/motions_NN.json` | `{"moveset_id", "motions": ["motion_NNNN.glb", ...]}`: the motions moveset NN's moves play |

**A character** is every section of the model (the fight draws them all: FORMATS.md section 7) in one mesh, skinned
with up to 4 influences on the first 98 nodes, which every fighter shares, in their bind pose (`skins[0].joints` =
nodes 0-97). A vertex on a node past them (cloth, hair) is put on its nearest ancestor among them: no motion moves
those nodes, so the vertex stays where it was. The 18 animated joints are named as the Tekken 3 converter names its
joints, slot for slot (FORMATS.md section 7): `root` 0, `chest` 1, `head` 5, `clavicle_A` 29, `upperarm_A` 31,
`forearm_A` 34, `hand_A` 38, `clavicle_B` 54, `upperarm_B` 56, `forearm_B` 59, `hand_B` 63, `pelvis` 79, `thigh_A`
81, `shin_A` 84, `foot_A` 87, `thigh_B` 90, `shin_B` 93, `foot_B` 96; a slot's twist copies are `name.1`, `name.2`
(they turn with it); the other nodes `node_N`. The texture is one RGBA PNG (OpenKumite's characters have one): each
strip's texture in its palette, cut to the texels its strips reach and repeated as far as they reach (the GS repeats
a texture; to 1024 texels: 53 vertices of all the characters reach further and are clamped), packed with a border of
edge texels. The opaque strips are the first primitive (material `opaque`), the blended ones the second (`blend`),
with their texture's alpha; each primitive has its own vertices. No animation.

**A motion** has no mesh: nodes named as the joints and their twist copies, and one animation `motion_NNNN`, a key
a frame at 60 fps: each joint's rotation (normalised 16-bit integers, as the Tekken 3 converter writes them; a twist
copy takes its joint's) and the root's position in metres (`translation` of `root`: its absolute height, and its
heading and distance as x and z, forward being -z), which replaces the root's own. Moves that share a motion share
its file. Motions are stored once (80 MB): a file per moveset with every motion it plays would be 869 MB.

**A stage** is laid out as the arcade converter's `write_game`: node `sky` is the scenery, which the game keeps
centred on the fighters (scene extras `drum: "model"`, `drum_cull: false`: the strips' winding is not stored, both
sides are drawn; `background_rgb` black); node `floor` holds copies `floor_K` of the floor mesh, a period apart, to a
little past the hole in the far ground, and the scene extras' `floor_period` (metres) is that period. An arena's
floors are one: the one of the longest period, each shorter one (2 or 4 m in 8 or 4 m) laid inside it. Only the
opaque, cut-out and ordinarily blended strips are there: the additive ones (lights, floor gloss) and the darkening
ones (shadows, dirt; on Lei's arena the wet street's, so its reflections show bare) are left out, and so are the
background characters and the objects the game moves. Each primitive (a picture and a blend mode) has its own
positions, texture coordinates and vertex colours; the first image is a white texel, which strips without a texture
draw; each picture is cut to the texels its strips reach (all of it along an axis they repeat it on).

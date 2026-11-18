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
| `camera.json` | 78 KB | the fight camera: projection per arena, framing constants, round-start fly-in, per-move camera lists |
| `ai.json` | 60 KB | the CPU AI's tables (**new** file here; Tekken 3's shape) |
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
| `flags24` | +24 | `raw` (the whole word) and `camera_script`: **bits 23-31**, the camera id (Tekken 3: the low byte). No bit names: see "Numbering" |
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

## `camera.json`

The Tekken 3 converter's `camera.json` (OpenKumite's `kumite_sim::camera`), from FIGHT_DATA.md section 18: Tekken
3's camera module with Tekken Tag's constants. Angles 4096 a turn here (Tekken 3's units). Tekken 3's keys with
Tekken Tag's values, unless **new**; Tekken 3's `tables` (sine, atan), `gsqrt`, `shake`, `ko`, `win`, `replay`, `end`
and `attract` are not written (not decoded).

- `projection_h`: arena 0's H (615; Tekken 3 500 everywhere). **New** `arena_projection.rows` (0x507FE0, by arena):
  `arena`, `h`, `h_min`, `h_max` (the clamp: 600-900, 500-900 on arenas 5, 11, 12), `dolly` (a camera motion's zoom
  is a dolly; 0 on arenas 3, 7), `variant` (1 on entry 24, H 850, which no stage of the list uses). **New**
  `screen_width` 512: the pixels H and the framing's constants are measured in (the fight's 512-wide frame: off
  screen is x >= 533 or < -20, 0x34FE7C; the constants are Tekken 3's x 512 / 368). Tekken 3's is 368.
- `fight`: the framing's constants (code 0x38F7F8, 0x38FD90), Tekken 3's keys: `distance` 4564 at H 615 and **new**
  `distance_per_h` [1900, 256] (the depth is H x 1900 / 256), `fit_px` 392, `dead_zone_px` 68, `vertical_keep_px`
  156, **new** `centre_drop` 140 (the drop is 140 - 450 x H / distance; Tekken 3 100), `initial_eye` [0, -1200,
  -8000], `yaw.move_caps` by camera id 0x13-0x16 (Tekken 3 0x27-0x2A); the rest as Tekken 3's.
- `intro`: the round-start fly-in (0x3954E8): one offset (0x509C20, eye z -3700), `frames_first_round` 120,
  `frames_other` 90, **new** `ease_frames` 90 (the offset eases over the last 90 frames; round 1 holds it for the
  first 30), `blend_frames` 13 (the hand-off to the fight camera).
- `situation_presets` (0x508908): the 30 situation types, Tekken 3's shape (`field_1` is the hold).
- **New** `move_camera_lists` (0x434C78; Tekken 3's `script_selectors`): `lists` by camera id (move flags >> 23)
  with an entry: `id`, `split`, `first_mirrored`, `entries` (`flags`, `weight` out of the list's sum, `type` 1 a
  situation type / 2 a camera motion / 0 none, `value`, and `mirrored_type`, `mirrored_value` for the mirrored
  orientation).
- **New** `camera_motions`: `count` 485 in disc file 12 (offsets at 0x508130), `decoded` false: the motions' format is
  not decoded, so the lists' type-2 entries name nothing a player can play.

## `ai.json` (new)

The CPU AI's tables (FIGHT_DATA.md sections 19 and 21), in the shape of the Tekken 3 converter's `ai.json`, with
`dialect` `"ttt"`:

- `profiles.entries` (0x431918, 128 bytes, 5 difficulties EASY .. ULTRA HARD x 8 levels): `difficulty`,
  `difficulty_name`, `level`, `fields`: `wake_up_intervals` (4 bytes), then Tekken 3's named fields (the Tekken 3
  converter's names), each 4 bytes later than in Tekken 3's profile; **new** Tekken Tag's own readers (section
  21.8): `stun_attack_prob` (P68), `tag_out_prob` (P6E), `stun_tag_out_prob` (P70), `guard_after_reaction_prob`
  (P72), `delay_input_prob` (P74), `repeat_count` (P76), `repeat_guard_prob` (P78); the others as `off_XX`, XX the
  offset in Tekken Tag's profile. Tekken 3's `followup_count_a` / `followup_prob_a`, `consecutive_attack_limit_a` /
  `followup_prob_b`, `consecutive_attack_limit_b` / `followup_prob_c` are Tekken Tag's lists C, B and D (P32-P3C).
- `spacing.entries` (0x432E10): per body (`character`), the 6 reach thresholds.
- `zone_masks` (0x4318D8, 0x4318F0): Tekken 3's values.
- `handlers.entries` (0x507A80, 34): `moveset`, `vs_handler`, `self_handler` (code addresses); `routines`: the
  seven live ones (section 21.6) as the Tekken 3 converter's kinds (`back_off` Paul, `canned_throws` King,
  `stance_dash` Hwoarang, `fire_dodge` Ogre and Devil / Angel) with their aliases, and **new** `poke_or_crouch`
  (Ganryu); Yoshimitsu's is not described (a stub).
- `canned_sequences.entries`: `sidestep_0` / `sidestep_1` (0x507452 + 0x18 x side), `backdash` (0x5074A2),
  `okizeme` (0x507482), King's `throws_stand_0..2` (0x507A52), `throws_crouch_0..1` (0x507A32), `throws_down_0..1`
  (0x507A12), Ganryu's `poke` (0x507618): step words (buttons << 8 | numpad direction), the first one as it is.
- `boss_punish_pads` (0x507A08): Ogre's four pad words (Devil's at 0x507A00 are the same).
- `motion_records.entries`: commands 0x8001 / 0x8002 (the dash and the backdash, 0x36CF30), `cmd` Tekken Tag's.
- `evade_choices` (0x507990, u32), `wakeup_categories` (0x5078E0, u64): Tekken 3's values.
- `rules`: Tekken 3's keys with none of their move sets (-1) or aliases (0xFFFF), and **new** `followup_rules`: the
  per-alias rule of the follow-ups and chain throws (0x3876A4): `aliases`, `movesets` (mine; all when absent),
  `allow` (`never`, `not_easy`, `coin` / `no_coin`: `rand() & 0x70` set / clear).
- `rng`: `ai_lcg_seed` (640, the AI generator's start, gp-0x64E8) and `rand_shift` 0 (libc `rand()` returns bits
  0-14 of its state; Tekken 3's 16-30).
- **new** `distance_jitter`: `amplitude` 192 and `sine` (0x4FCE88, 4096 x s16, 4096 = 1): bookkeeping's swing of
  the distance before the zone tests.

Not exported: the survival ladder and the attract mode's profile.

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
  bits 17 and 21. The camera id is bits 23-31 (FIGHT_DATA.md section 18.2).
- **Property ops**: 1-10 as Tekken 3; damage to the opponent is 0x0C (was 0x0B), damage to self 0x0D; hand poses
  moved up (0x0E -> 0x0F, 0x15 -> 0x18). Ops up to 0x58 occur; most are not decoded.
- **Sound entries**: the type nibbles are Tekken 3's (6, 7, 2, 8, 9; rarely 0, 1, 3); the indices are renumbered.
- **Reaction none**: 0x592.

## Raw or unknown

Posture and attack-mask bits beyond those named; flag bits 0x4000, 0x8000, 17, 21; the camera values cond
0x48-0x5B test; transition bit 0x4000's effect; req 0xEB; property ops past the ones listed; sound index -> sample;
the camera motions; the `prefilter`'s classes (table 0x4FF8D0); how a reversal condition picks its motion list; the
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
| `motions/bowl_NNN.glb` | Tekken Bowl's bowler motions (114, file 128 part 0, BOWL_ENDINGS.md 4): clip `bowl_NNN`, NNN its track |
| `stages/stage_NN.glb` | an arena (23). NN is its number (geometry file 189 + 7 x NN; 20 is Tekken Bowl, 21 and 22 are not in the stage list); `stages/index.json`: `[{"id", "name", "file"}]`. Tekken Bowl's hall without its objects, which stand at the origin until the game moves them; `bowl_ball.glb`, `bowl_pin.glb` (its base at the origin) and `bowl_deck.glb` (the pin deck where the object table puts it) are the objects the bowl draws (BOWL_ENDINGS.md 6) |
| `gamedata/*.json` | the files above, but each move's `anim` is its clip's name (`motion_NNNN`), not a file |
| `gamedata/roster.json` | Tekken 3's shape: `characters` (a body each: `id`, `name`, `moveset_id`, `selections`), `selections.entries` (each character file: `selection` = body x 8 + costume, `character` = body, `costume`, `name`, `model_record`, `glb`, `moveset_file`, `moveset_id`), `select_grid.entries` (every character, in body order), and **new** `front_end` (below) |
| `gamedata/shadows.json` | **new** (format 12) the fighters' shadow meshes in the Tekken 3 converter's shape: `{"meshes": [{"models": [record], "joints": the 18 joint names, "vertices": [[joint, x, y, z]] (metres, in the joint's axes), "triangles", "quads": []}]}`, one per character file, from its pack's part 8 (the low-detail model the game draws its shadow with; a vertex on its heavier joint, a hair or cloth joint's carried by its parent: EFFECTS.md 2) |
| `gamedata/stages.json` | **new** (format 12) `{"stages": [{"id": arena, "shadow": {"toward", "colour": [0, 0, 0], "alpha"}}]}`: the arena's light 0 (0x416288, turned up: the shadow falls along the line either way) and the alpha the floor is darkened by under a shadow (0x46B280 / 0x80); the Tekken 3 converter's stage without a light table, plus `alpha`; **new** (format 13) `lights`: `{"toward": [[x, y, z] x 3], "colour": [[r, g, b] x 3], "ambient": [r, g, b]}`, the arena's three lights for the fighters as the table holds them (towards each, the arena's axes, y up; 128 = 1: EFFECTS.md 2); `dust`: `{"count", "scale", "colour": [r, g, b]}`, the arena's dust (0x437AC0: so many particles on paths at random, their size times `scale`, colour 128 = 1; EFFECTS.md 4) |
| `pictures/effects/` | **new** (format 12) `atlas.png` (128-texel cells: the four effect sets' pictures, files 4-7, and file 8's 20 fireballs) and `effects.json` in the Tekken 3 converter's shape: strips `spark` (set 0, 35 frames), `zero` (set 3, 30), `block` (set 1, 40), `dust` (set 2, 150, half blend), `heavy_a` / `heavy_b` (the orange and the blue fireballs), no `characters`, one `ground_offsets`; **`bursts`** `{spark, zero, block, dust}`: `{"cell", "size" (metres at size 1: 128 / 400), "level" (brightness, the guard's 0.25), "random" (since format 13 the dust's, not the guard's: paths at random), "paths": [[[x, y, z, size, fade]]] (metres from where the effect starts, a frame each)}` (EFFECTS.md 3-4) |
| `pictures/FFF_NNN.png` | the front end's 2D pictures (files 3, 130-132, 134, 136, 141-182, as `textures/` names them: images counted without palettes), each in the palette its code draws it with (FRONTEND.md 1); `003_006_angry.png`-`003_011_angry.png`: the health-bar strips in the angry palettes 0x3D-0x42 |

### `gamedata/roster.json` `front_end` (**new**)

The front end's tables from the executable (docs/FRONTEND.md, by section), written by `src/front.rs`. Pictures are
named by their file in `pictures/`; positions are on the 640 x 448 screen; characters are character ids (0-33),
bodies 0-38 (34-38 Tiger, Panda, Tetsujin, Alex, Angel).

| Key | Content |
|---|---|
| `title` | the title's logo (`134_003`), `press_start` (0x51EED0), `copyright` (0x51EF40, 0x51EF70; the font's `@` written `©`) |
| `fonts.big` | the chrome font `003_001`: `glyph` size [52, 42], its `rows` of glyphs (read off the picture, not the code) |
| `menu.items` | the main menu 0x51EFE0: `mode` (the menu id = the game mode, FRONTEND.md 2), `label`, `locked` (shown only once unlocked: Tekken Bowl, Theater, Gallery) |
| `select` | `titles` by mode (0x51EC18); `slots` (0x440A68, 36: `slot`, neighbours `up`/`down`/`left`/`right` or null, `x`, `y`, `character` from 0x520460, null for RANDOM; a face is drawn at (x, y + `face_dy`)); `hidden_slots` (Mokujin's, Unknown's); `faces` by body (0x520508), `random_face`, `locked_face`, `cursor` (at `cursor_dxy`), `tags` (1P, 2P, 1A-1D, CPU); `selectable` from the start (0x51E850), `copy_pool` (Mokujin's, 0x51E8A0); `portraits` by body (files 143-181) |
| `hud` | `bars` (x of each side, y, w, h), the six `strips` and `angry_strips` (anger from `angry_at`, blinking every `angry_blink` frames), `trail_per_frame` (max / 800, max / 400); `plates` by body (0x5206B8: `picture`, `x` offset, `streak` 0x51F678), `plate_y`, `streaks`; `lamps` (`lit`, `unlit`, x of each side, `dx`, `y`, `blink` [frames, period]); `team_icon`, `chicken`; `timer` (`digits`, `digit_w`, `infinity`); `stage` (STAGE / FINAL texts, alternating every 32 frames) |
| `arcade` | `fights` 8; `rivals` per character (0x51F3C8: three, null = any), `fallback` (0x51F560), `pairs` (0x5207F0: [a, b]); `rival_arena` 6, `boss` 33 alone in `boss_arena` 14 for `boss_rounds` 1; `early_fights` 3 without `early_excluded`; `continue` (`frames` 898, `digit_frames` 100); `time_format` |
| `team_battle` | `members` [1, 8], `refill_percent` 15 (FIGHT_DATA.md 11) |
| `unlocks` | `schedule` (0x440818: `stage`, `kind`, `character`), the `kinds`' meaning, `points_per_stage` 50 |
| `options` | the game options' descriptors (0x439CE0): `label`, `config` (offset in the 0x68-byte config), `default`, `values` |

| `gamedata/movesets/index.json` | as above: its `models` are the characters (`model_record`, `glb` = `model_NNN.glb`), the shape of Tekken 3's |
| `gamedata/movesets/motions_NN.json` | `{"moveset_id", "motions": ["motion_NNNN.glb", ...]}`: the motions moveset NN's moves play |
| `gamedata/sound.json`, `music.json`, `rounds.json` | the sound, music and round-flow tables (below; written by `src/sound.rs` and `fight::rounds`) |
| `audio/sfx/NNN/KKK.wav` | sample KKK of the sound bank in file NNN of TEKKEN.BIN (all 156 banks, 2 350 samples, 16-bit mono, at the rate of its `VAGp` header; 45 MB) |
| `audio/music/NN.flac` | the music stream in file NN (25, 16-bit stereo 44 100 Hz; 197 MB, 291 MB as WAV) |
| `endings/ending_EE_*` | ending EE's scene (EE 00-37, `gamedata/endings.json` names the files; BOWL_ENDINGS.md 7.3): `_camera.bin` its camera path as on the disc (file 2EE + 47: `u32 frames`, then eye xyz, target xyz, H as f32 a frame, game units, y down), `_0.glb` / `_1.glb` the body tracks of actor slots 0 and 1 (file 2EE + 48 part 0) as clips `ending_EE_0` / `ending_EE_1` in `anim::clip_glb`'s shape (the root's track places the actor in the arena), `_face.bin` / `_hands.bin` parts 1 and 2 as on the disc (packs of tracks `u16 frames, u16 triples`, then s16 triples) |
| `audio/index.json` | Tekken 3's shape: `sfx` (a bank each: `bank` its file, `samples`: `file` = `NNN/KKK.wav`, `rate`, `seconds`, `name` from the `VAGp` header; no `tones`: Tekken Tag has no VAB programs), `music` (`name`, `file` = `NN.flac`, `seconds`) |

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
sides are drawn; **new** (format 13) `drum_depth: "floor"`: the PS2 draws the arena with one z-buffer, so the floor
hides the ground below it, unlike the arcade stages' model; `background_rgb` black); node `floor` holds copies `floor_K` of the floor mesh, a period apart, to a
little past the hole in the far ground, and the scene extras' `floor_period` (metres) is that period. An arena's
floors are one: the one of the longest period, each shorter one (2 or 4 m in 8 or 4 m) laid inside it. Only the
opaque, cut-out and ordinarily blended strips are there: the additive ones (lights, floor gloss) and the darkening
ones (shadows, dirt; on Lei's arena the wet street's, so its reflections show bare) are left out, and so are the
background characters and the objects the game moves: since format 13 all of part 3, the object models (the object
table 0x520FC0 places them, the arena's tasks move them: King's helicopters and their rotors, which stood at the
centre). Each primitive (a picture and a blend mode) has its own
positions, texture coordinates and vertex colours; the first image is a white texel, which strips without a texture
draw; each picture is cut to the texels its strips reach (all of it along an axis they repeat it on).

### `gamedata/sound.json`

Tekken 3's keys where the concept is the same, but the numbers are **sound ids**, not Tekken 3's SePlay codes or
sound-list table indices: an entry is `type << 12 | id`, and `id & 0xFFF` is the sound (0 none). The engine needs
this dialect: an id is played as sample `id - first_id` of the group whose range holds it.

- `groups.entries` (**new**, 0x42EBD8, 62): `group`, `first_id`, `count`, `kind` (0 resident, 1 loaded with the
  arena, 2 a voice bank), `bank_bytes`, `bank` (the bank file: `audio/sfx/NNN/`; group 0 is file 1, groups 1-20
  arena k - 1's file 187 + 7k, 21 Tekken Bowl's 327, 22 file 45; null for a voice group), `voice_row` (groups 23-61:
  row group - 23), `samples` (`pitch`: 0x1000 = 48 kHz, the rate the game plays it at, which differs from the WAV's
  for some voices; `volume`; `channel`: classes 6 and 8 take one voice per side).
- `voice_banks.characters` (**new**): each character file's `model_record`, its voice `bank` (the file before it)
  and its `voice_row`: the row whose group has that bank's size and sample count (the game picks the row by player
  +0x2C, the roster's byte 6, not the body: Tiger, Eddy's fifth costume, has row 21).
- `voice_rows.rows` (0x501808, 39): `row`, `counts` (types 2-5 = categories 0-3), `codes` (sound ids:
  `codes[sum(counts[:cat]) + index]`), **new** `cooldown` (10 frames; 6 for rows 21 and 35): a type-2 voice plays
  only when the fighter's cooldown is out, and sets it.
- `entry_types` (**new**): what each entry type does (0x375B60). Type 6, Tekken 3's swing, plays nothing here.
- `impact_rules`: Tekken 3's keys `blocked` (0x7010), `ko` (arm, leg), `auto`, `legs` (by damage band), `arms`
  (`rows` by band, column 0, or 0 / 1 by `rand()` in band 0; **new** `rows_of_bodies` for attacker bodies 5, 7, 10,
  18), **new** `damage_bands` [13, 23, 33] and `ko_arm_end_bone_below`; an arm is a move whose end bone is below 0xC.
  `special_victims`: rows whose hits have their own handlers, not decoded.
- `ko_voice`, `move_sounds` (`take_off`, **new** `footsteps` with `footsteps_gap_frames`, `landing`), **new**
  `arena_sounds` (`by_arena`: [landing, step, step] for arenas 0-20, ids 0x321 / 0x322; `specials`: ids 0x323-0x326
  and the arenas that have them), `scripts` (Tekken 3's `entries`, 668, but each step is `[frame, kind, sound]`),
  `announcer` (Tekken 3's keys; entries, `call_frames`, `result`).

### `gamedata/music.json`

`bgm` (0x50CD00, 28 ids): Tekken 3's `{id, columns, characters}` with **one** column (no arranged / arcade choice):
`file` (`NN.flac`, null for id 0, a sound bank), `loop`, `raw`, **new** `kind`, `source_file`, `start_seconds` (id 7
is the endings' piece of file 20, from sector 3319), `loop_seconds` (the driver's restart point). `by_arena.bgm`
(**new**, 0x4FB9C0): the fight's BGM id of arenas 0-22. The table is what 0x397A00 sends the sound driver
(RSPU2DRV.IRX) at boot, so the id -> file map is read, not guessed: arena themes 9-22 are files 22, 24, 25, 26, 27,
23, 28-35 (not id + 13). Which ids loop: kind 1, from the driver's code (medium confidence).

### `gamedata/rounds.json` (**new**)

The round machine's constants (0x36E198; FIGHT_DATA.md sections 11, 13): `call` (the fight starts on frame 151 / 91,
banner and READY frames), `start_motions` (aliases 4 / 6 in round 1, picked by pad bits or `rand() & 1`, 5 later; the
start frame rule; alias 9 for Unknown), `end` (K.O.: replay from frame 16, played twice, ends at 110, done at 135, or
120 without a replay; time up done at 160; others 600), `win_poses` (aliases 4770-4773, the pick by pad bits, else
the frame counter's parity; the partner poses too; 300 frames), `team_lose` (alias 4780, 60 frames), `result_flags`
(bits, perfect, great when the winner's score is at most 0xCC of 0x1000), `round_time` (frames by option, the
untimed option 5, the displayed second of 70 frames), `rounds_to_win`. The default round-time and rounds options are
null: no code was found that writes them. The pad bits are the game's pad word, not yet tied to buttons.

### `gamedata/endings.json` (**new**)

The endings, the staff roll and Theater (BOWL_ENDINGS.md 7-8). `table` (0x51B678): 34 characters x 8 costumes ->
the ending, -1 none. `endings` (0x51B488, 38): `arena` (the stage's number), `cast` (`[character, costume]`, costume
-1 the player's), `script` (the 10-byte records `[frame, op, a, b, c]` to op -1, as in the executable: records run
in order once their frame has come), `camera`, `body`, `face`, `hands` (the files in `endings/`), `camera_file`,
`pack_file` (their files in TEKKEN.BIN). `staff_roll`: `lines` (0x51B880, 434: `kind`, `text`, `font`,
`alternative`: of the two flagged lines the first shows until Tekken Bowl is unlocked), `cards`, `card_frames`,
`frames` (the roll alone before the scene), `scroll_px`, `line_px`, `start_y`, `bgm`. `playback`: `ending_bgm`,
`fade_in`, `skip_fade`, `max_frames`, `unknown` (Unknown's ending, which plays the movie `unknown_movie`). `theater`:
`labels` (0x455E50: per character 8 costume labels or null), `music` (0x455D80: the music player's `{bgm, name}` to
EXIT, 254), `extras` (the movies and the EMBU entries after the rows). The constants are the code's, read in
BOWL_ENDINGS.md 7.5, not tables.

### `gamedata/bowl.json` (**new**)

Tekken Bowl (BOWL_ENDINGS.md sections 1-6; `src/bowl.rs`), in the game's units and axes (y down, z down the lane):
`characters` (34, by FE id, from 0x523618, 0x523590, 0x5236A0: `s` the stat S, `left`, `female` the charge's style,
`reactions` four strike reactions as bowl motions, `voices` two throw voices and the charge's, as sound entries),
`copied` (Mokujin and Unknown bowl with another's stats), `strike_poses` (by moveset, 0x5237D8), `motions` (`frames`
of each `bowl_NNN`; by hand: stand, step right, step left, throw, + 4 for a left-hander; the charges 0x6A / 0x6E, + 2
for the female style, + 1 left-handed), `throw` (the phases' steps, limits, the meters' bounds and speeds, the release
at frame 106), `charge`, `ball` (start, radius, the lane's height, the fall, the bounce, the hook's start and length,
where a roll ends), `pins` (`y` the centre's height, `rack` the ten spots (x, z), the side, gravity, contact heights,
the down test's radius^2, the deck walls), `spectators` (the test's radius^2, the K.O.'s frames and voice, `people`:
the 25 beside the lane, of the arena's cast), `speed` (the MPH display), `sounds` (sound entries: the first touch,
the pin clatter by level, the strike announcer by run), `callouts` (the texts: STRIKE! .. !!GREAT!!, PUNCH OUT, PERFECT,
MISS, FOUL, SPARE, NICE COVER from the executable; SPLIT, K.O., OVER CHARGED), `split_gap`, `camera` (the decoded
bases of modes 0-2), `models` (the ball's, pin's and deck's glbs, the pin's centre above its base). The code-read
rules are written as read (strings where they are formulas); the tables are read from the executable.


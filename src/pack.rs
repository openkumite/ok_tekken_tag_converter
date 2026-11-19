//! The pack: the disc's fighters, arenas, motions, pictures and sounds converted ([`convert`]).
//!
//! A pack (any `Vfs`: a folder, or the browser's pack file):
//!
//! ```text
//! manifest.json        {"pack": "ttt", "format": FORMAT, "source": {"image", "disc": "SLUS-20001", "version": "2.00"},
//!                       "models", "movesets", "own", "anims", "stages", "textures", "sounds", "music", "video",
//!                       "raw", "gamedata": the index of the files below (README.md)}
//! models/NNN.glb       the fighters, skinned, with their textures (NNN: the file's number in TEKKEN.BIN)
//! anims/NNNN.glb       the motions, one animation each, by joint name
//! stages/NNN.glb       the arenas; NNN_pK.glb their background characters, NNN_mK.glb those characters' motions
//! textures/NNN_KKK.png the pictures
//! audio/NNN_KKK.wav    the sounds; audio/NNN.wav the music, and the movies' sound
//! video/NNN.pss        the movies' pictures as on the disc (MPEG-2 program stream)
//! raw/NNN.bin          files and packs not (fully) decoded, as on the disc, unscrambled
//! gamedata/*.json      the fight data: moves, cancels, hits, reactions, rules, movesets/ (docs/GAMEDATA.md)
//! assets/              what OpenKumite reads, laid out as it reads Tekken 3's (`assets`, docs/GAMEDATA.md "assets/"):
//!   manifest.json             {"game": "ttt", "format": assets::FORMAT, "fingerprint"}
//!   characters/model_NNN.glb  the roster's fighters (NNN: the file's number in TEKKEN.BIN, as models/)
//!   motions/motion_NNNN.glb   the motions the characters' movesets play, a clip `motion_NNNN` each (NNNN: as anims/);
//!                             bowl_NNN.glb Tekken Bowl's bowler motions (file 128 part 0), clips `bowl_NNN`
//!   stages/stage_NN.glb       the arenas (NN: the arena's number), stages/index.json; bowl_ball.glb, bowl_pin.glb,
//!                             bowl_deck.glb the bowling hall's ball, pin and pin deck
//!   gamedata/                 the fight data again (moves' `anim`: their clip), roster.json (with the front end's
//!                             tables, `front_end`); sound.json, music.json, rounds.json (the sound, music and
//!                             round-flow tables)
//!   pictures/FFF_NNN.png      the front end's 2D pictures (as textures/, the files front_end names), and
//!                             003_NNN_angry.png the angry health-bar strips; effects/ the hit and move effects'
//!                             atlas.png and effects.json
//!   gamedata/shadows.json, stages.json  the fighters' shadow meshes, each arena's shadow (docs/EFFECTS.md)
//!   audio/sfx/NNN/KKK.wav     the sound banks' samples (NNN: the bank's file in TEKKEN.BIN); audio/music/NN.flac the
//!                             music streams (NN: their file); audio/index.json
//!   video/NN.avi, NN.flac     the movies (NN: their file, 14 the opening, 15 the arcade's, 16 Unknown's ending): their
//!                             pictures as Motion-JPEG at their own size and rate, their sound (`movie`)
//!   endings/ending_EE_*       ending EE's scene (gamedata/endings.json names the files): its camera path
//!                             (`_camera.bin`), its actor slots' body tracks as clips `ending_EE_K` (`_K.glb`), its face
//!                             and hand tracks (`_face.bin`, `_hands.bin`) as on the disc
//! ```
//!
//! The layout outside `assets/` is the research tool's, to look at in any glTF viewer.

use crate::archive::{self, Archive};
use crate::{
    anim, assets, audio, bowl, effects, ending, fight, front, model, moves, movie, objects, sound, stage, tex, u32le,
};
use anyhow::{Context, Result, bail};
use ok_converter_common::io::{Ctx, DiscFile, Sub, Vfs};
use ok_converter_common::psx;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

/// The pack's layout and contents; another one is converted again (3: `assets/`; 4: every section of a fighter; 5:
/// the air reactions, the default life, the damage constants; 6: `assets/audio/`, sound, music and rounds; 7: the
/// camera, the camera id from bit 23; 8: ai.json; 9: the front end, `assets/pictures/` and roster.json `front_end`, the
/// 2D files' palettes as drawn; 10: the endings, `assets/endings/` and gamedata/endings.json; 11: Tekken Bowl, `bowl.json`,
/// its motions, ball, pin and deck; 12: the shadows and effects, `shadows.json`, `stages.json`, `pictures/effects/`;
/// 14: the arenas' objects, placed and moved, and their added and multiplying strips, in their stages; 15: the front
/// end's motion, its sounds per action, the font's colours; 16: the select's 3D characters; 17: the front end's
/// sounds and music, the small font's colours; 18: the fight HUD as drawn, its round texts and the chrome font's advances;
/// 19: ai.json `survival`; 20: Yoshimitsu's CPU handler in ai.json; 21: the movies, `assets/video/`, and the EMBU,
/// endings.json `embu`; 22: the arenas' background people, the lens flare, faces to the camera; 23: bowl.json `replay`,
/// `strike`, `records`, `texts`).
pub const FORMAT: i64 = 23;
/// What a pack's manifest names it.
pub const KIND: &str = "ttt";

const FIGHTERS: usize = 345; // the fighter of costume slot s is file 345 + 3 * s
const ARENAS: usize = 189; // the geometry of arena n is file 189 + 7 * n
const BOWL: usize = 329; // the bowling hall: not in the stage list, and its lane stands at the centre for good
const MOTIONS: (usize, usize) = (133, 10); // the motions' two banks

/// The innermost chunks of nested packs (the data itself when it is not a pack).
fn leaves(d: &[u8]) -> Vec<&[u8]> {
    // (the disc's packs nest three deep)
    fn at(d: &[u8], depth: u32) -> Vec<&[u8]> {
        match archive::pack(d).filter(|_| depth < 8) {
            Some(p) => p.into_iter().flat_map(|c| at(c, depth + 1)).collect(),
            None => vec![d],
        }
    }
    at(d, 0)
}

type Arena = (Vec<stage::Batch>, Vec<stage::Batch>, Vec<stage::Floor>, Vec<Vec<tex::Upload>>);

/// An arena from its geometry pack and the texture pack before it. The n-th part of one goes with
/// the n-th part of the other (empty parts aside): the banks all load at the same GS addresses.
fn arena(geometry: &[u8], textures: &[u8]) -> Option<Arena> {
    let full = |d| archive::pack(d).map(|p| p.into_iter().filter(|c| !c.is_empty()).collect::<Vec<_>>());
    let (g, t) = (full(geometry)?, full(textures).unwrap_or_default());
    let parts: Vec<(usize, &[u8])> =
        g.iter().enumerate().flat_map(|(k, c)| leaves(c).into_iter().map(move |l| (k, l))).collect();
    let load = |(k, c): (usize, &&[u8])| {
        let loads = leaves(c).into_iter().enumerate();
        loads
            .flat_map(move |(l, d)| {
                tex::uploads(d)
                    .into_iter()
                    .map(move |u| tex::Upload { set: u.set + ((k as u64) << 24 | (l as u64) << 16), ..u })
            })
            .collect()
    };
    let mut banks: Vec<Vec<tex::Upload>> = t.iter().enumerate().map(load).collect();
    if banks.len() != g.len() {
        // not the usual layout: one pool for every part
        // shortcut: the pool is copied for each part (64 at most); share it if an arena's textures ever weigh much
        let own = parts.iter().enumerate().flat_map(|(k, p)| {
            tex::uploads(p.1).into_iter().map(move |u| tex::Upload { set: u.set + ((k as u64 + 1) << 32), ..u })
        });
        let all: Vec<tex::Upload> = banks.into_iter().flatten().chain(own).collect();
        banks = (0..g.len()).map(|_| all.clone()).collect();
    }
    // part 3 (counted with the empty ones) holds the arena's object models, which the object table (0x520FC0) places
    // and the arena's tasks move (King's arena: the helicopters and their rotors): not scenery at the origin
    let raw = archive::pack(geometry)?;
    let object = raw.get(3).filter(|c| !c.is_empty()).map(|_| raw.iter().take(3).filter(|c| !c.is_empty()).count());
    let (models, parts): (Vec<_>, Vec<_>) = parts.into_iter().partition(|p| Some(p.0) == object);
    let (scenery, mut objects, floors) = stage::arena(&parts);
    for (k, d) in models {
        objects.extend(stage::batches(d).into_iter().map(|b| stage::Batch { bank: k, ..b }));
    }
    Some((scenery, objects, floors, banks))
}

/// Converts the Tekken Tag Tournament disc (its data track `src`) into a pack in `ctx.out`, emptied first; writes
/// the manifest last and returns it. Without `research`, only `assets/` and the manifest (with the source alone):
/// what OpenKumite plays, a third of the size.
pub fn convert(ctx: &Ctx, src: &dyn DiscFile, research: bool) -> Result<Value> {
    let out = ctx.out;
    let mut a = Archive::open(src)?;
    out.remove_all("")?;
    let assets = Sub { inner: out, prefix: "assets".into() };
    let (mut characters, mut arenas) = (vec![], vec![]);
    let (mut sounds, mut music, mut video, mut raw, mut models, mut textures, mut stages) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    // OpenKumite's audio/index.json, and each bank's (file, bytes, samples) and each music stream's file
    let (mut sfx, mut tracks, mut banks, mut streams, mut movies) = (vec![], vec![], vec![], vec![], vec![]);
    // where the select's music loops from (seconds)
    let mut select_loop = None;
    let roster = moves::roster(&a.elf)?;
    // file 3's uploads: the HUD and select atlas, whose pictures the front end's tables name
    let mut atlas = vec![];
    // the fighters' shadow meshes, and the effect files' pictures (files 4-8)
    let (mut shadows, mut effect_pics, mut effect_paths) = (vec![], vec![vec![]; 5], vec![vec![]; 4]);
    ctx.log(&format!("[ttt] {}: the {} files of TEKKEN.BIN", src.name(), archive::FILES));
    // (the first and the last file are random filler)
    for i in 1..archive::FILES - 1 {
        let d = a.read(i)?;
        if d.is_empty() {
            continue;
        }
        let plain = a.toc.get(i).is_some_and(|e| e.plain);
        let pack = archive::pack(&d);
        let mut unknown = false;
        if d.starts_with(b"VAGp") {
            let (mut items, mut samples) = (vec![], vec![]);
            let bank = audio::vag_bank(&d);
            // the select's music (BGM 0, a sound bank: its left and right halves, looped by the SPU) as a stream too
            if let [(_, 44100, l, Some(start)), (_, 44100, r, _)] = &bank[..] {
                let pcm = l.iter().zip(r).flat_map(|(a, b)| [*a, *b]).collect();
                let flac = format!("{i:02}.flac");
                assets.write(
                    &format!("audio/music/{flac}"),
                    &psx::flac(&psx::Pcm { rate: 44100, channels: 2, samples: pcm })?,
                )?;
                tracks
                    .push(json!({"name": format!("bank file {i}"), "file": flac, "seconds": l.len() as f64 / 44100.0}));
                streams.push(i);
                select_loop = Some(*start as f64 / 44100.0);
            }
            for (k, (name, rate, pcm, _)) in bank.into_iter().enumerate() {
                let (file, wav, seconds) =
                    (format!("audio/{i:03}_{k:03}.wav"), audio::wav(rate, 1, &pcm), pcm.len() as f64 / rate as f64);
                if research {
                    out.write(&file, &wav)?;
                }
                // OpenKumite's: audio/sfx/NNN/KKK.wav, a folder a bank
                let sample = format!("{i:03}/{k:03}.wav");
                assets.write(&format!("audio/sfx/{sample}"), &wav)?;
                samples.push(json!({"file": sample, "rate": rate, "seconds": seconds, "name": name}));
                items.push(json!({"name": name, "file": file, "seconds": seconds}));
            }
            banks.push((i, d.len(), items.len()));
            sfx.push(json!({"bank": i, "samples": samples}));
            sounds.push(json!({"id": i, "sounds": items}));
        } else if plain {
            let (pcm, pss) = audio::stream(&d);
            let wav = format!("audio/{i:03}.wav");
            if research {
                out.write(&wav, &audio::wav(44100, 2, &pcm))?;
            }
            let seconds = pcm.len() as f64 / 88200.0;
            if pss.is_empty() {
                music.push(json!({"id": i, "file": wav, "seconds": seconds}));
                // OpenKumite's: audio/music/NN.flac
                let flac = format!("{i:02}.flac");
                assets.write(
                    &format!("audio/music/{flac}"),
                    &psx::flac(&psx::Pcm { rate: 44100, channels: 2, samples: pcm })?,
                )?;
                tracks.push(json!({"name": format!("stream file {i}"), "file": flac, "seconds": seconds}));
                streams.push(i);
            } else {
                let file = format!("video/{i:03}.pss");
                if research {
                    out.write(&file, &pss)?;
                }
                video.push(json!({"id": i, "file": file, "sound": wav, "seconds": seconds}));
                // OpenKumite's: video/NN.avi and NN.flac
                let (avi, frames, fps, errors) = movie::avi(&pss).with_context(|| format!("movie {i}"))?;
                if errors > 0 {
                    ctx.log(&format!(
                        "[ttt] movie {i}: {errors} damaged pictures (concealed): the image is not a good copy"
                    ));
                }
                assets.write(&format!("video/{i:02}.avi"), &avi)?;
                assets.write(
                    &format!("video/{i:02}.flac"),
                    &psx::flac(&psx::Pcm { rate: 44100, channels: 2, samples: pcm })?,
                )?;
                let name = ["opening", "arcade", "Unknown's ending"].get(i.wrapping_sub(14)).unwrap_or(&"movie");
                movies.push(json!({"name": name, "file": format!("{i:02}.avi"), "audio_file": format!("{i:02}.flac"),
                    "fps": fps, "frames": frames, "seconds": seconds}));
            }
        } else if let Some((m, ups)) = pack.as_ref().and_then(|p| {
            let m = p.iter().find_map(|c| model::parse(c))?;
            Some((m, p.iter().flat_map(|c| tex::uploads(c)).collect::<Vec<_>>()))
        }) {
            let file = format!("models/{i:03}.glb");
            if research {
                out.write(&file, &m.glb(&ups))?;
            }
            let tris: usize = m.sections.iter().flatten().map(|s| s.verts.len().saturating_sub(2)).sum();
            let slot = i.checked_sub(FIGHTERS).filter(|s| s % 3 == 0).map(|s| s / 3);
            let who = slot.map(|s| moves::fighter(&a.elf, s)).transpose()?.flatten();
            let (name, moveset) = match who.and_then(|(body, costume)| Some((roster.get(body)?, costume))) {
                Some(((name, set), costume)) => {
                    (format!("{name} · costume {}", costume + 1), Some(*set).filter(|&m| m < moves::MOVESETS))
                }
                None => (format!("Extra {i}"), None),
            };
            // OpenKumite's character: a fighter of the roster, with its moveset
            if let (Some((body, costume)), Some(set)) = (who, moveset) {
                match m.fighter_glb(&ups) {
                    Some(glb) => {
                        let file = format!("model_{i:03}.glb");
                        assets.write(&format!("characters/{file}"), &glb)?;
                        characters.push(json!({"id": i, "name": name, "body": body, "costume": costume, "moveset": set, "file": file}));
                        // its shadow: the low-detail model, part 8
                        if let Some(low) = pack.as_ref().and_then(|p| model::parse_low(p.get(8)?)) {
                            shadows.push(effects::shadow(&low, i));
                        }
                    }
                    None => ctx.log(&format!("[ttt] {name} (file {i}): not a fighter OpenKumite can read")),
                }
            }
            models.push(json!({"id": i, "name": name, "body": who.map(|w| w.0), "moveset": moveset, "file": file, "nodes": m.nodes.len(), "triangles": tris}));
        } else if let Some((mut b, mut objects, floors, banks)) = pack
            .as_ref()
            .map(|_| a.read(i - 1))
            .transpose()?
            .and_then(|t| arena(&d, &t))
            .filter(|b| b.0.len() + b.2.iter().map(|f| f.tiles.len() * f.at.len()).sum::<usize>() > 20)
        {
            let mut bare = b.clone();
            b.extend(floors.iter().flat_map(|f| f.laid()));
            if i == BOWL {
                // OpenKumite's: its objects apart, the ball, the pin and the pin deck where the object table puts
                // them; the object part (taken for scenery and objects, all at the origin) not in its stage
                let part = archive::pack(&d).map(|p| p.into_iter().filter(|c| !c.is_empty()).collect::<Vec<_>>());
                let models = part.as_ref().and_then(|p| p.get(bowl::OBJECT_PART)).map(|m| stage::blocks(m));
                let [ball, pin, deck] = bowl::models(&a.elf, &models.unwrap_or_default())?;
                bare.retain(|x| x.bank != bowl::OBJECT_PART);
                for (name, m) in [("ball", ball), ("pin", pin), ("deck", deck)] {
                    assets.write(&format!("stages/bowl_{name}.glb"), &stage::game_glb(name, &m, &[], &[], &banks))?;
                }
                b.append(&mut objects);
            }
            let file = format!("stages/{i:03}.glb");
            if research {
                out.write(&file, &stage::glb(&b, &objects, &banks))?;
            }
            // an arena of the stage list, by its number there
            let listed = i.checked_sub(ARENAS).filter(|k| k % 7 == 0).map(|k| k / 7);
            // the arena's background characters: part 0 is a pack of their motions and their models (OpenKumite's:
            // `stages/stage_NN_pK.glb`, K the model, as characters; their motions `stage_NN_mK.glb`, clips of that name)
            let (mut people, mut motions) = (vec![], vec![]);
            let cast = pack.as_ref().filter(|_| research || listed.is_some()).and_then(|p| archive::pack(p.first()?));
            let cast = cast.unwrap_or_default();
            for (k, c) in cast.iter().enumerate().skip(1) {
                let Some(m) = model::parse_low(c) else { continue };
                let file = format!("stages/{i:03}_p{k}.glb");
                // model k's textures are the k-th load of texture part 0 (every load uses the same addresses)
                let own: Vec<tex::Upload> =
                    banks.iter().flatten().filter(|u| u.set >> 16 == k as u64 - 1).cloned().collect();
                if research {
                    out.write(&file, &m.glb(&own))?;
                }
                if let (Some(n), Some(glb)) = (listed, m.fighter_glb(&own)) {
                    assets.write(&format!("stages/stage_{n:02}_p{}.glb", k - 1), &glb)?;
                }
                people.push(json!({"id": k - 1, "file": file}));
            }
            if let Some(index) = cast.first().filter(|_| !people.is_empty()) {
                // their motions: a pack of unpacked ones, or (offset, size) pairs before packed ones
                let unpacked = archive::pack(index);
                let count = unpacked
                    .as_ref()
                    .map_or((u32le(index, 0).unwrap_or(0) as usize / 8).min(index.len() / 8), |p| p.len());
                for k in 0..count {
                    let frames = match &unpacked {
                        Some(p) => p.get(k).and_then(|m| anim::raw(m)),
                        None => u32le(index, 8 * k).and_then(|o| anim::decode(index, o as usize, true)),
                    };
                    let Some(frames) = frames else { continue };
                    let file = format!("stages/{i:03}_m{k}.glb");
                    if research {
                        out.write(&file, &anim::glb(&frames))?;
                    }
                    if let Some(n) = listed {
                        let name = format!("stage_{n:02}_m{k}");
                        assets.write(&format!("stages/{name}.glb"), &anim::clip_glb(&name, &frames))?;
                    }
                    motions.push(json!({"id": k, "file": file, "frames": frames.len()}));
                }
            }
            let name = match listed.map(|n| moves::arena(&a.elf, n)).transpose()?.flatten() {
                Some(name) => name,
                None if i == BOWL => "Tekken Bowl".into(),
                None => format!("Arena {}", stages.len() + 1),
            };
            let cast = match listed.filter(|_| !people.is_empty()) {
                Some(n) => moves::cast(&a.elf, n)?,
                None => vec![],
            };
            let cast: Vec<Value> = (cast.iter())
                .map(|p| json!({"model": p.model, "pos": p.pos.map(|v| (v as f64 * model::SCALE) as f32), "heading": p.heading, "motions": p.motions, "start": p.start}))
                .collect();
            // OpenKumite's stage, by the arena's number
            if let Some(n) = listed {
                let file = format!("stage_{n:02}.glb");
                // its objects (part 3's models) as its code places and moves them
                // (part 3 counted with the empty parts, its textures' bank without them, as `arena` takes them)
                let raw = archive::pack(&d).unwrap_or_default();
                let bank = raw.iter().take(3).filter(|c| !c.is_empty()).count();
                let blocks: Vec<Vec<stage::Batch>> = (raw.get(3).map(|m| stage::blocks(m)).unwrap_or_default())
                    .into_iter()
                    .map(|m| m.into_iter().map(|b| stage::Batch { bank, ..b }).collect())
                    .collect();
                let things = if i == BOWL { vec![] } else { objects::objects(&a.elf, n, &blocks)? };
                assets.write(&format!("stages/{file}"), &stage::game_glb(&name, &bare, &floors, &things, &banks))?;
                arenas.push(json!({"id": n, "name": name, "file": file}));
            }
            let triangles = b.iter().map(|x| x.pos.len().saturating_sub(2)).sum::<usize>();
            stages.push(json!({"id": i, "name": name, "file": file, "people": people, "motions": motions, "cast": cast, "triangles": triangles}));
        } else if let Some(pics) = Some(
            leaves(&d)
                .iter()
                .flat_map(|c| tex::pictures(&tex::uploads(c), &|k| front::palette(i, k)))
                .collect::<Vec<_>>(),
        )
        .filter(|p| !p.is_empty())
        {
            let mut images = vec![];
            for (k, (w, h, px)) in pics.iter().enumerate() {
                let file = format!("textures/{i:03}_{k:03}.png");
                let png = tex::png(*w, *h, px);
                if research {
                    out.write(&file, &png)?;
                }
                // OpenKumite's: the front end's pictures (front_end names them)
                if front::is_2d(i) {
                    assets.write(&format!("pictures/{i:03}_{k:03}.png"), &png)?;
                }
                images.push(file);
            }
            if let Some(e) = i.checked_sub(4).and_then(|k| effect_pics.get_mut(k)) {
                *e = pics.clone();
            }
            // file 9: the lens flare's discs (EFFECTS.md 5), opaque: the GS adds them by their colour alone
            if i == 9 {
                for (k, (w, h, px)) in pics.iter().enumerate() {
                    let px: Vec<u8> = px.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, _]| [r, g, b, 255]).collect();
                    assets.write(&format!("pictures/effects/flare_{k}.png"), &tex::png(*w, *h, &px))?;
                }
            }
            if let Some(e) = i.checked_sub(4).and_then(|k| effect_paths.get_mut(k)) {
                *e = effects::paths(&d);
            }
            if i == 3 {
                atlas = tex::uploads(&d);
                for (name, png) in front::angry_strips(&atlas)
                    .into_iter()
                    .chain(front::cursor_2p(&atlas))
                    .chain(front::font_colours(&atlas))
                {
                    assets.write(&format!("pictures/{name}"), &png)?;
                }
            }
            textures.push(json!({"id": i, "images": images}));
        } else {
            unknown = true;
        }
        // kept as they are: files nothing was converted from, and packs (their other parts are not decoded yet)
        if research && !plain && (unknown || pack.is_some()) {
            let file = format!("raw/{i:03}.bin");
            out.write(&file, &d)?;
            raw.push(json!({"id": i, "file": file, "size": d.len(), "parts": pack.map(|p| p.iter().map(|c| c.len()).collect::<Vec<_>>())}));
        }
    }
    ctx.log(&format!(
        "[ttt] {} fighters, {} arenas, {} files of pictures, {} sound banks, {} music tracks, {} movies, {} files kept raw",
        models.len(),
        stages.len(),
        textures.len(),
        sounds.len(),
        music.len(),
        video.len(),
        raw.len()
    ));
    // motions: the first bank starts with (offset, size) pairs; offsets from 0x4000000 are in the second
    let (bank, bank2) = (a.read(MOTIONS.0)?, a.read(MOTIONS.1)?);
    // several moves share one motion: `first` is the number its file is written under
    let (mut anims, mut first) = (vec![], HashMap::new());
    // each motion decoded, by its first move's number
    let mut decoded: Vec<Option<Vec<anim::Frame>>> = vec![];
    let count = (u32le(&bank, 8).context("the motions' bank is empty")? as usize / 8).min(bank.len() / 8);
    let offsets: Vec<u32> = (0..count).map_while(|k| u32le(&bank, 8 * k)).collect();
    let canon: Vec<usize> = offsets.iter().enumerate().map(|(k, &o)| *first.entry(o).or_insert(k)).collect();
    for (k, &o) in offsets.iter().enumerate() {
        if canon.get(k) != Some(&k) {
            continue;
        }
        let frames = match (o as usize).checked_sub(0x400_0000) {
            Some(o) => anim::decode(&bank2, o, false),
            None => anim::decode(&bank, o as usize, false),
        };
        let Some(frames) = frames else { continue };
        let file = format!("anims/{k:04}.glb");
        if research {
            out.write(&file, &anim::glb(&frames))?;
        }
        anims.push(json!({"id": k, "file": file, "frames": frames.len()}));
        decoded.resize_with(k + 1, || None);
        if let Some(d) = decoded.get_mut(k) {
            *d = Some(frames);
        }
    }
    ctx.log(&format!("[ttt] {} motions", anims.len()));
    let mut movesets = Vec::new();
    for m in 0..moves::MOVESETS {
        let mut seen = HashSet::new();
        movesets.push(
            moves::moveset(&a.elf, m)?
                .into_iter()
                .filter_map(|v| canon.get(v).copied())
                .filter(|&v| seen.insert(v))
                .collect::<Vec<usize>>(),
        );
    }
    // a fighter's own motions first: those fewer than half of the movesets use (the rest are the
    // reactions, falls and throws everybody shares); `own` is how many of them lead each list
    let mut users: HashMap<usize, usize> = HashMap::new();
    for v in movesets.iter().flatten() {
        *users.entry(*v).or_default() += 1;
    }
    let shared = |v: &usize| users.get(v).is_some_and(|n| n * 2 > moves::MOVESETS);
    let mut own = Vec::new();
    for m in &mut movesets {
        m.sort_by_key(shared);
        own.push(m.iter().filter(|v| !shared(v)).count());
    }
    models.sort_by_key(|m| (m["body"].as_u64().unwrap_or(99), m["name"].as_str().unwrap_or("").to_string()));
    // the fight data: a move's motion is the one of its number
    let written: HashSet<u64> = anims.iter().filter_map(|a| a["id"].as_u64()).collect();
    let file = |k: &usize| written.contains(&(*k as u64)).then(|| format!("anims/{k:04}.glb"));
    let files: Vec<Option<String>> = canon.iter().map(file).collect();
    let frames: Vec<u8> = (offsets.iter())
        .map(|&o| match (o as usize).checked_sub(0x400_0000) {
            Some(o) => bank2.get(o),
            None => bank.get(o as usize),
        })
        .map(|f| f.copied().unwrap_or(0))
        .collect();
    let gamedata = if research { fight::export(out, &a.elf, &files, &frames, &models)? } else { vec![] };
    ctx.log(&format!("[ttt] the fight data: {} files", gamedata.len()));
    // OpenKumite's motions: each one its characters' movesets play, once (a file a moveset would be 869 MB), and
    // each moveset's list of them
    let motion = |v: usize| canon.get(v).copied().filter(|&c| decoded.get(c).is_some_and(Option::is_some));
    let sets: BTreeSet<u64> = characters.iter().filter_map(|c| c["moveset"].as_u64()).collect();
    let mut used = BTreeSet::new();
    for &m in &sets {
        let own: BTreeSet<usize> = moves::moveset(&a.elf, m as usize)?.into_iter().filter_map(motion).collect();
        let files: Vec<String> = own.iter().map(|c| format!("motion_{c:04}.glb")).collect();
        let list = json!({"moveset_id": m, "motions": files});
        assets.write(&format!("gamedata/movesets/motions_{m:02}.json"), serde_json::to_string(&list)?.as_bytes())?;
        used.extend(own);
    }
    for &c in &used {
        if let Some(Some(f)) = decoded.get(c) {
            assets.write(&format!("motions/motion_{c:04}.glb"), &anim::clip_glb(&format!("motion_{c:04}"), f))?;
        }
    }
    let clips: Vec<Option<String>> = (0..offsets.len()).map(|v| motion(v).map(|c| format!("motion_{c:04}"))).collect();
    fight::export(&assets, &a.elf, &clips, &frames, &characters)?;
    // Tekken Bowl's bowler motions (file 128 part 0: unpacked, as the arenas' people's) as clips `bowl_NNN`, and
    // their frame counts for bowl.json (0: a track not read)
    let scene = a.read(bowl::MOTIONS)?;
    let bowler = archive::pack(&scene).and_then(|p| archive::parts(p.first()?, 128)).unwrap_or_default();
    let mut lengths = vec![];
    for (k, t) in bowler.iter().enumerate() {
        let f = anim::raw(t).unwrap_or_default();
        if !f.is_empty() {
            assets.write(&format!("motions/bowl_{k:03}.glb"), &anim::clip_glb(&format!("bowl_{k:03}"), &f))?;
        }
        lengths.push(f.len());
    }
    for (name, v) in [
        ("bowl", bowl::bowl(&a.elf, &lengths)?),
        ("sound", sound::sound(&a.elf, &banks, &characters)?),
        ("music", sound::music(&a.elf, &streams, select_loop)?),
        ("rounds", fight::rounds()),
    ] {
        assets.write(&format!("gamedata/{name}.json"), serde_json::to_string(&v)?.as_bytes())?;
    }
    assets.write("gamedata/shadows.json", serde_json::to_string(&json!({"meshes": shadows}))?.as_bytes())?;
    assets.write("gamedata/stages.json", serde_json::to_string(&effects::stages(&a.elf))?.as_bytes())?;
    match effects::atlas(&effect_pics, &effect_paths) {
        Some((png, json)) => {
            assets.write("pictures/effects/atlas.png", &png)?;
            assets.write("pictures/effects/effects.json", serde_json::to_string(&json)?.as_bytes())?;
        }
        None => ctx.log("[ttt] the effect files' pictures are not as expected: no hit effects"),
    }
    assets.write(
        "audio/index.json",
        serde_json::to_string(&json!({"sfx": sfx, "music": tracks, "video": movies}))?.as_bytes(),
    )?;
    // the endings and the EMBU: their tables, and each scene's files (the EMBU's camera file holds its 22 paths)
    let mut endings = ending::endings(&a.elf)?;
    let scenes =
        (0..ending::COUNT).map(|e| (format!("ending_{e:02}"), ending::files(e))).chain([("embu".into(), ending::EMBU)]);
    for (name, (camera, pack)) in scenes {
        let (camera, pack) = (a.read(camera)?, a.read(pack)?);
        assets.write(&format!("endings/{name}_camera.bin"), &camera)?;
        let clips = ending::body_clips(&name, &pack);
        for (k, glb) in clips.iter().enumerate() {
            assets.write(&format!("endings/{name}_{k}.glb"), glb)?;
        }
        if let (true, Some(e)) = (name == "embu", endings.get_mut("embu").and_then(Value::as_object_mut)) {
            e.insert("body".into(), json!((0..clips.len()).map(|k| format!("embu_{k}.glb")).collect::<Vec<_>>()));
        }
        for (k, part) in [(1, "face"), (2, "hands")] {
            if let Some(p) = ending::part(&pack, k) {
                assets.write(&format!("endings/{name}_{part}.bin"), p)?;
            }
        }
    }
    assets.write("gamedata/endings.json", serde_json::to_string(&endings)?.as_bytes())?;
    let mut roster = assets::roster(&roster, &characters);
    if let Some(r) = roster.as_object_mut() {
        r.insert("front_end".into(), front::front_end(&a.elf, &atlas, &clips)?);
    }
    assets.write("gamedata/roster.json", serde_json::to_string(&roster)?.as_bytes())?;
    assets.write("stages/index.json", serde_json::to_string(&arenas)?.as_bytes())?;
    let fingerprint = assets::fingerprint(&assets)?;
    let engine = json!({"game": assets::GAME, "format": assets::FORMAT, "fingerprint": fingerprint});
    assets.write("manifest.json", serde_json::to_string_pretty(&engine)?.as_bytes())?;
    ctx.log(&format!(
        "[ttt] OpenKumite's assets: {} characters, {} motions, {} stages",
        characters.len(),
        used.len(),
        arenas.len()
    ));
    let source = json!({"image": src.name(), "disc": "SLUS-20001", "version": "2.00"});
    let manifest = match research {
        true => {
            json!({"pack": KIND, "format": FORMAT, "source": source, "models": models, "movesets": movesets, "own": own,
            "anims": anims, "stages": stages, "textures": textures, "sounds": sounds, "music": music, "video": video, "raw": raw, "gamedata": gamedata})
        }
        false => json!({"pack": KIND, "format": FORMAT, "source": source}),
    };
    out.write("manifest.json", serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    out.checkpoint(&[])?;
    Ok(manifest)
}

/// The manifest of the pack in `pack`, checked: a Tekken Tag Tournament pack of this format.
pub fn manifest(pack: &dyn Vfs) -> Result<Value> {
    let m: Value = serde_json::from_slice(&pack.read("manifest.json")?).context("the pack's manifest.json")?;
    if m.get("pack").and_then(Value::as_str) != Some(KIND) {
        bail!("not a Tekken Tag Tournament pack");
    }
    if m.get("format").and_then(Value::as_i64) != Some(FORMAT) {
        bail!("a Tekken Tag Tournament pack of another version: convert the disc again");
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_of_packs_nested_without_end_stop() {
        // a pack whose one part is the 16 bytes after its header: read as a pack, zeros are not one
        let mut d = Vec::new();
        for v in [1u32, 16, 16, u32::MAX] {
            d.extend(v.to_le_bytes());
        }
        d.extend([0; 16]);
        assert_eq!(leaves(&d), [&[0u8; 16][..]]);
        assert_eq!(leaves(b"abc"), [&b"abc"[..]]);
        // 50 packs one in another: the walk stops 8 deep, with a pack as its leaf
        let mut deep = vec![7u8; 16];
        for _ in 0..50 {
            let mut outer = Vec::new();
            for v in [1u32, 16, deep.len() as u32, u32::MAX] {
                outer.extend(v.to_le_bytes());
            }
            outer.extend(&deep);
            deep = outer;
        }
        assert_eq!(leaves(&deep).len(), 1);
        assert!(archive::pack(leaves(&deep).first().copied().unwrap_or_default()).is_some());
    }
}

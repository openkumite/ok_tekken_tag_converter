//! The pack end to end. Without the disc: an image of zeros, and a disc of another game, give an error and leave
//! the output as it was. With it (`TTT_DISC`: the image, in any format the converter reads): 133 fighters, 23
//! arenas, 4291 motions, 2350 sounds, every file the manifest names, the fight data's known facts (docs/FIGHT_DATA.md);
//! OpenKumite's `assets/`, a character and motions read as its loader reads them; converted twice, the same bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use ok_converter_common::disc::Disc;
use ok_converter_common::io::{Ctx, DiscFile, Entry, ReadSeek, Vfs};
use ok_tekken_tag_converter::pack;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hasher};
use std::sync::Mutex;

/// A disc image in memory.
struct MemDisc(Vec<u8>);

impl DiscFile for MemDisc {
    fn name(&self) -> String {
        "mem.bin".into()
    }
    fn open(&self) -> anyhow::Result<Box<dyn ReadSeek>> {
        Ok(Box::new(std::io::Cursor::new(self.0.clone())))
    }
}

/// A pack in memory that keeps each file's size and hash (the pack is 3 GB), and the manifests, the fight data, the
/// motions, the characters, the effects' table and King's ending whole.
#[derive(Default)]
struct Hashed(Mutex<BTreeMap<String, (usize, u64)>>, Mutex<BTreeMap<String, Vec<u8>>>);

impl Vfs for Hashed {
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.1
            .lock()
            .unwrap()
            .get(path)
            .filter(|_| self.exists(path))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no {path}"))
    }
    fn write(&self, path: &str, data: &[u8]) -> anyhow::Result<()> {
        let mut h = DefaultHasher::new();
        h.write(data);
        self.0.lock().unwrap().insert(path.into(), (data.len(), h.finish()));
        let kept = |p: &str| path.strip_prefix("assets/").is_some_and(|a| a.starts_with(p));
        if path == "manifest.json"
            || path.starts_with("gamedata/")
            || [
                "manifest.json",
                "gamedata/",
                "motions/",
                "stages/index.json",
                "stages/bowl_",
                "characters/",
                "audio/index.json",
                "pictures/141_000",
                "pictures/effects/effects.json",
                "endings/ending_04_",
            ]
            .iter()
            .any(|p| kept(p))
        {
            self.1.lock().unwrap().insert(path.into(), data.to_vec());
        }
        Ok(())
    }
    fn exists(&self, path: &str) -> bool {
        self.0.lock().unwrap().contains_key(path)
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<Entry>> {
        Ok(Vec::new())
    }
    fn remove_all(&self, path: &str) -> anyhow::Result<()> {
        self.0.lock().unwrap().retain(|p, _| !p.starts_with(path));
        Ok(())
    }
}

fn convert(disc: &dyn DiscFile, out: &Hashed) -> anyhow::Result<serde_json::Value> {
    pack::convert(&Ctx { out, log: &|_| {}, jobs: 1 }, disc, true)
}

#[test]
fn zeros_and_another_disc_are_errors() {
    let out = Hashed::default();
    out.write("kept", b"a pack from before").unwrap();
    for sectors in [0, 1, 40] {
        assert!(convert(&MemDisc(vec![0; sectors * 2352]), &out).is_err());
    }
    // a disc with an empty root directory: ISO 9660, but not this game
    let mut disc = vec![0u8; 20 * 2352];
    let pvd = 16 * 2352 + 24;
    disc[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
    disc[pvd + 158..pvd + 162].copy_from_slice(&18u32.to_le_bytes());
    disc[pvd + 166..pvd + 170].copy_from_slice(&2048u32.to_le_bytes());
    let e = convert(&MemDisc(disc), &out).unwrap_err();
    assert!(format!("{e:#}").contains("not Tekken Tag Tournament USA v2.00 (SLUS-20001)"), "{e:#}");
    // nothing was removed, nothing written; and that is no pack
    assert_eq!(out.0.lock().unwrap().len(), 1);
    assert!(pack::manifest(&out).is_err());
}

#[test]
fn manifest_of_another_pack_is_an_error() {
    let out = Hashed::default();
    for (m, ok) in [
        (format!(r#"{{"pack": "ttt", "format": {}}}"#, pack::FORMAT), true),
        (format!(r#"{{"pack": "ttt", "format": {}}}"#, pack::FORMAT + 1), false),
        (format!(r#"{{"pack": "t2", "format": {}}}"#, pack::FORMAT), false),
        ("not json".to_string(), false),
    ] {
        out.write("manifest.json", m.as_bytes()).unwrap();
        assert_eq!(pack::manifest(&out).is_ok(), ok, "{m}");
    }
}

#[test]
fn the_disc_converts_twice_the_same() {
    let Ok(path) = std::env::var("TTT_DISC") else {
        eprintln!("skipped: TTT_DISC is not set (the Tekken Tag Tournament disc image)");
        return;
    };
    let disc = Disc::open_path(std::path::Path::new(&path)).unwrap();
    let track = disc.data().unwrap();
    let (a, b) = (Hashed::default(), Hashed::default());
    a.write("old", b"of a pack before: removed").unwrap();
    // (side by side: a conversion takes most of a minute)
    let (m, m2) = std::thread::scope(|s| {
        let second = s.spawn(|| pack::convert(&Ctx { out: &b, log: &|_| {}, jobs: 1 }, track, false).unwrap());
        (convert(track, &a).unwrap(), second.join().unwrap())
    });
    assert_eq!(pack::manifest(&a).unwrap(), m);
    assert_eq!(m["source"]["disc"], "SLUS-20001");
    let count = |k: &str| m[k].as_array().unwrap().len();
    let sounds: usize = m["sounds"].as_array().unwrap().iter().map(|s| s["sounds"].as_array().unwrap().len()).sum();
    assert_eq!((count("models"), count("stages"), count("anims"), sounds), (133, 23, 4291, 2350));
    assert_eq!((count("music"), count("video"), count("movesets"), count("own")), (25, 3, 33, 33));
    assert!(
        m["video"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["converted"] == false && v["file"].as_str().unwrap().ends_with(".pss"))
    );
    // every file the manifest names is in the pack, and no other
    const FOLDERS: [&str; 8] = ["models", "anims", "stages", "textures", "audio", "video", "raw", "gamedata"];
    fn files(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s)
                if FOLDERS.iter().any(|f| s.strip_prefix(f).is_some_and(|s| s.starts_with('/'))) =>
            {
                out.push(s.clone())
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| files(v, out)),
            serde_json::Value::Object(o) => o.values().for_each(|v| files(v, out)),
            _ => {}
        }
    }
    let mut named = vec!["manifest.json".to_string()];
    files(&m, &mut named);
    named.sort();
    let written = a.0.lock().unwrap().clone();
    let research: Vec<String> = written.keys().filter(|k| !k.starts_with("assets/")).cloned().collect();
    assert_eq!(named, research);
    assert!(written.values().all(|f| f.0 > 0));
    fight_data(&a);
    openkumite(&a);
    front_end(&a);
    bowl(&a);
    endings(&a);
    // the same disc again, without the research outputs: the same assets/, and a manifest of the source alone
    assert_eq!(m2, serde_json::json!({"pack": "ttt", "format": pack::FORMAT, "source": m["source"]}));
    let game: Vec<_> = written.iter().filter(|(k, _)| k.starts_with("assets/")).collect();
    let b = b.0.lock().unwrap();
    assert_eq!(game, b.iter().filter(|(k, _)| *k != "manifest.json").collect::<Vec<_>>());
}

/// The fight data's known facts (docs/FIGHT_DATA.md), and damage values equal to Tekken 3's for the same move.
fn fight_data(pack: &Hashed) {
    let load = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&pack.read(&format!("gamedata/{name}")).unwrap()).unwrap()
    };
    let (moves, cancels) = (load("moves.json"), load("cancels.json"));
    let moves = moves["moves"].as_array().unwrap();
    assert_eq!(moves.len(), 7351);
    // every cancel list (from a move's start to the next move's) ends in exactly one END
    let entries = cancels["cancels"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 0x82cf);
    let mut starts: Vec<usize> = moves.iter().map(|m| m["cancel_start"].as_u64().unwrap() as usize).collect();
    starts.sort();
    starts.push(entries.len());
    for l in starts.windows(2) {
        let list = &entries[l[0]..l[1]];
        assert_eq!(list.iter().filter(|e| e["kind"] == "end").count(), 1, "the list at {}", l[0]);
        assert_eq!(list.last().unwrap()["cmd"], 0x8000);
    }
    assert_eq!(cancels["shared_cancels"]["entries"].as_array().unwrap().len(), 782);
    assert_eq!(cancels["shared_cancels"]["list_starts"].as_array().unwrap().len(), 40);
    assert_eq!(cancels["sequences"]["entries"].as_array().unwrap().len(), 100);
    assert_eq!(cancels["reversal_families"]["families"].as_array().unwrap().len(), 54);
    // the movesets: 33 of 5532 aliases; Paul stands on alias 58 and crouches on 80
    let index = load("movesets/index.json");
    assert_eq!((index["files"].as_array().unwrap().len(), index["fighters"].as_array().unwrap().len()), (33, 39));
    assert_eq!(index["models"].as_array().unwrap().len(), 127);
    for m in 0..33 {
        let set = load(&format!("movesets/moveset_{m:02}.json"));
        assert_eq!((set["moveset_id"].as_u64(), set["aliases"].as_array().unwrap().len()), (Some(m), 5532));
    }
    let paul = load("movesets/moveset_00.json");
    assert_eq!(paul["fighters"][0]["name"], "Paul");
    assert_eq!(
        (&paul["aliases"][58], &paul["aliases"][80], &paul["aliases"][7], &paul["move_count"]),
        (&2.into(), &299.into(), &serde_json::Value::Null, &2257.into())
    );
    // Paul's jab: 5 points high on frame 8 of 37 with the hand (bone 10), as in Tekken 3; 2 is accepted on frames
    // 1-12 and commits at 8; the list ends at 25, back to standing
    let jab = &moves[paul["aliases"][0x15a].as_u64().unwrap() as usize];
    assert_eq!((jab["index"].as_u64(), jab["damage"].as_u64(), jab["frames"].as_u64()), (Some(806), Some(5), Some(37)));
    assert_eq!((&jab["active"], jab["attack_mask"]["level_name"].as_str()), (&serde_json::json!([8, 8]), Some("high")));
    assert_eq!((jab["attack"]["bones"]["end0"].as_u64(), jab["anim"].as_str()), (Some(10), Some("anims/0806.glb")));
    let list = &entries[jab["cancel_start"].as_u64().unwrap() as usize..];
    let two = list.iter().find(|e| e["cmd"] == 2).unwrap();
    assert_eq!(
        (two["win_start"].as_u64(), two["win_end"].as_u64(), two["exec"].as_u64()),
        (Some(1), Some(12), Some(8))
    );
    let end = list.iter().find(|e| e["kind"] == "end").unwrap();
    assert_eq!((end["win_start"].as_u64(), end["dest"].as_u64()), (Some(25), Some(58)));
    // damage equal to Tekken 3's converted data for the same move of Paul (its moves 119, 191, 135, 179)
    for (i, damage, level) in [(1068, 100, "unblockable"), (1826, 33, "mid"), (1204, 15, "mid"), (1485, 30, "mid")] {
        assert_eq!(
            (moves[i]["damage"].as_u64(), moves[i]["attack_mask"]["level_name"].as_str()),
            (Some(damage), Some(level)),
            "move {i}"
        );
    }
    // hit records, reactions (two more than the 1429 FIGHT_DATA.md first counted: hit records name 1429 and 1430),
    // curves (two more than 72: reactions point at 72 and 73)
    let (hits, reactions) = (load("hits.json"), load("reactions.json"));
    let records = hits["records"].as_array().unwrap();
    assert_eq!((records.len(), records.iter().filter(|r| r["gate"].is_null()).count()), (4174, 3676));
    assert_eq!(records[jab["hit"].as_u64().unwrap() as usize]["reaction"], 46);
    assert!(
        records.iter().all(|r| r["reaction"].as_u64().unwrap() < 1431 && r["close_reaction"].as_u64().unwrap() < 1431)
    );
    assert_eq!(reactions["entries"].as_array().unwrap().len(), 1431);
    assert_eq!(reactions["pushback_curves"]["entries"].as_array().unwrap().len(), 74);
    assert_eq!(reactions["entries"][1]["params"], serde_json::json!([0, 0, 0, 0, 0, 32, 9]));
    // the air reactions (FIGHT_DATA.md section 9): 0-7 shared, 8-43 one move's each
    let air = reactions["air_reactions"]["entries"].as_array().unwrap();
    let aliases: Vec<u64> = air[..8].iter().map(|a| a["alias"].as_u64().unwrap()).collect();
    assert_eq!((air.len(), aliases), (44, vec![4727, 4762, 4766, 4767, 4729, 4764, 4765, 4763]));
    assert_eq!(
        (air[4]["target_height"].as_i64(), air[1]["rise"].as_i64(), &air[0]["pushback"]),
        (Some(1357), Some(400), &serde_json::json!([100, 50, 10, 0, 0, 0, 0, 0]))
    );
    let own: Vec<u64> = moves.iter().filter_map(|m| m["air_reaction"].as_u64().filter(|&k| k != 0)).collect();
    assert_eq!((own.len(), own.iter().min(), own.iter().max()), (36, Some(&8), Some(&43)));
    // the hurt and push radii: the normal set, and the large bodies' (Kuma, Gun Jack, True Ogre, Jack-2, P.Jack)
    let rules = load("rules.json");
    let radii = |table: &str, body: usize| rules[table]["per_character"][body]["radii"].clone();
    let normal = serde_json::json!([200, 200, 120, 120, 300, 300, 100, 100, 250, 300, 300, 200, 400, 400]);
    let large = serde_json::json!([300, 300, 200, 200, 450, 450, 150, 150, 450, 450, 450, 400, 600, 600]);
    for body in 0..34 {
        assert_eq!(
            radii("hurt_spheres", body),
            if [11, 16, 20, 21, 22].contains(&body) { large.clone() } else { normal.clone() },
            "body {body}"
        );
    }
    assert_eq!(radii("push_spheres", 0), serde_json::json!([240, 96, 96, 300, 120, 120, 120, 120]));
    assert_eq!(radii("push_spheres", 11), serde_json::json!([300, 96, 96, 375, 120, 120, 120, 120]));
    let bones: Vec<u64> =
        rules["hurt_spheres"]["layout"].as_array().unwrap().iter().map(|l| l["bone_slot"].as_u64().unwrap()).collect();
    assert_eq!(bones, [17, 14, 10, 6, 16, 13, 9, 5, 2, 8, 4, 0, 15, 12]);
    assert_eq!(
        rules["health"]["life_options"].as_array().unwrap()[..5],
        [125, 130, 135, 140, 160].map(serde_json::Value::from)
    );
    assert_eq!((&rules["health"]["default_life_option"], &rules["health"]["default_max"]), (&3.into(), &140.into()));
    assert_eq!(rules["counter_hit"]["bonus_tenths"], serde_json::json!([3]));
    let percent = &rules["health"]["percent_per_character"];
    assert_eq!(
        (
            percent[0].as_u64(),
            percent[2].as_u64(),
            percent[3].as_u64(),
            percent[11].as_u64(),
            percent[14].as_u64(),
            percent[26].as_u64()
        ),
        (Some(100), Some(108), Some(109), Some(120), Some(115), Some(111))
    );
    // the camera (FIGHT_DATA.md section 18): H 615 but arena 24's 850, [500, 900] on arenas 5, 11, 12; 485 camera
    // motions; list 0x69: 30 % type 18 (26 mirrored), 30 % type 28, 40 % motion 1 (2 mirrored)
    let camera = load("camera.json");
    let rows = camera["arena_projection"]["rows"].as_array().unwrap();
    let h: Vec<[i64; 2]> = rows.iter().map(|r| [r["h"].as_i64().unwrap(), r["h_min"].as_i64().unwrap()]).collect();
    assert_eq!((h.len(), h[0], h[5], h[11], h[24]), (25, [615, 600], [615, 500], [615, 500], [850, 600]));
    assert_eq!(camera["camera_motions"]["count"], 485);
    assert_eq!(camera["intro"]["offsets"], serde_json::json!([[0, 0, -3700]]));
    let list = camera["move_camera_lists"]["lists"].as_array().unwrap().iter().find(|l| l["id"] == 0x69).unwrap();
    let picks: Vec<[i64; 4]> = list["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| ["type", "value", "mirrored_type", "mirrored_value"].map(|k| e[k].as_i64().unwrap()))
        .collect();
    assert_eq!(picks, [[1, 18, 1, 26], [1, 28, 1, 28], [2, 1, 2, 2]]);
}

/// OpenKumite's `assets/`: its manifest, the counts, the roster and indexes, and a character and the motions read as
/// `kumite_assets::Character` reads them (the first mesh, the first skin, the first image; clips by joint name).
fn openkumite(pack: &Hashed) {
    let load = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&pack.read(&format!("assets/{name}")).unwrap()).unwrap()
    };
    let m = load("manifest.json");
    assert_eq!((&m["game"], &m["format"], m["fingerprint"].as_str().unwrap().len()), (&"ttt".into(), &8.into(), 32));
    let written = pack.0.lock().unwrap().clone();
    let count = |dir: &str| written.keys().filter(|k| k.starts_with(&format!("assets/{dir}/"))).count();
    // (and Tekken Bowl's 114 motions, its ball, pin and pin deck)
    assert_eq!((count("characters"), count("motions"), count("stages")), (127, 4231 + 114, 23 + 1 + 3));
    let roster = load("gamedata/roster.json");
    let entries = roster["selections"]["entries"].as_array().unwrap();
    assert_eq!((entries.len(), roster["characters"].as_array().unwrap().len()), (127, 34));
    assert_eq!(
        entries[0],
        serde_json::json!({"selection": 0, "character": 0, "costume": 0, "name": "Paul",
        "model_record": 351, "glb": "model_351.glb", "moveset_file": "movesets/moveset_00.json", "moveset_id": 0})
    );
    let index = load("gamedata/movesets/index.json");
    assert!(
        index["models"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| written.contains_key(&format!("assets/characters/{}", e["glb"].as_str().unwrap()))
                && e["model_record"].is_u64())
    );
    assert_eq!(load("stages/index.json")[20]["name"], "Tekken Bowl");
    // every character file's shadow mesh; each arena's shadow (Paul's: alpha 0x30)
    let shadows = load("gamedata/shadows.json");
    let meshes = shadows["meshes"].as_array().unwrap();
    assert!(meshes.len() == 127 && meshes.iter().all(|m| m["triangles"].as_array().unwrap().len() > 150));
    let stages = load("gamedata/stages.json");
    assert_eq!(
        (stages["stages"].as_array().unwrap().len(), &stages["stages"][0]["shadow"]["alpha"]),
        (23, &0.375.into())
    );
    // and its lights: Law's key at 255, Paul's ambient 38.4
    assert_eq!(
        (&stages["stages"][1]["lights"]["colour"][0], &stages["stages"][0]["lights"]["ambient"][0]),
        (&serde_json::json!([255.0, 255.0, 255.0]), &serde_json::json!(38.400001525878906))
    );
    let effects = load("pictures/effects/effects.json");
    assert_eq!((&effects["atlas"]["rows"], &effects["strips"]["spark"]["life"]), (&3.into(), &35.into()));
    sound(&load, count("audio/sfx"), count("audio/music"));
    let rules = load("gamedata/rules.json");
    assert_eq!((&rules["dialect"], &rules["hurt_spheres"]["layout"][8]["gltf_joint"]), (&"ttt".into(), &"head".into()));
    // Paul: his character, skinned on the 98 nodes, the 18 joints named, one picture
    let (doc, buffers, images) = gltf::import_slice(pack.read("assets/characters/model_351.glb").unwrap()).unwrap();
    let skin = doc.skins().next().unwrap();
    let joints: Vec<String> = skin.joints().map(|j| j.name().unwrap().to_string()).collect();
    assert_eq!(joints.len(), 98);
    for name in [
        "root",
        "chest",
        "head",
        "clavicle_A",
        "upperarm_A",
        "forearm_A",
        "hand_A",
        "clavicle_B",
        "upperarm_B",
        "forearm_B",
        "hand_B",
        "pelvis",
        "thigh_A",
        "shin_A",
        "foot_A",
        "thigh_B",
        "shin_B",
        "foot_B",
    ] {
        assert!(joints.iter().any(|j| j == name), "no joint {name}");
    }
    assert_eq!((doc.meshes().count(), images.len(), doc.animations().count()), (1, 1, 0));
    let get = |b: gltf::Buffer| Some(&buffers[b.index()].0[..]);
    let mut vertices = 0;
    for p in doc.meshes().next().unwrap().primitives() {
        let r = p.reader(get);
        let n = r.read_positions().unwrap().count();
        let (jn, uv) = (r.read_joints(0).unwrap().into_u16(), r.read_tex_coords(0).unwrap().into_f32());
        assert!(jn.flatten().all(|j| (j as usize) < joints.len()));
        assert!(uv.flatten().all(|c| (0.0..=1.0).contains(&c)));
        assert_eq!(r.read_weights(0).unwrap().into_f32().count(), n);
        vertices += n;
    }
    assert!(vertices > 1000, "{vertices}");
    // every character: its legs too (the sections past the first; a twist copy, "thigh_A.1", is its joint's),
    // within OpenKumite's loader caps (an image's side and pixels, an accessor's elements)
    for e in entries {
        let glb = e["glb"].as_str().unwrap();
        let (doc, buffers, images) =
            gltf::import_slice(pack.read(&format!("assets/characters/{glb}")).unwrap()).unwrap();
        let names: Vec<&str> = doc.skins().next().unwrap().joints().map(|j| j.name().unwrap()).collect();
        let mut weighted = std::collections::BTreeSet::new();
        for p in doc.meshes().next().unwrap().primitives() {
            let r = p.reader(|b| Some(&buffers[b.index()].0[..]));
            let (jn, wt) = (r.read_joints(0).unwrap().into_u16(), r.read_weights(0).unwrap().into_f32());
            for (j, w) in jn.zip(wt) {
                weighted.extend(
                    j.iter().zip(w).filter(|(_, w)| *w > 0.0).map(|(&j, _)| names[j as usize].split('.').next()),
                );
            }
        }
        for leg in ["thigh_A", "shin_A", "foot_A", "thigh_B", "shin_B", "foot_B"] {
            assert!(weighted.contains(&Some(leg)), "{glb}: no vertex on {leg}");
        }
        assert!(images.iter().all(|i| i.width.max(i.height) <= 8192 && i.width as u64 * i.height as u64 <= 1 << 24));
        assert!(doc.accessors().all(|a| a.count() <= 1 << 22));
    }
    // his motions: every move of his moveset that has one names a clip whose file is in his list, and the clip's
    // channels are on the character's joints
    let moves = load("gamedata/moves.json");
    let set = load("gamedata/movesets/moveset_00.json");
    let list = load("gamedata/movesets/motions_00.json");
    let files: Vec<&str> = list["motions"].as_array().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
    let mut clips = 0;
    for alias in set["aliases"].as_array().unwrap().iter().filter_map(|a| a.as_u64()) {
        let Some(clip) = moves["moves"][alias as usize]["anim"].as_str() else { continue };
        assert!(files.contains(&format!("{clip}.glb").as_str()), "{clip}");
        clips += 1;
    }
    assert!(clips > 2000 && files.len() > 1000, "{clips} {}", files.len());
    for file in [files[0], files[files.len() - 1]] {
        let (doc, buffers, _) = gltf::import_slice(pack.read(&format!("assets/motions/{file}")).unwrap()).unwrap();
        assert_eq!(doc.meshes().count(), 0);
        let a = doc.animations().next().unwrap();
        assert_eq!(Some(file.trim_end_matches(".glb")), a.name());
        for ch in a.channels() {
            assert!(joints.iter().any(|j| Some(j.as_str()) == ch.target().node().name()));
            let frames = ch.sampler().input().count();
            match ch.reader(|b| Some(&buffers[b.index()].0[..])).read_outputs().unwrap() {
                gltf::animation::util::ReadOutputs::Rotations(q) => assert_eq!(q.into_f32().count(), frames),
                gltf::animation::util::ReadOutputs::Translations(t) => {
                    assert_eq!((ch.target().node().name(), t.count()), (Some("root"), frames))
                }
                _ => panic!("a channel of neither rotations nor translations"),
            }
        }
    }
}

/// The front end (docs/FRONTEND.md): roster.json `front_end`'s tables, every picture it names written, file 141's
/// pictures in colour.
fn front_end(pack: &Hashed) {
    let roster: serde_json::Value = serde_json::from_slice(&pack.read("assets/gamedata/roster.json").unwrap()).unwrap();
    let fe = &roster["front_end"];
    let labels: Vec<&str> =
        fe["menu"]["items"].as_array().unwrap().iter().map(|i| i["label"].as_str().unwrap()).collect();
    assert_eq!(
        labels,
        [
            "ARCADE",
            "VS BATTLE",
            "TEAM BATTLE",
            "TIME ATTACK",
            "SURVIVAL",
            "1 ON 1",
            "1 ON 1 VS",
            "PRACTICE",
            "TEKKEN BOWL",
            "THEATER",
            "GALLERY",
            "OPTIONS"
        ]
    );
    let select = &fe["select"];
    assert_eq!(
        select["selectable"],
        serde_json::json!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 16, 18, 23, 26, 27, 30, 32])
    );
    assert_eq!(
        select["slots"][24],
        serde_json::json!({"slot": 24, "up": 14, "down": 32, "left": 23, "right": 25, "x": 374, "y": 112, "character": 0})
    );
    assert_eq!(
        (&select["slots"][0]["character"], &select["faces"][0], &select["faces"][34]),
        (&serde_json::Value::Null, &"003_097.png".into(), &"003_109.png".into())
    );
    let schedule = fe["unlocks"]["schedule"].as_array().unwrap();
    assert_eq!((schedule.len(), &schedule[29]), (32, &serde_json::json!({"stage": 14, "kind": 1, "character": 33})));
    assert_eq!(fe["arcade"]["rivals"][0], serde_json::json!([19, 11, 1]));
    assert_eq!((&fe["options"][2]["label"], &fe["options"][2]["default"]), (&"ROUND TIME".into(), &2.into()));
    // the motion and sounds (FRONTEND.md 9): ARCADE gold, the select's sounds, the tags' x, "1ST", the line
    assert_eq!(
        (&fe["menu"]["items"][0]["colour"], &fe["menu"]["items"][11]["colour"], &fe["sounds"]["last_pick"]),
        (&11.into(), &19.into(), &5.into())
    );
    assert_eq!(
        (&select["tag_dx"], &select["ordinal"]["text"], &fe["title"]["press_start"]),
        (&serde_json::json!([2, 25]), &"1ST".into(), &"PRESS P1 AND/OR P2 BUTTON".into())
    );
    // the 3D characters (FRONTEND.md 9): H 850, 1P's place, Xiaoyu's stance and pick pose, their files there
    let models = &select["models"];
    assert_eq!(
        (&models["camera"]["h"], &models["places"][0], &models["motions"][7]),
        (&850.into(), &serde_json::json!([400, 112, 0, 0xE800]), &serde_json::json!(["motion_6459", "motion_6490"]))
    );
    for m in models["motions"].as_array().unwrap().iter().flat_map(|m| m.as_array().unwrap()) {
        assert!(pack.exists(&format!("assets/motions/{}.glb", m.as_str().unwrap())), "{m}");
    }
    fn pictures<'a>(v: &'a serde_json::Value, out: &mut Vec<&'a str>) {
        match v {
            serde_json::Value::String(s) if s.ends_with(".png") => out.push(s),
            serde_json::Value::Array(a) => a.iter().for_each(|v| pictures(v, out)),
            serde_json::Value::Object(o) => o.values().for_each(|v| pictures(v, out)),
            _ => {}
        }
    }
    let mut named = vec![];
    pictures(fe, &mut named);
    assert_eq!(named.len(), 236);
    for p in named {
        assert!(pack.exists(&format!("assets/pictures/{p}")), "{p}");
    }
    // file 141's pictures take its first palette: in colour, not transparent
    let png = pack.read("assets/pictures/141_000.png").unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap();
    let mut px = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut px).unwrap();
    assert!(px.chunks(4).filter(|p| p[3] == 255 && p[2] > 100).count() > 10_000);
}

/// The endings (docs/BOWL_ENDINGS.md 7-8): endings.json's tables, and each scene's files.
fn endings(pack: &Hashed) {
    let e: serde_json::Value = serde_json::from_slice(&pack.read("assets/gamedata/endings.json").unwrap()).unwrap();
    // Xiaoyu's costumes 4, 6, 7 and Eddy's (Tiger) take another ending
    assert_eq!((&e["table"][7], &e["table"][8][4]), (&serde_json::json!([9, 9, 9, 9, 10, 9, 10, 10]), &33.into()));
    let all = e["endings"].as_array().unwrap();
    assert_eq!(all.len(), 38);
    // King's: arena 11, King in the player's costume, Armor King, Mokujin 6; 1800 frames, op -1 last
    let king = &all[4];
    assert_eq!((&king["arena"], &king["cast"]), (&11.into(), &serde_json::json!([[3, -1], [26, 0], [15, 6]])));
    assert_eq!(king["script"].as_array().unwrap().last().unwrap(), &serde_json::json!([1799, -1, 0, 0, 0]));
    let camera = pack.read("assets/endings/ending_04_camera.bin").unwrap();
    assert_eq!((u32::from_le_bytes(camera[..4].try_into().unwrap()), camera.len()), (1800, 4 + 1800 * 28));
    for (k, d) in all.iter().enumerate() {
        for f in d["body"].as_array().unwrap().iter().chain([&d["camera"], &d["face"], &d["hands"]]) {
            assert!(pack.exists(&format!("assets/endings/{}", f.as_str().unwrap())), "ending {k}: {f}");
        }
    }
    // a body track: a clip of the ending's frames on the joints by name
    let (doc, _, _) = gltf::import_slice(pack.read("assets/endings/ending_04_1.glb").unwrap()).unwrap();
    let a = doc.animations().next().unwrap();
    assert_eq!((a.name(), a.channels().next().unwrap().sampler().input().count()), (Some("ending_04_1"), 1800));
    let roll = e["staff_roll"]["lines"].as_array().unwrap();
    assert_eq!(
        (roll.len(), &roll[0]["text"], &roll[431]["text"]),
        (434, &"PROJECT DIRECTORS".into(), &"NAMCO LTD.".into())
    );
    assert_eq!(roll.iter().filter(|l| l["alternative"] == true).count(), 2);
    assert_eq!(
        (&e["theater"]["labels"][0][0], &e["theater"]["labels"][0][4]),
        (&"(1P)PAUL".into(), &serde_json::Value::Null)
    );
    assert_eq!(
        e["theater"]["music"].as_array().unwrap().last().unwrap(),
        &serde_json::json!({"bgm": 254, "name": "EXIT"})
    );
}

/// OpenKumite's sound, music and round flow: the samples per bank and voice group, a voice row, Paul's jab, every
/// arena's music (docs/FIGHT_DATA.md sections 13-14).
fn sound(load: &dyn Fn(&str) -> serde_json::Value, samples: usize, tracks: usize) {
    let (index, sound, music) = (load("audio/index.json"), load("gamedata/sound.json"), load("gamedata/music.json"));
    assert_eq!((samples, tracks, index["music"].as_array().unwrap().len()), (2350, 25, 25));
    let bank = |n: u64| {
        let b = index["sfx"].as_array().unwrap().iter().find(|b| b["bank"] == n).unwrap();
        b["samples"].as_array().unwrap().len() as u64
    };
    assert_eq!((bank(1), bank(2), bank(45), bank(344)), (96, 2, 28, 22));
    let groups = sound["groups"]["entries"].as_array().unwrap();
    assert_eq!(groups.len(), 62);
    for g in groups.iter().filter(|g| g["bank"].is_u64()) {
        assert_eq!(bank(g["bank"].as_u64().unwrap()), g["count"].as_u64().unwrap(), "{}", g["group"]);
    }
    // every character's voice bank holds its voice row's group; Paul's row 0, Tiger (Eddy's 5th costume) Tiger's
    let banks = sound["voice_banks"]["characters"].as_array().unwrap();
    assert_eq!(banks.len(), 127);
    for c in banks {
        let row = c["voice_row"].as_u64().unwrap();
        assert_eq!(bank(c["bank"].as_u64().unwrap()), groups[23 + row as usize]["count"].as_u64().unwrap());
    }
    let row = |model: u64| banks.iter().find(|c| c["model_record"] == model).unwrap()["voice_row"].clone();
    assert_eq!((row(351), row(462)), (0.into(), 21.into()));
    let paul = &sound["voice_rows"]["rows"][0];
    assert_eq!((&paul["counts"], &paul["cooldown"]), (&serde_json::json!([8, 5, 3, 5]), &10.into()));
    assert_eq!((&paul["codes"][0], &paul["codes"][20]), (&0xd3.into(), &0xe7.into()));
    assert_eq!(sound["voice_rows"]["rows"][21]["cooldown"], 6);
    // Paul's jab (n+1: alias 346, move 806): no voice, a type-6 entry (silent in this game) and hit sound 8
    let moves = load("gamedata/moves.json");
    let raw: Vec<u64> =
        moves["moves"][806]["sounds"].as_array().unwrap().iter().map(|e| e["raw"].as_u64().unwrap()).collect();
    assert_eq!(raw, [0x600f, 0x7008]);
    assert_eq!(sound["scripts"]["entries"].as_array().unwrap().len(), 668);
    // every arena has a theme, in a stream written; Lei's (BGM 14) is file 23, not 14 + 13
    let bgm = music["bgm"].as_array().unwrap();
    let by_arena = music["by_arena"]["bgm"].as_array().unwrap();
    assert_eq!(by_arena.len(), 23);
    for id in by_arena {
        let file = bgm[id.as_u64().unwrap() as usize]["columns"][0]["file"].as_str().unwrap();
        assert!(index["music"].as_array().unwrap().iter().any(|m| m["file"] == file), "{file}");
    }
    assert_eq!((&bgm[9]["columns"][0]["file"], &bgm[14]["columns"][0]["file"]), (&"22.flac".into(), &"23.flac".into()));
    let rounds = load("gamedata/rounds.json");
    assert_eq!(rounds["win_poses"]["aliases"], serde_json::json!([4770, 4771, 4772, 4773]));
    // the AI: 5 x 8 profiles, Tekken 3's fields 4 bytes later (EASY / NORMAL level 0's cooldowns), King's reach
    let ai = load("gamedata/ai.json");
    let profiles = ai["profiles"]["entries"].as_array().unwrap();
    assert_eq!(profiles.len(), 40);
    let cooldowns =
        |p: &serde_json::Value| (1..4).map(|k| p["fields"][format!("attack_cooldown_{k}")].clone()).collect::<Vec<_>>();
    assert_eq!(
        (cooldowns(&profiles[0]), cooldowns(&profiles[8])),
        (vec![300.into(), 400.into(), 400.into()], vec![150.into(), 200.into(), 200.into()])
    );
    assert_eq!(ai["spacing"]["entries"][3]["values"], serde_json::json!([1632, 1792, 2592, 3072, 1792, 0]));
    assert_eq!(ai["canned_sequences"]["entries"][0]["words"], serde_json::json!([5, 2, 2, 2, 2, 5, 5, 5, 5, 5]));
    assert_eq!(ai["motion_records"]["entries"][0]["steps"], serde_json::json!([5, 6, 5, 6]));
    assert_eq!((&ai["rng"]["ai_lcg_seed"], &ai["evade_choices"][3]), (&640.into(), &4.into()));
    // section 21: NORMAL level 4's tag out, delayed input and list D; King's throws; the sine table
    let normal4 = &profiles[12]["fields"];
    assert_eq!(
        (&normal4["tag_out_prob"], &normal4["delay_input_prob"], &normal4["followup_prob_c"]),
        (&245.into(), &1228.into(), &2048.into())
    );
    let canned = ai["canned_sequences"]["entries"].as_array().unwrap();
    assert!(canned.iter().any(|c| c["id"] == "throws_stand_0" && c["words"] == serde_json::json!([5, 6, 3, 1283, 3])));
    let sine = ai["distance_jitter"]["sine"].as_array().unwrap();
    assert_eq!((sine.len(), &sine[1024], &sine[3072]), (4096, &4096.into(), &(-4096).into()));
}

/// Tekken Bowl (docs/BOWL_ENDINGS.md sections 1-5): bowl.json's tables and constants, the bowler's motions as clips,
/// the ball and the pin.
fn bowl(pack: &Hashed) {
    let b: serde_json::Value = serde_json::from_slice(&pack.read("assets/gamedata/bowl.json").unwrap()).unwrap();
    let c = b["characters"].as_array().unwrap();
    assert_eq!(c.len(), 34);
    assert_eq!(
        c[0],
        serde_json::json!({"id": 0, "s": 360, "left": false, "female": false, "reactions": [8, 39, 70, 71], "voices": [0x2001, 0x2000, 0x4002]})
    );
    assert_eq!(
        (&c[16]["s"], &c[8]["left"], &c[7]["female"], &c[33]["left"]),
        (&500.into(), &true.into(), &true.into(), &true.into())
    );
    let rack = b["pins"]["rack"].as_array().unwrap();
    assert_eq!(rack.len(), 10);
    assert!(
        (rack[0][1].as_f64().unwrap() - 7211.9).abs() < 0.1 && rack[9] == serde_json::json!([682.5, 8394.041564941406])
    );
    assert!((b["pins"]["y"].as_f64().unwrap() + 250.66).abs() < 0.01);
    let people = b["spectators"]["people"].as_array().unwrap();
    assert!(people.len() == 25 && people.iter().all(|p| p["z"].as_f64().unwrap() > 0.0));
    assert_eq!(b["callouts"]["strikes"], serde_json::json!(["STRIKE!", "DOUBLE!!", "TURKEY!!!", "!!GREAT!!"]));
    assert_eq!((&b["callouts"]["perfect"], &b["callouts"]["nice_cover"]), (&"PERFECT".into(), &"NICE COVER".into()));
    assert_eq!(b["strike_poses"].as_array().unwrap().len(), 33);
    // the motions: 114, the throws 150 frames, the charges 175
    let frames: Vec<u64> = b["motions"]["frames"].as_array().unwrap().iter().map(|f| f.as_u64().unwrap()).collect();
    assert_eq!((frames.len(), frames[3], frames[7], frames[0x6A]), (114, 150, 150, 175));
    let (doc, ..) = gltf::import_slice(pack.read("assets/motions/bowl_003.glb").unwrap()).unwrap();
    assert_eq!(doc.animations().next().unwrap().name(), Some("bowl_003"));
    // the ball a sphere of radius 0.158 m about the origin, the pin standing 0.66 m tall
    for (file, lo, hi) in [("ball", [-0.158; 3], [0.158; 3]), ("pin", [-0.135, 0.001, -0.149], [0.135, 0.661, 0.202])] {
        let (doc, ..) = gltf::import_slice(pack.read(&format!("assets/stages/bowl_{file}.glb")).unwrap()).unwrap();
        let p = doc.meshes().next().unwrap().primitives().next().unwrap();
        let bb = p.bounding_box();
        assert!(
            bb.min.iter().zip(lo).chain(bb.max.iter().zip(hi)).all(|(a, b)| (a - b).abs() < 0.002),
            "{file}: {bb:?}"
        );
    }
    // the pin deck where the object table puts it: behind the pins (z 10 m and on), its sign up to 4.9 m
    let (doc, ..) = gltf::import_slice(pack.read("assets/stages/bowl_deck.glb").unwrap()).unwrap();
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in doc.meshes().flat_map(|m| m.primitives().collect::<Vec<_>>()) {
        let bb = p.bounding_box();
        (0..3).for_each(|k| (lo[k], hi[k]) = (lo[k].min(bb.min[k]), hi[k].max(bb.max[k])));
    }
    assert!(lo[2] > 9.0 && hi[2] < 13.5 && hi[1] > 4.5, "{lo:?} {hi:?}");
}

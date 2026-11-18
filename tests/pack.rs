//! The pack end to end. Without the disc: an image of zeros, and a disc of another game, give an error and leave
//! the output as it was. With it (`TTT_DISC`: the image, in any format the converter reads): 133 fighters, 23
//! arenas, 4291 motions, 2350 sounds, every file the manifest names, the fight data's known facts (docs/FIGHT_DATA.md);
//! converted twice, the same bytes.

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

/// A pack in memory that keeps each file's size and hash (the pack is 1.7 GB), and the manifest and the fight data
/// whole.
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
        if path == "manifest.json" || path.starts_with("gamedata/") {
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
    pack::convert(&Ctx { out, log: &|_| {}, jobs: 1 }, disc)
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
        let second = s.spawn(|| convert(track, &b).unwrap());
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
    assert_eq!(named, written.keys().cloned().collect::<Vec<_>>());
    assert!(written.values().all(|f| f.0 > 0));
    fight_data(&a);
    // the same disc again: the same bytes
    assert_eq!(m, m2);
    assert_eq!(written, *b.0.lock().unwrap());
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
}

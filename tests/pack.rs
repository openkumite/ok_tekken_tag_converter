//! The pack end to end. Without the disc: an image of zeros, and a disc of another game, give an error and leave
//! the output as it was. With it (`TTT_DISC`: the image, in any format the converter reads): 133 fighters, 23
//! arenas, 4291 motions, 2350 sounds, every file the manifest names; converted twice, the same bytes.

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

/// A pack in memory that keeps each file's size and hash (the pack is 1.7 GB), and the manifest whole.
#[derive(Default)]
struct Hashed(Mutex<BTreeMap<String, (usize, u64)>>, Mutex<Vec<u8>>);

impl Vfs for Hashed {
    fn read(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(path == "manifest.json" && self.exists(path), "no {path}");
        Ok(self.1.lock().unwrap().clone())
    }
    fn write(&self, path: &str, data: &[u8]) -> anyhow::Result<()> {
        let mut h = DefaultHasher::new();
        h.write(data);
        self.0.lock().unwrap().insert(path.into(), (data.len(), h.finish()));
        if path == "manifest.json" {
            *self.1.lock().unwrap() = data.to_vec();
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
    const FOLDERS: [&str; 7] = ["models", "anims", "stages", "textures", "audio", "video", "raw"];
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
    // the same disc again: the same bytes
    assert_eq!(m, m2);
    assert_eq!(written, *b.0.lock().unwrap());
}

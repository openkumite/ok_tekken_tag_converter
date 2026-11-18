//! The pack's `assets/`: the files OpenKumite reads, laid out as the Tekken 3 converter lays out its own
//! (docs/GAMEDATA.md "assets/"). [`crate::pack::convert`] writes them; here are their index files and the
//! manifest's fingerprint.

use anyhow::{Context, Result, bail};
use ok_converter_common::io::{Vfs, join};
use serde_json::{Value, json};
use sha2::Digest;

/// manifest.json's `format`: the layout of OpenKumite's data this follows (the Tekken 3 converter's FORMAT, which
/// `kumite_assets::FORMAT` asks for at least).
pub const FORMAT: i64 = 8;
/// What the manifest names the game.
pub const GAME: &str = "ttt";
/// The folders a fight reads, which the fingerprint covers.
const FIGHT: [&str; 3] = ["characters", "motions", "gamedata"];

/// `gamedata/roster.json` in the shape OpenKumite's roster reads (the Tekken 3 converter's keys): `characters` (a
/// body each: `id`, `name`, `moveset_id`), `selections.entries` (each character file: `selection` = body * 8 +
/// costume, `character`, `costume`, `name`, `model_record`, `glb`, `moveset_file`, `moveset_id`) and
/// `select_grid.entries` (every character, in body order: the select screen's layout is not decoded). `bodies`:
/// each body's (name, moveset); `models`: the characters written (`id`, `body`, `costume`, `moveset`, `file`).
pub fn roster(bodies: &[(String, usize)], models: &[Value]) -> Value {
    let int = |m: &Value, k: &str| m.get(k).and_then(Value::as_u64).unwrap_or(0);
    let named = |body: u64| bodies.get(body as usize).map_or("", |b| b.0.as_str());
    let mut entries: Vec<Value> = (models.iter())
        .map(|m| {
            let (body, costume, set) = (int(m, "body"), int(m, "costume"), int(m, "moveset"));
            json!({"selection": body * 8 + costume, "character": body, "costume": costume, "name": named(body),
                   "model_record": int(m, "id"), "glb": m.get("file"), "moveset_file": format!("movesets/moveset_{set:02}.json"),
                   "moveset_id": set})
        })
        .collect();
    entries.sort_by_key(|e| e["selection"].as_u64());
    let ids: std::collections::BTreeSet<u64> = entries.iter().filter_map(|e| e["character"].as_u64()).collect();
    let characters: Vec<Value> = (ids.iter())
        .map(|&id| {
            let selections: Vec<&Value> = entries.iter().filter(|e| e["character"] == id).map(|e| &e["selection"]).collect();
            json!({"id": id, "name": named(id), "moveset_id": bodies.get(id as usize).map(|b| b.1), "selections": selections})
        })
        .collect();
    let grid: Vec<Value> =
        ids.iter().enumerate().map(|(slot, &id)| json!({"slot": slot, "character": id, "name": named(id)})).collect();
    json!({"characters": characters, "selections": {"formula": "body * 8 + costume", "entries": entries},
           "select_grid": {"entries": grid}})
}

/// The fingerprint of what a fight reads (`characters/`, `motions/`, `gamedata/` of `out`): the Tekken 3
/// converter's (`convert_all::fingerprint`), the same hash of the same canonical forms (a JSON file with its keys
/// sorted, a glb without where its data lies), over these folders.
pub fn fingerprint(out: &dyn Vfs) -> Result<String> {
    // the sub-folders of `base`/`rel` (relative to `base`) and itself
    fn walk(out: &dyn Vfs, base: &str, rel: &str, dirs: &mut Vec<String>) -> Result<()> {
        dirs.push(rel.to_string());
        for e in out.list(&join(base, rel))? {
            if e.is_dir {
                walk(out, base, &join(rel, &e.name), dirs)?;
            }
        }
        Ok(())
    }
    let mut h = sha2::Sha256::new();
    for sub in FIGHT {
        let mut dirs = Vec::new();
        walk(out, sub, "", &mut dirs)?;
        dirs.sort();
        for d in dirs {
            let dir = join(sub, &d);
            for e in out.list(&dir)? {
                if e.is_dir || !(e.name.ends_with(".glb") || e.name.ends_with(".json")) {
                    continue;
                }
                let rel = join(&dir, &e.name);
                h.update(rel.as_bytes());
                h.update(b"\0");
                let data = out.read(&rel)?;
                let canon = if e.name.ends_with(".glb") {
                    canonical_glb(&data)
                } else {
                    serde_json::from_slice(&data).map_err(anyhow::Error::from).map(|v| canonical_json(&v).into_bytes())
                };
                h.update(canon.with_context(|| format!("{rel} is not valid"))?);
            }
        }
    }
    Ok(h.finalize().iter().take(16).map(|b| format!("{b:02x}")).collect())
}

/// Python's `json.dumps(v, sort_keys=True, separators=(',', ':'))` (ASCII only), as the Tekken 3 converter hashes
/// a JSON file.
fn canonical_json(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => i.to_string(),
            (_, Some(u)) => u.to_string(),
            _ => float_repr(n.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(s) => string(s),
        Value::Array(a) => format!("[{}]", a.iter().map(canonical_json).collect::<Vec<_>>().join(",")),
        Value::Object(m) => {
            let mut items: Vec<(&String, &Value)> = m.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));
            let items: Vec<String> =
                items.iter().map(|(k, v)| format!("{}:{}", string(k), canonical_json(v))).collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

/// A string as Python's encoder writes it with `ensure_ascii`.
fn string(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                for u in c.encode_utf16(&mut [0u16; 2]) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Python's `repr(float)`: the shortest digits that round-trip, positional when the decimal exponent is in -4..16,
/// else `d.ddde+XX`; whole numbers keep `.0`.
fn float_repr(f: f64) -> String {
    if !f.is_finite() {
        return if f.is_nan() {
            "NaN"
        } else if f > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .into();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    // Rust's `{:e}` is the shortest round-trip form too; on a tie between two shortest candidates it rounds half up,
    // Python half to even (as exact formatting does)
    let mut e = format!("{f:e}");
    let digits = e.split('e').next().unwrap_or("").bytes().filter(u8::is_ascii_digit).count();
    let even = format!("{:.*e}", digits.saturating_sub(1), f);
    if even.parse::<f64>().ok() == Some(f) {
        e = even;
    }
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mant) = mant.strip_prefix('-').map_or(("", mant), |m| ("-", m));
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let n = digits.len() as i32;
    let split = |k: i32| (digits.get(..k as usize).unwrap_or(""), digits.get(k as usize..).unwrap_or(""));
    if (-4..16).contains(&exp) {
        let point = exp + 1; // digits before the point
        if point <= 0 {
            format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
        } else if point >= n {
            format!("{sign}{digits}{}.0", "0".repeat((point - n) as usize))
        } else {
            let (a, b) = split(point);
            format!("{sign}{a}.{b}")
        }
    } else {
        let m = if n > 1 { format!("{}.{}", split(1).0, split(1).1) } else { digits.clone() };
        format!("{sign}{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    }
}

/// A glb's JSON without where its data lies (buffers, views, images), canonical, then each accessor's elements.
fn canonical_glb(data: &[u8]) -> Result<Vec<u8>> {
    let slice = |d: &[u8], a: usize, b: usize| -> Vec<u8> {
        d.get(a.min(d.len())..b.min(d.len())).unwrap_or_default().to_vec()
    };
    let u32_at = |d: &[u8], o: usize| Some(u32::from_le_bytes(*d.get(o..)?.first_chunk()?));
    let jlen = u32_at(data, 12).context("not a glb")? as usize;
    let mut doc: Value = serde_json::from_slice(&slice(data, 20, 20usize.saturating_add(jlen)))?;
    let rest = slice(data, 20usize.saturating_add(jlen), usize::MAX);
    let blob = match u32_at(&rest, 0) {
        Some(n) => slice(&rest, 8, 8 + n as usize),
        None => vec![],
    };
    let views = doc.get("bufferViews").and_then(Value::as_array).cloned().unwrap_or_default();
    let int = |v: &Value, k: &str, default: Option<u64>| -> Result<usize> {
        match v.get(k) {
            Some(x) => x.as_u64().map(|x| x as usize).with_context(|| format!("{k} is not a count")),
            None => default.map(|x| x as usize).with_context(|| format!("no {k}")),
        }
    };
    let mut parts = Vec::new();
    for a in doc.get("accessors").and_then(Value::as_array).into_iter().flatten() {
        if a.get("bufferView").is_none() {
            continue;
        }
        let v = views.get(int(a, "bufferView", None)?).context("an accessor's bufferView is missing")?;
        let size = match a.get("componentType").and_then(Value::as_u64) {
            Some(5120 | 5121) => 1,
            Some(5122 | 5123) => 2,
            Some(5125 | 5126) => 4,
            t => bail!("unknown componentType {t:?}"),
        };
        let count = match a.get("type").and_then(Value::as_str) {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4" | "MAT2") => 4,
            Some("MAT3") => 9,
            Some("MAT4") => 16,
            t => bail!("unknown accessor type {t:?}"),
        };
        let element = size * count;
        let stride = int(v, "byteStride", Some(element as u64))?;
        let at = int(v, "byteOffset", Some(0))? + int(a, "byteOffset", Some(0))?;
        let n = int(a, "count", None)?;
        if stride == element {
            parts.extend(slice(&blob, at, at.saturating_add(element.saturating_mul(n))));
        } else {
            for i in 0..n {
                let o = at.saturating_add(i.saturating_mul(stride));
                parts.extend(slice(&blob, o, o.saturating_add(element)));
            }
        }
    }
    if let Some(m) = doc.as_object_mut() {
        for key in ["buffers", "bufferViews", "images"] {
            m.shift_remove(key);
        }
    }
    let mut c = canonical_json(&doc).into_bytes();
    c.extend(parts);
    Ok(c)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn json_as_python_hashes_it() {
        for (f, s) in [
            (1.0, "1.0"),
            (-0.0, "-0.0"),
            (1e-5, "1e-05"),
            (0.0001, "0.0001"),
            (1.5e16, "1.5e+16"),
            (1e15, "1000000000000000.0"),
            (123.456, "123.456"),
            (-2.5e-7, "-2.5e-07"),
            (13.472f32 as f64, "13.472000122070312"),
        ] {
            assert_eq!(float_repr(f), s, "{f}");
        }
        let v = json!({"b": [1, 2.0, {"x": "é\n"}], "a": [], "c": {}});
        assert_eq!(canonical_json(&v), r#"{"a":[],"b":[1,2.0,{"x":"\u00e9\n"}],"c":{}}"#);
    }

    #[test]
    fn a_glb_hashes_without_its_layout() {
        // the same accessor data at another place in the buffer: the same canonical form
        let glb = |pad: usize| {
            let mut g = crate::glb::Glb::default();
            g.view(&vec![7; pad]);
            g.accessor(&[1, 2, 3, 4], crate::glb::U8, "VEC4", 1);
            g.finish(json!({"scene": 0}))
        };
        assert_eq!(canonical_glb(&glb(4)).unwrap(), canonical_glb(&glb(12)).unwrap());
        assert!(canonical_glb(&glb(4)).unwrap().ends_with(&[1, 2, 3, 4]));
        assert!(canonical_glb(b"glTF").is_err());
    }

    #[test]
    fn roster_by_body_and_costume() {
        let bodies = vec![("Paul".to_string(), 0), ("Law".to_string(), 1)];
        let models = [
            json!({"id": 348, "body": 1, "costume": 0, "moveset": 1, "file": "model_348.glb"}),
            json!({"id": 345, "body": 0, "costume": 1, "moveset": 0, "file": "model_345.glb"}),
        ];
        let r = roster(&bodies, &models);
        assert_eq!(r["selections"]["entries"][0]["selection"], 1);
        assert_eq!(r["selections"]["entries"][1]["model_record"], 348);
        assert_eq!(r["selections"]["entries"][1]["moveset_file"], "movesets/moveset_01.json");
        assert_eq!(r["characters"][1], json!({"id": 1, "name": "Law", "moveset_id": 1, "selections": [8]}));
        assert_eq!(r["select_grid"]["entries"].as_array().unwrap().len(), 2);
    }
}

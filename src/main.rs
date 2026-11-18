//! `ttt_convert pack DISC PACK`: the pack (pack.rs) of the disc image DISC (.cue/.bin, .iso, .chd, or one in an
//! archive) into the folder PACK (emptied first). `ttt_convert list DISC`: the files of its TEKKEN.BIN.

use anyhow::{Result, bail};
use ok_converter_common::disc::Disc;
use ok_converter_common::io::{Ctx, StdFs, print_line};
use ok_tekken_tag_converter::{archive::Archive, pack};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(String::as_str) {
        Some("pack") => pack_main(args.get(1..).unwrap_or_default()),
        Some("list") => list_main(args.get(1..).unwrap_or_default()),
        _ => Err(anyhow::anyhow!("usage: ttt_convert pack DISC PACK\n       ttt_convert list DISC")),
    };
    if let Err(e) = r {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

/// `pack DISC PACK`
fn pack_main(args: &[String]) -> Result<()> {
    let [disc, out] = args else { bail!("usage: ttt_convert pack DISC PACK") };
    let disc = Disc::open_path(Path::new(disc))?;
    pack::convert(&Ctx { out: &StdFs(out.into()), log: &print_line, jobs: 1 }, disc.data()?, true)?;
    Ok(())
}

/// `list DISC`: each file of TEKKEN.BIN: its number, first sector, size, and whether it is a stream.
fn list_main(args: &[String]) -> Result<()> {
    let [disc] = args else { bail!("usage: ttt_convert list DISC") };
    let disc = Disc::open_path(Path::new(disc))?;
    for (i, e) in Archive::open(disc.data()?)?.toc.iter().enumerate() {
        print_line(&format!(
            "{i:3}  sector {:>6}  {:>9} bytes{}",
            e.sector,
            e.size,
            if e.plain { "  stream" } else { "" }
        ));
    }
    Ok(())
}

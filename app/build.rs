/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use clap_complete::{generate_to, shells};
use std::env;
use std::io::Error;
use std::path::{Path, PathBuf};

include!("../lib/src/args.rs");

fn get_output_path() -> PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let target = out_dir.split_at(out_dir.find(("/".to_string() + &build_type).as_str()).unwrap()).0;
    Path::new(&target).join(build_type)
}

fn main() -> Result<(), Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let release_dir = get_output_path();

    let mut cmd = Arguments::command();
    let bash_path = generate_to(
        shells::Bash,
        &mut cmd,            // We need to specify what generator to use
        "ledger-beautifier", // We need to specify the bin name manually
        outdir.clone(),      // We need to specify where to write to
    )?;

    let fish_path = generate_to(
        shells::Fish,
        &mut cmd,            // We need to specify what generator to use
        "ledger-beautifier", // We need to specify the bin name manually
        outdir.clone(),      // We need to specify where to write to
    )?;

    let zsh_path = generate_to(
        shells::Zsh,
        &mut cmd,            // We need to specify what generator to use
        "ledger-beautifier", // We need to specify the bin name manually
        outdir.clone(),      // We need to specify where to write to
    )?;

    let ps_path = generate_to(
        shells::PowerShell,
        &mut cmd,            // We need to specify what generator to use
        "ledger-beautifier", // We need to specify the bin name manually
        outdir.clone(),      // We need to specify where to write to
    )?;

    let elvish_path = generate_to(
        shells::Elvish,
        &mut cmd,            // We need to specify what generator to use
        "ledger-beautifier", // We need to specify the bin name manually
        outdir.clone(),      // We need to specify where to write to
    )?;

    let man = clap_mangen::Man::new(cmd);
    let man_path = std::path::PathBuf::from(&outdir)
        .join("share")
        .join("man")
        .join("man1");
    std::fs::create_dir_all(&man_path)?;
    let man_path = man_path.join("ledger-beautifier.1");
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    std::fs::write(man_path, buffer)?;

    let share = std::path::PathBuf::from(&release_dir).join("share");

    let copy_options = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64000,
        copy_inside: false,
        content_only: false,
        depth: 0,
    };

    println!(
        "Copying from {:?} into {:?}",
        std::path::PathBuf::from(&outdir).join("share"),
        std::path::PathBuf::from(&release_dir)
    );

    fs_extra::remove_items(&[&share]).unwrap();
    fs_extra::copy_items(
        &[std::path::PathBuf::from(&outdir).join("share")],
        std::path::PathBuf::from(&release_dir),
        &copy_options,
    )
    .unwrap();

    std::fs::create_dir_all(share.join("fish").join("completions"))?;
    fs_extra::copy_items(
        &[fish_path],
        share.join("fish").join("completions"),
        &fs_extra::dir::CopyOptions::new(),
    )
    .unwrap();

    std::fs::create_dir_all(share.join("zsh").join("completions"))?;
    fs_extra::copy_items(
        &[zsh_path],
        share.join("zsh").join("completions"),
        &fs_extra::dir::CopyOptions::new(),
    )
    .unwrap();

    std::fs::create_dir_all(share.join("bash").join("completions"))?;
    fs_extra::copy_items(
        &[bash_path],
        share.join("bash").join("completions"),
        &fs_extra::dir::CopyOptions::new(),
    )
    .unwrap();

    std::fs::create_dir_all(share.join("elvish").join("completions"))?;
    fs_extra::copy_items(
        &[elvish_path],
        share.join("elvish").join("completions"),
        &fs_extra::dir::CopyOptions::new(),
    )
    .unwrap();

    std::fs::create_dir_all(share.join("powershell").join("completions"))?;
    fs_extra::copy_items(
        &[ps_path],
        share.join("powershell").join("completions"),
        &fs_extra::dir::CopyOptions::new(),
    )
    .unwrap();

    Ok(())
}

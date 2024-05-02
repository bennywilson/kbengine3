use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

fn main() -> Result<()> {
    // This tells Cargo to rerun this script if something in /res/ changes.
  /*  println!("cargo::rerun-if-changed=../../engine_assets");
    println!("cargo::rerun-if-changed=game_assets");
    println!("cargo::rerun-if-changed=build.rs");

    println!("-----------------------------------------------");
    let current_exe = std::env::current_exe();
	let exe_path = current_exe.as_ref().unwrap().parent().unwrap();
    let out_dir = format!("{}/../../", exe_path.display());
    println!("Copying to {}", out_dir);
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true; 
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("game_assets/");
    paths_to_copy.push("../../engine_assets/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;
    println!("-----------------------------------------------");
    */
    Ok(())
}
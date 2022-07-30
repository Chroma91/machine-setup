use ansi_term::Color::{Green, Red, White, Yellow};
use ergo_fs::{Path, PathArc};
use std::fs::remove_file;
use symlink::{remove_symlink_file, symlink_file};
use tracing::info;

use crate::{
    command::{CommandConfig, CommandInterface},
    config::config_value::ConfigValue,
    utils::directory::{expand_path, get_source_and_target, walk_files},
};

pub struct SymlinkCommand {}

fn should_force(args: ConfigValue) -> bool {
    if !args.is_hash() {
        return false;
    }

    let arg_values = args.as_hash().unwrap();

    if let Some(force) = arg_values.get("force") {
        return force.as_bool().unwrap_or(false);
    }

    false
}

impl CommandInterface for SymlinkCommand {
    fn install(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        let dirs = get_source_and_target(args.clone(), &config.config_dir)?;

        create_symlink(&dirs.src, &dirs.target, dirs.ignore, should_force(args))
    }

    fn uninstall(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        let dirs = get_source_and_target(args, &config.config_dir)?;

        remove_symlink(&dirs.src, &dirs.target)
    }

    fn update(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        self.install(args, config)
    }
}

fn link_files(
    source_dir: &PathArc,
    destination_dir: &Path,
    ignore: Vec<ConfigValue>,
    force: bool,
) -> Result<(), String> {
    info!(
        "Creating symlinks: {} {} {} ...",
        White.bold().paint(source_dir.to_string()),
        Green.bold().paint("->"),
        White.bold().paint(destination_dir.to_str().unwrap())
    );

    walk_files(source_dir, destination_dir, ignore, |src, target| {
        info!(
            "Linking {} to {} ...",
            White.bold().paint(src.to_str().unwrap()),
            White.bold().paint(target.to_str().unwrap())
        );

        if force && target.is_file() {
            info!(
                "{}",
                Yellow.paint("Replacing exisiting file with symlink (force) ...")
            );

            remove_file(target).ok();
        }

        symlink_file(src, target)
            .map_err(|e| format!("Failed to link file: {}", Red.paint(e.to_string())))
            .ok();
    })
}

fn unlink_files(source_dir: &PathArc, destination_dir: &Path) -> Result<(), String> {
    info!(
        "Unlinking files in {} ...",
        White.bold().paint(destination_dir.to_str().unwrap())
    );

    walk_files(source_dir, destination_dir, vec![], |_src, target| {
        info!(
            "Unlinking {} ...",
            White.bold().paint(target.to_str().unwrap())
        );
        remove_symlink_file(target)
            .map_err(|e| format!("Failed to unlink file: {}", Red.paint(e.to_string())))
            .ok();
    })
}

pub fn create_symlink(
    source: &str,
    destination: &str,
    ignore: Vec<ConfigValue>,
    force: bool,
) -> Result<(), String> {
    let source_dir = expand_path(source, false)?;

    if !source_dir.exists() {
        return Err(format!("Source directory does not exist: {}", source));
    }

    let destination_dir = expand_path(destination, true)?;

    if source_dir.to_string() == destination_dir.to_string() {
        return Err(format!(
            "Source and destination directories are the same: {}",
            source
        ));
    }

    link_files(&source_dir, &destination_dir, ignore, force)
}

pub fn remove_symlink(source: &str, destination: &str) -> Result<(), String> {
    let source_dir = expand_path(source, false)?;
    let destination_dir = expand_path(destination, false)?;

    unlink_files(&source_dir, &destination_dir)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::File, vec};
    use tempfile::tempdir;

    #[test]
    fn it_fails_when_dirs_are_the_same() {
        let dir = tempdir().unwrap();
        let src_path = dir.path();
        File::create(&src_path.join("example.txt")).unwrap();

        let src = src_path.to_str().unwrap();

        println!("{:?}", create_symlink(src, src, vec![], false));

        assert!(create_symlink(src, src, vec![], false)
            .unwrap_err()
            .contains("Source and destination directories are the same"));
    }

    #[test]
    fn it_symlinks_files() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        create_symlink(src, dest, vec![], false).unwrap();

        let dest_path = dest_dir.path().join("example.txt");
        assert!(dest_path.is_symlink())
    }

    #[test]
    fn it_overrides_file_with_symlink() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();
        let dest_path = dest_dir.path().join("example.txt");

        File::create(&dest_path).unwrap();

        create_symlink(src, dest, vec![], true).unwrap();

        assert!(dest_path.is_symlink());
    }

    #[test]
    fn it_removes_symlink() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        create_symlink(src, dest, vec![], false).unwrap();

        let dest_path = dest_dir.path().join("example.txt");
        assert!(dest_path.exists());

        remove_symlink(src, dest).unwrap();

        assert!(!dest_path.exists());
    }
}

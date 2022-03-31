use ergo_fs::{Path, PathDir};
use std::{collections::HashMap, fs};
use yaml_rust::Yaml;

use crate::{
    command::CommandInterface,
    config::{validation_rules::required::Required, validator::validate_named_args},
    utils::directory::{expand_dir, get_source_and_target, walk_files, DIR_TARGET},
};

pub struct CopyDirCommand {}

impl CommandInterface for CopyDirCommand {
    fn install(&self, args: Yaml) -> Result<(), String> {
        let dirs = get_source_and_target(args);
        if dirs.is_err() {
            return Err(dirs.err().unwrap());
        }
        let dirs = dirs.unwrap();

        let result = copy_dir(&dirs.src, &dirs.target, dirs.ignore);
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        return Ok(());
    }

    fn uninstall(&self, args: Yaml) -> Result<(), String> {
        let validation = validate_named_args(
            args.to_owned(),
            HashMap::from([(String::from(DIR_TARGET), vec![&Required {}])]),
        );

        if validation.is_err() {
            return Err(validation.unwrap_err());
        }

        let target_dir = args
            .as_hash()
            .unwrap()
            .get(&Yaml::String(String::from(DIR_TARGET)))
            .unwrap()
            .as_str()
            .unwrap();

        let result = remove_dir(&target_dir);
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        return Ok(());
    }

    fn update(&self, args: Yaml) -> Result<(), String> {
        unimplemented!()
    }
}

fn copy_files(
    source_dir: &PathDir,
    destination_dir: &Path,
    ignore: Vec<Yaml>,
) -> Result<(), String> {
    println!(
        "Copying files from {} to {} ...",
        source_dir.to_string(),
        destination_dir.to_str().unwrap()
    );

    let result = walk_files(&source_dir, &destination_dir, ignore, |src, target| {
        println!(
            "Copying {} to {} ...",
            src.to_str().unwrap(),
            target.to_str().unwrap()
        );
        fs::copy(src, target)
            .map_err(|e| format!("Failed to copy file: {}", e))
            .ok();
    });

    if result.is_err() {
        return Err(result.unwrap_err());
    }

    return Ok(());
}

pub fn copy_dir(source: &str, destination: &str, ignore: Vec<Yaml>) -> Result<(), String> {
    let expanded_source = expand_dir(source, false);
    if expanded_source.is_err() {
        return Err(expanded_source.unwrap_err().to_string());
    }
    let source_dir = expanded_source.to_owned().unwrap();

    let expanded_destination = expand_dir(destination, true);
    if expanded_destination.is_err() {
        return Err(expanded_destination.unwrap_err().to_string());
    }
    let destination_dir = expanded_destination.to_owned().unwrap();

    if source_dir.to_string() == destination_dir.to_string() {
        return Err(format!(
            "Source and destination directories are the same: {}",
            source
        ));
    }

    return copy_files(&source_dir, &destination_dir, ignore);
}

pub fn remove_dir(target: &str) -> Result<(), String> {
    let expanded_target_dir = expand_dir(target, false);
    if expanded_target_dir.is_err() {
        return Err(expanded_target_dir.err().unwrap());
    }
    let expanded_target_dir = expanded_target_dir.unwrap();

    let result = fs::remove_dir_all(expanded_target_dir);

    if result.is_err() {
        return Err(result.err().unwrap().to_string());
    }

    return Ok(());
}

// -- tests --

#[cfg(test)]
mod test {
    use std::fs::File;

    use super::*;
    use tempfile::{tempdir, tempfile_in, NamedTempFile};

    #[test]
    fn it_fails_when_src_dir_doesnt_exist() {
        let test = copy_dir("invalid", "invalid", vec![]);

        assert!(copy_dir("invalid", "invalid", vec![])
            .unwrap_err()
            .contains("path is not a dir when resolving"));
    }

    #[test]
    fn it_fails_when_dirs_are_the_same() {
        let dir = tempdir().unwrap();
        let src_path = dir.path();
        let src_file = tempfile_in(&src_path).unwrap();
        let src = src_path.to_str().unwrap();

        assert!(copy_dir(src, src, vec![])
            .unwrap_err()
            .contains("Source and destination directories are the same"));
    }

    // FIXME: this test fails for some reason (error is thrown outside of tests correctly)
    #[test]
    fn it_fails_when_dest_file_exists() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path();
        let src_file = NamedTempFile::new_in(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();
        let dest_file = dest_dir.path().join(src_file.path().file_name().unwrap());

        let dest_file_path = Path::new(&dest_file);
        File::create(&dest_file_path);

        assert!(copy_dir(src, dest, vec![])
            .unwrap_err()
            .contains("Destination file already exists"));
    }

    #[test]
    fn it_copies_files() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path();
        let src_file = NamedTempFile::new_in(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        assert!(copy_dir(src, dest, vec![]).is_ok());

        let dest_file = dest_dir.path().join(src_file.path().file_name().unwrap());

        assert!(dest_file.exists());
    }

    #[test]
    fn it_removes_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        assert!(remove_dir(path).is_ok());
        assert!(!dir.path().exists());
    }
}

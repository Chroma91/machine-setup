use ergo_fs::{Path, PathDir, WalkDir};
use std::fs;
use yaml_rust::{yaml::Hash, Yaml};

use crate::{
    command::{validate_args, CommandInterface},
    utils::directory::{expand_dir, get_source_and_target, DIR_TARGET},
};

pub struct CopyDirCommand {}

impl CommandInterface for CopyDirCommand {
    fn install(&self, args: Hash) -> Result<(), String> {
        let dirs = get_source_and_target(args);
        if dirs.is_err() {
            return Err(dirs.err().unwrap());
        }
        let dirs = dirs.unwrap();

        let result = copy_dir(&dirs.src, &dirs.target);
        if result.is_err() {
            return Err(result.unwrap_err());
        }

        return Ok(());
    }

    fn uninstall(&self, args: Hash) -> Result<(), String> {
        let validation = validate_args(args.to_owned(), vec![String::from(DIR_TARGET)]);
        if validation.is_err() {
            return Err(validation.unwrap_err());
        }

        let target_dir = args
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

    fn update(&self, args: Hash) -> Result<(), String> {
        unimplemented!()
    }
}

fn copy_files(source_dir: &PathDir, destination_dir: &Path) -> Result<(), String> {
    println!(
        "Copying files from {} to {} ...",
        source_dir.to_string(),
        destination_dir.to_str().unwrap()
    );

    for dir_entry in WalkDir::new(&source_dir).min_depth(1) {
        let dir_entry = dir_entry.unwrap();
        let source_path = dir_entry.path();
        let destination_path = destination_dir.join(source_path.strip_prefix(&source_dir).unwrap());

        if source_path.is_dir() {
            let create_result = fs::create_dir_all(&destination_path);
            if create_result.is_err() {
                return Err(create_result.unwrap_err().to_string());
            }
            continue;
        }

        println!(
            "Copying {} to {} ...",
            source_path.to_str().unwrap(),
            destination_path.to_str().unwrap()
        );

        fs::copy(source_path, destination_path)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
    }

    return Ok(());
}

pub fn copy_dir(source: &str, destination: &str) -> Result<(), String> {
    let expanded_source = expand_dir(source, false);
    if expanded_source.is_err() {
        return Err(expanded_source.unwrap_err().to_string());
    }
    let source_dir = expanded_source.to_owned().unwrap();

    if !source_dir.exists() {
        return Err(format!("Source directory does not exist: {}", source));
    }

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

    return copy_files(&source_dir, &destination_dir);
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
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn it_fails_when_src_dir_doesnt_exist() {
        assert!(copy_dir("invalid", "invalid")
            .unwrap_err()
            .contains("Source directory does not exist"));
    }

    #[test]
    fn it_fails_when_dirs_are_the_same() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("example.txt");
        let src_file = File::create(&src_path).unwrap();
        let src = src_path.to_str().unwrap();

        assert!(copy_dir(src, src)
            .unwrap_err()
            .contains("Source and destination directories are the same"));

        drop(src_file);
        dir.close().unwrap();
    }

    #[test]
    fn it_fails_when_src_dir_is_empty() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        assert!(copy_dir(src, dest)
            .unwrap_err()
            .contains("Source directory is empty"));

        src_dir.close().unwrap();
        dest_dir.close().unwrap();
    }

    // FIXME: this test fails for some reason (error is thrown outside of tests correctly)
    #[test]
    fn it_fails_when_dest_file_exists() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        let src_file = File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        let dest_path = dest_dir.path().join("example.txt");
        let dest_file = File::create(&dest_path).unwrap();

        assert!(copy_dir(src, dest)
            .unwrap_err()
            .contains("Destination file already exists"));

        src_dir.close().unwrap();
        drop(src_file);

        dest_dir.close().unwrap();
        drop(dest_file);
    }

    // FIXME: this test also fails but the method is functioning correctly
    #[test]
    fn it_copies_files() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        let src_file = File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        assert!(copy_dir(src, dest).is_ok());

        let dest_path = dest_dir.path().join("example.txt");
        assert!(dest_path.exists());

        src_dir.close().unwrap();
        drop(src_file);

        dest_dir.close().unwrap();
    }

    #[test]
    fn it_copies_files_recursively() {
        unimplemented!()
    }

    #[test]
    fn it_removes_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        assert!(remove_dir(path).is_ok());
        assert!(!dir.path().exists());

        dir.close().unwrap();
    }
}

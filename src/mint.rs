//! Used to create goldenfiles.

use std::env;
use std::fs;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::thread;

use tempdir::TempDir;

use differs::*;

/// A Mint creates goldenfiles.
///
/// When a Mint goes out of scope, it will do one of two things depending on the
/// value of the `REGENERATE_GOLDENFILES` environment variable:
///
///   1. If `REGENERATE_GOLDENFILES!=1`, it will check the new goldenfile
///      contents against their old contents, and panic if they differ.
///   2. If `REGENERATE_GOLDENFILES=1`, it will replace the old goldenfile
///      contents with the newly written contents.
pub struct Mint {
    path: PathBuf,
    tempdir: TempDir,
    files: Vec<(PathBuf, Differ)>,
}

impl Mint {
    /// Create a new goldenfile Mint.
    ///
    /// All goldenfiles will be created in the Mint's directory.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let tempdir = TempDir::new("rust-goldenfiles").unwrap();
        let mint = Mint {
            path: path.as_ref().to_path_buf(),
            files: vec![],
            tempdir: tempdir,
        };
        fs::create_dir_all(&mint.path).expect(&format!(
            "Failed to create goldenfile directory {:?}",
            mint.path
        ));
        return mint;
    }

    /// Create a new goldenfile.
    ///
    /// The returned file is actually a temporary file, not the goldenfile
    /// itself. When the Mint goes out of scope, it will either check the temp
    /// file against the real goldenfile, or replace the real goldenfile based
    /// on the value of the `REGENERATE_GOLDENFILES` environment variable.
    pub fn new_goldenfile<P: AsRef<Path>>(&mut self, path: P) -> Result<File> {
        if path.as_ref().is_absolute() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Path must be relative.",
            ));
        }

        let abs_path = self.tempdir.path().to_path_buf().join(path.as_ref());
        let maybe_file = File::create(abs_path.clone());
        if maybe_file.is_ok() {
            // TODO: Use other differs for different file extensions.
            let differ = Box::new(text_diff);
            self.files.push((path.as_ref().to_path_buf(), differ));
        }
        maybe_file
    }

    /// Check new goldenfile contents against old, and panic if they differ.
    ///
    /// This is called automatically when a Mint goes out of scope and
    /// `REGENERATE_GOLDENFILES!=1`.
    pub fn check_goldenfiles(&self) {
        for &(ref file, ref differ) in &self.files {
            let old = self.path.join(&file);
            let new = self.tempdir.path().join(&file);

            println!("\nGoldenfile diff for {:?}:", file.to_str().unwrap());
            println!("To regenerate the goldenfile, run");
            println!("    env REGENERATE_GOLDENFILES=1 cargo test");
            println!("------------------------------------------------------------");
            differ(&old, &new);
            println!("<NO DIFFERENCE>");
        }
    }

    /// Overwrite old goldenfile contents with their new contents.
    ///
    /// This is called automatically when a Mint goes out of scope and
    /// `REGENERATE_GOLDENFILES=1`.
    pub fn update_goldenfiles(&self) {
        for &(ref file, _) in &self.files {
            let old = self.path.join(&file);
            let new = self.tempdir.path().join(&file);

            println!("Updating {:?}.", file.to_str().unwrap());
            fs::copy(&new, &old).expect(&format!("Error copying {:?} to {:?}.", &new, &old));
        }
    }
}

impl Drop for Mint {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }
        let regen_var = env::var("REGENERATE_GOLDENFILES");
        if regen_var.is_ok() && regen_var.unwrap() == "1" {
            self.update_goldenfiles();
        } else {
            self.check_goldenfiles();
        }
    }
}

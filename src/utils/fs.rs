use std::{path::PathBuf, fs};

use color_eyre::Result;
use fs_extra::{dir::copy as copy_directory, file::copy as copy_file};


pub fn copy_things(a: &PathBuf, b: &PathBuf) -> Result<()> {
        if a.is_dir() {
            fs::create_dir_all(b)?;
        } else {
            let parent = b.parent().unwrap();
            fs::create_dir_all(parent)?;
        }

        let result = if a.is_dir() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;
            options.content_only = true;
            // copy it over
            copy_directory(a, b, &options)
        } else {
            // if it's a file, copy that over instead
            let mut options = fs_extra::file::CopyOptions::new();
            options.overwrite = true;
            copy_file(a, b, &options)
        };

        result?;
        Ok(())
    }
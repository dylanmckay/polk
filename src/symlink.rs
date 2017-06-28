use Dotfile;

use std::path::PathBuf;
use std::{io, fs, env};
use std::os::unix;

/// Creates a symlink to a dotfile.
pub fn build(dotfile: &Dotfile) -> Result<(), io::Error> {
    let dest_path = self::dotfile_path(dotfile);

    // If the dotfile is in a subdirectory, we need to
    // create the subdirectory inside the home directory
    // for the symlink to live in.
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    unix::fs::symlink(&dotfile.full_path, &dest_path)?;

    Ok(())
}

/// Destroys the symlink to a dotfile.
pub fn destroy(dotfile: &Dotfile) -> Result<(), io::Error> {
    use std::io::ErrorKind::NotFound;

    let dest_path = self::dotfile_path(dotfile);

    match fs::remove_file(&dest_path) {
        // No point complaining if the symlink is already gone.
        Err(ref e) if e.kind() == NotFound => Ok(()),
        result => result,
    }

    // TODO: We could clean up any subdirectories which only
    // contained symlinks and are now empty. Not very useful in
    // real life use and could be too aggressive.
}

/// Gets the path where where the dotfile symlink should live.
fn dotfile_path(dotfile: &Dotfile) -> PathBuf {
    // let home_dir = env::home_dir().expect("user has no home directory");
    let home_dir = env::current_dir().unwrap().join("dotfiles");
    home_dir.join(&dotfile.relative_path)
}


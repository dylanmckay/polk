use {Dotfile, Error};

use std::path::{PathBuf, Path};
use std::fs;
use std::os::unix;

/// Creates a symlink to a dotfile.
pub fn build(dotfile: &Dotfile) -> Result<(), Error> {
    let dest_path = self::path(dotfile);

    if dest_path.exists() {
        if dest_path.is_dir() {
            warn!("there is an existing directory at '{}', will not create symlink", dest_path.display());
            return Ok(());
        } else {
            let metadata = fs::symlink_metadata(&dest_path)?;

            if metadata.file_type().is_symlink() {
                let current_target = fs::read_link(&dest_path)?;

                if current_target.canonicalize()? == dotfile.full_path.canonicalize()? {
                    // No harm in recreating a symlink to the same location.
                    fs::remove_file(&dest_path)?;
                } else {
                    ilog!("there is an existing symlink to a different file at '{}', deleting it", dest_path.display());
                }
            } else {
                warn!("there is an existing file at '{}', will not create symlink", dest_path.display());
                return Ok(());
            }
        }
    }

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
pub fn destroy(dotfile: &Dotfile) -> Result<(), Error> {
    use std::io::ErrorKind::NotFound;

    let dest_path = self::path(dotfile);

    match fs::remove_file(&dest_path) {
        Ok(..) => Ok(()),
        // No point complaining if the symlink is already gone.
        Err(ref e) if e.kind() == NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }

    // TODO: We could clean up any subdirectories which only
    // contained symlinks and are now empty. Not very useful in
    // real life use and could be too aggressive.
}

/// Checks if the symlink for a dotfile exists.
pub fn exists(dotfile: &Dotfile) -> Result<bool, Error> {
    let symlink_path = self::path(dotfile);

    if !symlink_path.exists() { return Ok(false); }

    let metadata = fs::symlink_metadata(&symlink_path)?;
    Ok(metadata.file_type().is_symlink())
}

/// Gets the path where where the dotfile symlink should live.
pub fn path(dotfile: &Dotfile) -> PathBuf {
    // let home_dir = env::home_dir().expect("user has no home directory");
    let home_dir = Path::new("/tmp/dotfiles");
    home_dir.join(&dotfile.relative_path)
}


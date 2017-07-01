use {SourceSpec, Dotfile, FeatureSet, Error, ResultExt};
use backend::{self, Backend};
use symlink;

use walkdir::WalkDir;
use toml;

use std::io::prelude::*;
use std::path::{self, Path, PathBuf};
use std::fs;

/// Files which should not be considered dotfiles.
pub const DOTFILE_FILE_BLACKLIST: &'static [&'static str] = &[
    ".gitignore",
    ".git", // Git worktrees have `.git` files.
];

/// Folders which we should not recurse into whilst searching for dotfiles.
pub const DIRECTORY_BLACKLIST: &'static [&'static str] = &[
    ".git",
];

/// The main cache directory.
pub struct Cache {
    /// The directory that contains the cache.
    pub path: PathBuf,
}

/// Cache for a particular user.
pub struct UserCache<'a> {
    cache: &'a Cache,
    username: String,
}

/// A manifest file for a user cache.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserManifest {
    /// The source of the dotfiles.
    pub source: SourceSpec,
}

impl Cache {
    /// Opens or creates a cache directory given a path.
    pub fn at(path: PathBuf) -> Result<Self, Error> {
        if path.exists() { Cache::open(path) } else { Cache::create(path) }
    }

    /// Opens an existing cache directory.
    pub fn open(path: PathBuf) -> Result<Self, Error> {
        assert!(path.exists(), "cache must exist before opening");
        assert!(path.is_dir(), "cache path must be a directory");

        Ok(Cache { path: path })
    }

    /// Creates a new cache directory.
    pub fn create(path: PathBuf) -> Result<Self, Error> {
        assert!(!path.exists(), "cache already exists in this directory");

        fs::create_dir_all(&path)?;
        Ok(Cache { path: path })
    }

    /// Clears all symlinks and deletes the cache.
    pub fn forget(self, _verbose: bool) -> Result<(), Error> {
        for mut user_cache in self.user_caches()? {
            // Don't be verbose because there will be many false positives.
            user_cache.unlink(true).chain_err(|| "could not forget symlinks")?;
        }

        if self.path.exists() {
            fs::remove_dir_all(&self.path).chain_err(|| "could not remote cache")?;
        }

        Ok(())
    }

    /// Gets all of the dotfile caches.
    pub fn user_caches(&self) -> Result<Vec<UserCache>, Error> {
        let mut usernames = Vec::new();

        if !self.users_path().exists() {
            return Ok(Vec::new());
        }

        for entry in fs::read_dir(&self.users_path())? {
            let path = entry?.path();

            if path.is_dir() {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                usernames.push(file_name.to_owned());
            }
        }

        Ok(usernames.into_iter().map(|name| self.user(name)).collect())
    }

    /// Gets a user-specifc cache.
    pub fn user<S>(&self, username: S) -> UserCache where S: Into<String> {
        UserCache { cache: self, username: username.into() }
    }

    /// Gets the `users` directory path.
    pub fn users_path(&self) -> PathBuf { self.path.join("users") }
}

impl<'a> UserCache<'a> {
    /// The path to the root of the user cache.
    pub fn base_path(&self) -> PathBuf { self.cache.users_path().join(&self.username) }

    /// Gets the path to the manifest file.
    fn manifest_path(&self) -> PathBuf {
        self.base_path().join("manifest.toml")
    }

    /// The path to the dotfiles subdirectory inside the cache.
    fn dotfiles_path(&self) -> PathBuf {
        self.base_path().join("dotfiles")
    }

    /// Fetches dotfiles *and* creates symlinks.
    pub fn setup(&mut self, source: &SourceSpec, verbose: bool) -> Result<(), Error> {
        self.grab(source, verbose)?;

        self.build_symlinks(verbose).chain_err(|| "could not build symlinks")
    }

    /// Downloadu dotfiles but does not create symlinks.
    pub fn grab(&mut self, source: &SourceSpec, _verbose: bool) -> Result<(), Error> {
        // Create the parent directory if it doesn't exist.
        if let Some(parent) = self.dotfiles_path().parent() {
            if !parent.exists() {
                fs::create_dir_all(&parent)?;
            }
        }

        backup::path(self.dotfiles_path(), || {
            // Create the manifest file and save it to disk.
            let manifest = UserManifest { source: source.clone() };
            manifest.save(&self.manifest_path()).chain_err(|| "could not save user cache manifest")?;

            // Set up the Git repository, etc
            backend::setup(&self.dotfiles_path(), manifest.source)?;
            Ok(())
        })
    }

    /// Checks whether we have grabbed dotfiles for the user.
    pub fn is_grabbed(&self) -> bool {
        // We always create a manifest file when grabbing up.
        self.manifest_path().exists()
    }

    /// Updates all of the dotfiles.
    pub fn update(&mut self, verbose: bool) -> Result<(), Error> {
        if !self.is_grabbed() {
            fatal!("cannot update, there are no dotfiles grabbed for this user");
        }

        let (manifest, mut backend) = self.open_manifest_backend()?;

        ilog!("updating dotfiles from {}", manifest.source.description());
        backend.update(verbose)
    }

    /// Rebuilds symbolic links for the user.
    pub fn link(&mut self, verbose: bool) -> Result<(), Error> {
        self.build_symlinks(verbose)
    }

    /// Deletes all symbolic links.
    pub fn unlink(&mut self, verbose: bool) -> Result<(), Error> {
        for dotfile in self.dotfiles()? {
            if symlink::exists(&dotfile)? {
                vlog!(verbose => "deleting {}", symlink::path(&dotfile).display());

                symlink::destroy(&dotfile)?;
            }
        }

        Ok(())
    }

    /// Gets all of the dotfiles in the cache.
    pub fn dotfiles(&self) -> Result<Vec<Dotfile>, Error> {
        let mut dotfiles = Vec::new();

        if !self.dotfiles_path().exists() {
            return Ok(Vec::new());
        }

        for entry in WalkDir::new(self.dotfiles_path()) {
            let entry = entry?;

            if !entry.path().is_file() { continue; }

            let file_name = entry.path().file_name().unwrap().to_str().unwrap().to_owned();

            // Check that none of the parent folders are blacklisted.
            let folder_blacklisted = entry.path().components().any(|comp| {
                if let path::Component::Normal(ref p) = comp {
                    DIRECTORY_BLACKLIST.iter().any(|bl| bl == p)
                } else {
                    false
                }
            });
            let file_blacklisted = DOTFILE_FILE_BLACKLIST.iter().any(|&bl| file_name == bl);

            if !folder_blacklisted && !file_blacklisted {
                dotfiles.push(Dotfile {
                    full_path: entry.path().to_owned(),
                    relative_path: entry.path().strip_prefix(&self.dotfiles_path()).unwrap().to_owned(),
                });
            }
        }

        Ok(dotfiles)
    }

    /// Gets the manifest.
    pub fn manifest(&self) -> Result<UserManifest, Error> {
        UserManifest::load(&self.manifest_path()).chain_err(|| "reading user manifest")
    }

    /// Gets the manifest and backend.
    fn open_manifest_backend(&self) -> Result<(UserManifest, Box<Backend>), Error> {
        let manifest = self.manifest()?;
        backend::open(&self.dotfiles_path(), manifest.source.clone()).map(|b| (manifest, b))
    }

    /// Creates all symlinks.
    fn build_symlinks(&mut self, verbose: bool) -> Result<(), Error> {
        let features = FeatureSet::current_system();

        for dotfile in self.dotfiles()? {
            if features.supports(&dotfile) {
                symlink::build(&dotfile)?;

                let symlink_path = symlink::path(&dotfile);
                vlog!(verbose => "created {} -> {}", dotfile.full_path.display(), symlink_path.display());
            } else {
                ilog!("ignoring '{}' because is is not supported by this machine",
                      dotfile.relative_path.display());
            }
        }

        Ok(())
    }
}

impl UserManifest {
    /// Loads the manifest from disk.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let mut file = fs::File::open(path)?;
        let mut manifest_toml = String::new();
        file.read_to_string(&mut manifest_toml)?;

        Ok(toml::from_str(&manifest_toml).expect("could not parse user manifest"))
    }

    /// Saves the manifest to disk.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        let manifest_toml = toml::to_string(self).expect("failed to create manifest toml");

        let mut file = fs::File::create(path)?;
        file.write_all(manifest_toml.as_bytes())?;
        Ok(())
    }
}

mod backup {
    use {Error, ResultExt};
    use std::path::Path;
    use std::{fs, env, time};

    /// Move a file to a temporary location and restore it
    /// in the event of an error.
    pub fn path<P,T,F>(path: P, f: F) -> Result<T, Error>
        where P: AsRef<Path>, F: FnOnce() -> Result<T, Error> {
        let path = path.as_ref();

        if !path.exists() {
            // Nothing to back up in that case.
            return f();
        }

        let file_name = path.file_name().expect("cannot have transactions on paths with no file name").to_str().unwrap();
        let temp_path = env::temp_dir().join(format!("{}-{}", file_name, random_token()));

        // We don't need special error handling here because if we fail to backup,
        // we won't even end up executing the function.
        backup_path(path, &temp_path)?;

        match f() {
            Ok(result) => Ok(result),
            // An error occurred, attempt to restore the path.
            Err(e) => match restore_path(path, &temp_path) {
                // Successfully restored the file, propagate error.
                Ok(..) => Err(e),
                // Failure during restoration, add more context and propagate up.
                Err(f) => Err(f).chain_err(|| format!("could not restore backed up file '{}'", path.display())),
            },
        }
    }

    /// Backup a path.
    fn backup_path(path: &Path, temp_path: &Path) -> Result<(), Error> {
        fs::rename(path, &temp_path)?;
        Ok(())
    }

    /// Restore a path.
    fn restore_path(path: &Path, temp_path: &Path) -> Result<(), Error> {
        // If somebody has put a file in our place, delete it.
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }

        fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Generates a random token text.
    fn random_token() -> String {
        let elapsed = time::SystemTime::now().elapsed().unwrap_or(time::Duration::from_millis(0));
        (!(elapsed.subsec_nanos() as u32)).to_string()
    }
}


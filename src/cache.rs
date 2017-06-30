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

    /// Gets a user-specifc cache.
    pub fn user<S>(&self, username: S) -> UserCache where S: Into<String> {
        UserCache { cache: self, username: username.into() }
    }
}

impl<'a> UserCache<'a> {
    /// The path to the root of the user cache.
    pub fn base_path(&self) -> PathBuf { self.cache.path.join("users").join(&self.username) }

    /// Gets the path to the manifest file.
    fn manifest_path(&self) -> PathBuf {
        self.base_path().join("manifest.toml")
    }

    /// The path to the dotfiles subdirectory inside the cache.
    fn dotfiles_path(&self) -> PathBuf {
        self.base_path().join("dotfiles")
    }

    /// Initializes cache for a user.
    pub fn setup(&mut self, source: &SourceSpec, verbose: bool) -> Result<(), Error> {
        // Clear the directory because it may already exist.
        // FIXME: we shoulnd't do this because initialisation may fail and we would
        // want to keep existing configuration.
        if self.dotfiles_path().exists() {
            fs::remove_dir_all(&self.dotfiles_path()).chain_err(|| "could not remove old dotfiles cache directory")?;
        }
        fs::create_dir_all(&self.dotfiles_path()).chain_err(|| "could not create old dotfiles cache directory")?;

        // Create the manifest file and save it to disk.
        let manifest = UserManifest { source: source.clone() };
        manifest.save(&self.manifest_path()).chain_err(|| "could not save user cache manifest")?;

        // Retrieve the backend because it will get built if it doesn't exist.
        let (_, _backend) = self.manifest_backend()?;

        self.build_symlinks(verbose).chain_err(|| "could not build symlinks")
    }

    /// Updates all of the dotfiles.
    pub fn update(&mut self, verbose: bool) -> Result<(), Error> {
        let (manifest, mut backend) = self.manifest_backend()?;

        ilog!("updating dotfiles from {}", manifest.source.description());
        backend.update(verbose)
    }

    /// Rebuilds symbolic links for the user.
    pub fn rehash(&mut self, verbose: bool) -> Result<(), Error> {
        self.build_symlinks(verbose)
    }

    /// Cleans out all dotfiles.
    pub fn clean(&mut self, verbose: bool) -> Result<(), Error> {
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
    fn manifest_backend(&self) -> Result<(UserManifest, Box<Backend>), Error> {
        let manifest = self.manifest()?;
        backend::from_source(&self.dotfiles_path(), manifest.source.clone()).map(|b| (manifest, b))
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


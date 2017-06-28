use {Source, SourceSpec};

use git2::Repository;

use std::path::PathBuf;
use std::{fs, io};

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

/// A single dotfile.
pub struct Dotfile
{
    /// The full on-disk path of the dotfile.
    pub full_path: PathBuf,
    /// The path of the dotfile relative to the users home directory.
    pub relative_path: PathBuf,
}

impl Cache {
    /// Opens or creates a cache directory given a path.
    pub fn at(path: PathBuf) -> Result<Self, io::Error> {
        if path.exists() { Cache::open(path) } else { Cache::create(path) }
    }

    /// Opens an existing cache directory.
    pub fn open(path: PathBuf) -> Result<Self, io::Error> {
        assert!(path.exists(), "cache must exist before opening");
        assert!(path.is_dir(), "cache path must be a directory");

        Ok(Cache { path: path })
    }

    /// Creates a new cache directory.
    pub fn create(path: PathBuf) -> Result<Self, io::Error> {
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
    fn path(&self) -> PathBuf { self.cache.path.join("users").join(&self.username) }

    /// Initializes cache for a user.
    pub fn initialize(&mut self, source: &SourceSpec) -> Result<(), io::Error> {
        // Clear the directory because it may already exist.
        if self.path().exists() {
            fs::remove_dir_all(&self.path())?;
            fs::create_dir_all(&self.path())?;
        }

        match source.canonical() {
            Source::Git { url } => self.initialize_via_git(&url),
        }
    }

    /// Rebuilds symbolic links for the user.
    pub fn rehash(&mut self) -> Result<(), io::Error> {
        unimplemented!();
    }

    fn initialize_via_git(&mut self, repository_url: &str) -> Result<(), io::Error> {
        let repo = match Repository::clone(repository_url, self.path()) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to clone: {}", e),
        };

        Ok(())
    }
}


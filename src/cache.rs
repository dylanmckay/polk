use {Source, SourceSpec, Dotfile, FeatureSet};
use symlink;
use term;

use git2::Repository;
use walkdir::WalkDir;

use std::path::{self, PathBuf};
use std::{fs, io};

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
    pub fn initialize(&mut self, source: &SourceSpec, verbose: bool) -> Result<(), io::Error> {
        // Clear the directory because it may already exist.
        // FIXME: we shoulnd't do this because initialisation may fail and we would
        // want to keep existing configuration.
        if self.path().exists() {
            fs::remove_dir_all(&self.path())?;
            fs::create_dir_all(&self.path())?;
        }

        match source.canonical() {
            Source::Git { url } => self.initialize_via_git(&url, verbose),
        }?;

        self.build_symlinks(verbose)
    }

    /// Rebuilds symbolic links for the user.
    pub fn rehash(&mut self, verbose: bool) -> Result<(), io::Error> {
        self.build_symlinks(verbose)
    }

    /// Cleans out all dotfiles.
    pub fn clean(&mut self, verbose: bool) -> Result<(), io::Error> {
        for dotfile in self.dotfiles()? {
            if verbose {
                println!("deleting {}", symlink::path(&dotfile).display());
            }

            symlink::destroy(&dotfile)?;
        }

        Ok(())
    }

    /// Gets all of the dotfiles in the cache.
    pub fn dotfiles(&self) -> Result<Vec<Dotfile>, io::Error> {
        let mut dotfiles = Vec::new();

        for entry in WalkDir::new(self.path()) {
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
                    relative_path: entry.path().strip_prefix(&self.path()).unwrap().to_owned(),
                });
            }
        }

        Ok(dotfiles)
    }

    /// Creates all symlinks.
    fn build_symlinks(&mut self, verbose: bool) -> Result<(), io::Error> {
        let features = FeatureSet::current_system();

        for dotfile in self.dotfiles()? {
            if features.supports(&dotfile) {
                symlink::build(&dotfile)?;

                if verbose {
                    let mut t = term::stdout().unwrap();

                    let symlink_path = symlink::path(&dotfile);
                    print!("{}", dotfile.full_path.display());

                    t.fg(term::color::YELLOW)?;
                    print!(" -> {}", symlink_path.display());
                    t.reset()?;
                    println!();
                }
            } else {
                println!("ignoring '{}' because is is not supported by this machine",
                         dotfile.relative_path.display());
            }
        }

        Ok(())
    }


    fn initialize_via_git(&mut self, repository_url: &str, verbose: bool) -> Result<(), io::Error> {
        if verbose { println!("Cloning from Git repository at '{}' to '{}'", repository_url, self.path().display()); }

        let _repo = match Repository::clone(repository_url, self.path()) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to clone: {}", e),
        };

        if verbose { println!("Successfully cloned Git repository"); }

        Ok(())
    }
}


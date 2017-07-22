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
    pub username: String,
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
    pub fn manifest_path(&self) -> PathBuf {
        self.base_path().join("manifest.toml")
    }

    /// The path to the dotfiles subdirectory inside the cache.
    pub fn dotfiles_path(&self) -> PathBuf {
        self.base_path().join("dotfiles")
    }

    /// The home directory for custom shells.
    pub fn home_path(&self) -> PathBuf {
        self.base_path().join("home")
    }

    /// Fetches dotfiles *and* creates symlinks.
    pub fn setup(&mut self, source: &SourceSpec, verbose: bool) -> Result<(), Error> {
        self.grab(source, verbose).chain_err(|| "failed to grab dotfiles")?;

        self.link_ext(&symlink::Config::default(), verbose).
            chain_err(|| "could not build symlinks")
    }

    /// Download dotfiles but does not create symlinks.
    pub fn grab(&mut self, source: &SourceSpec, verbose: bool) -> Result<(), Error> {
        // Create the parent directory if it doesn't exist.
        if let Some(parent) = self.dotfiles_path().parent() {
            if !parent.exists() {
                vlog!(verbose => "{} does not exist, creating it", parent.display());
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

    /// Creates all symlinks.
    pub fn link(&mut self, verbose: bool) -> Result<(), Error> {
        self.link_ext(&symlink::Config::default(), verbose)
    }

    /// Creates all symlinks, with more options.
    pub fn link_ext(&mut self,
                    symlink_config: &symlink::Config,
                    verbose: bool) -> Result<(), Error> {
        let features = FeatureSet::current_system();

        for mut dotfile in self.dotfiles()? {
            if features.supports(&dotfile) {
                features.substitute_enabled_feature_names(&mut dotfile);
                symlink::build(&dotfile, &symlink_config)?;

                let symlink_path = symlink::path(&dotfile, &symlink_config);
                vlog!(verbose => "created {} -> {}", dotfile.full_path.display(), symlink_path.display());
            } else {
                ilog!("ignoring '{}' because is is not supported by this machine",
                      dotfile.relative_path.display());
            }
        }

        Ok(())
    }

    /// Deletes all symbolic links.
    pub fn unlink(&mut self, verbose: bool) -> Result<(), Error> {
        let symlink_config = symlink::Config::default();

        for dotfile in self.dotfiles()? {
            if symlink::exists(&dotfile, &symlink_config)? {
                vlog!(verbose => "deleting {}",
                      symlink::path(&dotfile, &symlink_config).display());

                symlink::destroy(&dotfile, &symlink_config)?;
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
            let mut is_submodule = false;
            let mut current_path = Some(entry.path().to_owned());

            while let Some(path) = current_path {
                // Stop looking once we hit the top level.
                if self.dotfiles_path() == path { break; }

                if path.join(".git").exists() {
                    is_submodule = true;
                    break;
                }

                current_path = path.parent().map(ToOwned::to_owned);
            }

            if !folder_blacklisted && !file_blacklisted && !is_submodule {
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
    use rand::random;
    use std::path::Path;
    use std::fs;

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
        let temp_path = path.clone().parent().unwrap_or(path).join(format!(".{}.{}.backup", file_name, random_token()));

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
        ilog!("backing up {} to {}", path.display(), temp_path.display());
        fs::rename(path, &temp_path).chain_err(|| "failed to move file to temporary path")?;
        Ok(())
    }

    /// Restore a path.
    fn restore_path(path: &Path, temp_path: &Path) -> Result<(), Error> {
        ilog!("restoring {} to {}", temp_path.display(), path.display());

        // If somebody has put a file in our place, delete it.
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(path).chain_err(|| "could not remove dirty directory")?;
            } else {
                fs::remove_file(path).chain_err(|| "could not remove dirty file")?;
            }
        }

        fs::rename(&temp_path, path).chain_err(|| "could not move backup file to original path")?;
        Ok(())
    }

    /// Generates a random token text.
    fn random_token() -> String {
        let (a,b): (u32,u32) = (random(), random());
        format!("{}{}", a, b)
    }
}

// FIXME: Get this test suite working.
#[cfg(sdfasfdf)]
mod test {
    use super::*;
    use SourceSpec;
    use symlink;

    use rand::random;
    use git2;

    use std::path::PathBuf;
    use std::env;

    lazy_static! {
        /// A path which contains a git repository with dotfiles.
        static ref DOTFILES_REPO_PATH: PathBuf = {
            let path = self::temp_directory();

            let mut repo = git2::Repository::init(&path).expect("failed to create repository");
            self::populate_dotfiles_repository(&mut repo, DOTFILES);
            path
        };

        static ref DOTFILES_SOURCE: SourceSpec = {
            let url = format!("file://{}", DOTFILES_REPO_PATH.display());
            SourceSpec::Url(url)
        };
    }

    /// Example dotfiles.
    const DOTFILES: &'static [(&'static str, &'static str)] = &[
        (".vimrc", "set ruler\nset sw=2\nset ts=2\n\n"),
        (".bashrc", "export PATH=/foo/bar:$PATH"),
    ];

    /// Get a temporary directory path.
    fn temp_directory() -> PathBuf {
        let (a,b): (u32,u32) = (random(), random());
        env::temp_dir().join(format!("cache-{}-{}", a, b))
    }

    /// Runs a function with a freshly-created cache.
    fn with_cache<F>(f: F)
        where F: FnOnce(&mut Cache) {
        clean_fake_home();
        let cache_path = temp_directory();

        let mut cache = Cache::create(cache_path).expect("could not create cache");
        f(&mut cache);

        destroy_cache(cache);
        clean_fake_home();
    }

    /// Runs a function with a freshly-created user cache.
    fn with_user_cache<F>(f: F)
        where F: FnOnce(&mut UserCache) {
        with_cache(|cache| {
            let mut user_cache = cache.user("jenny");
            f(&mut user_cache);
        })
    }

    /// Destroys a cache from disk.
    fn destroy_cache(cache: Cache) {
        fs::remove_dir_all(cache.path).expect("could not destroy cache");
    }

    /// Cleans the fake home directory.
    fn clean_fake_home() {
        let home = ::util::home_dir();
        if home.exists() {
            fs::remove_dir_all(&home).expect("could not clean fake home");
        }
        fs::create_dir_all(&home).ok();
    }

    /// Populates a repository with dotfiles.
    fn populate_dotfiles_repository(repo: &git2::Repository,
                                    dotfiles: &'static [(&'static str, &'static str)])  {
        use std::fs::File;
        let sig = repo.signature().unwrap();

        let tree_id = {
            let mut index = repo.index().unwrap();

            for &(file_name, content) in dotfiles.iter() {
                let repo_dir = repo.workdir().unwrap();
                let file_path = repo_dir.join(file_name);
                let repo_relative_path = file_path.strip_prefix(&repo_dir).unwrap();

                let mut file = File::create(&file_path).unwrap();
                file.write_all(content.as_bytes()).unwrap();
                drop(file);

                index.add_path(&repo_relative_path).unwrap();
            }

            index.write_tree().unwrap()
        };

        let tree = repo.find_tree(tree_id).unwrap();

        // Ready to create the initial commit.
        //
        // Normally creating a commit would involve looking up the current HEAD
        // commit and making that be the parent of the initial commit, but here this
        // is the first commit so there will be no parent.
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
    }

    /// Gets the symlinking configuration.
    fn symlink_config() -> symlink::Config {
        symlink::Config {
            home_path: ::util::home_dir(),
        }
    }

    /// Gets all dotfiles that have been symlinked.
    fn symlinked_dotfiles(user_cache: &UserCache) -> Vec<Dotfile> {
        let config = symlink_config();

        user_cache.dotfiles().unwrap().into_iter().filter(|dotfile| {
            symlink::exists(&dotfile, &config).unwrap()
        }).collect()
    }

    #[test]
    fn grab_works_as_expected() {
        with_user_cache(|user_cache| {
            assert!(user_cache.dotfiles().unwrap().is_empty(), "empty cache should not have dotfiles");
            assert!(symlinked_dotfiles(&user_cache).is_empty(), "empty cache should have no symlinked dotfiles");

            // Ensure that grabbing dotfiles gets all expected dotfiles.
            user_cache.grab(&DOTFILES_SOURCE, false).unwrap();
            assert_eq!(user_cache.dotfiles().unwrap().len(), DOTFILES.len());
            // assert_eq!(symlinked_dotfiles(&user_cache), &[], "grabbing should not create symlinks");

            // Ensure file names are correct.
            for dotfile in user_cache.dotfiles().unwrap() {
                let does_match = DOTFILES.iter().any(|&(file_name,_)| dotfile.relative_path == Path::new(file_name));
                assert!(does_match, "a file path in the cache does not match");
            }
        });
    }

    #[test]
    fn link_unlink_leaves_nothing() {
        with_user_cache(|user_cache| {
            assert_eq!(symlinked_dotfiles(&user_cache), &[]);
            // Ensure that grabbing dotfiles gets all expected dotfiles.
            user_cache.grab(&DOTFILES_SOURCE, false).unwrap();
            assert_eq!(user_cache.dotfiles().unwrap().len(), DOTFILES.len());
            // assert_eq!(symlinked_dotfiles(&user_cache), &[]);

            user_cache.link(false).unwrap();
            assert_eq!(symlinked_dotfiles(&user_cache).len(), DOTFILES.len(), "all symlinks should be created");
            user_cache.unlink(false).unwrap();
            assert!(symlinked_dotfiles(&user_cache).is_empty(), "all symlinks should be destroyed");
        });
    }

    #[test]
    fn link_without_grab_does_nothing() {
        with_user_cache(|user_cache| {
            user_cache.link(false).unwrap();
            assert!(symlinked_dotfiles(&user_cache).is_empty(), "no symlinks should exist");
        });
    }

    #[test]
    fn setup_creates_symlinks() {
        with_user_cache(|user_cache| {
            assert!(symlinked_dotfiles(&user_cache).is_empty(), "empty cache should have no symlinked dotfiles");

            // Ensure that grabbing dotfiles gets all expected dotfiles.
            user_cache.setup(&DOTFILES_SOURCE, false).unwrap();
            assert_eq!(user_cache.dotfiles().unwrap().len(), DOTFILES.len());
            assert_eq!(symlinked_dotfiles(&user_cache).len(), DOTFILES.len(),  "setup should create symlinks");

            // Ensure file names are correct.
            for dotfile in user_cache.dotfiles().unwrap() {
                let does_match = DOTFILES.iter().any(|&(file_name,_)| dotfile.relative_path == Path::new(file_name));
                assert!(does_match, "a file path in the cache does not match");
            }
        });
    }
}


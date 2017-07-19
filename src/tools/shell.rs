use {UserCache, Error, ResultExt};
use symlink;

use std::{fs, env};
use std::process::Command;
use std::os::unix::process::CommandExt;

/// Configuration for shell creation.
pub struct Config {
    pub shell_path: String,
}

/// A shell.
pub struct Shell<'a> {
    /// The user's cache.
    user_cache: &'a mut UserCache<'a>,
    /// Shell configuration.
    config: Config,
}

impl<'a> Shell<'a> {
    /// Creates a new shell for the user.
    pub fn create(user_cache: &'a mut UserCache<'a>, config: Config) -> Result<Self, Error> {
        let home_path = user_cache.home_path();
        if !home_path.exists() {
            fs::create_dir_all(&home_path)?;
        }

        let mut shell = Shell {
            user_cache: user_cache,
            config: config,
        };

        shell.build_symlinks()?;

        Ok(shell)
    }

    pub fn exec(&self) -> Result<(), Error> {
        let home_path = self.user_cache.home_path();

        let err: Error = Command::new(&self.config.shell_path)
            .current_dir(&home_path.display().to_string())
            .env("HOME", home_path.display().to_string())
            .exec()
            .into();

        Err(err).chain_err(|| "could not exec into shell")
    }

    /// Build all of the symlinks for the custom home directory.
    fn build_symlinks(&mut self) -> Result<(), Error> {
        let verbose = false;
        let home_path = self.user_cache.home_path();

        self.user_cache.link_ext(&symlink::Config {
            home_path: home_path,
        }, verbose)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            shell_path: env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()),
        }
    }
}


use {UserCache, Error};
use symlink;

use std::{fs, env};

/// Configuration for shell creation.
pub struct Config {
    pub shell: String,
}

/// A shell.
pub struct Shell<'a> {
    /// The user's cache.
    user_cache: &'a UserCache<'a>,
    /// Shell configuration.
    config: Config,
}

impl<'a> Shell<'a> {
    /// Creates a new shell for the user.
    pub fn create(user_cache: &'a UserCache, config: Config) -> Result<Self, Error> {
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

    fn build_symlinks(&mut self) -> Result<(), Error> {
        let symlink_config = symlink::Config {
            home_path: self.user_cache.home_path(),
        };

        for dotfile in self.user_cache.dotfiles()? {
            if !symlink::exists(&dotfile, &symlink_config)? {
                symlink::build(&dotfile, &symlink_config)?;
            }
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            shell: env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()),
        }
    }
}


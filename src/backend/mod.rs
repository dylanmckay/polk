pub mod git;

use {Source, Error};
use std::path::Path;

/// A dotfiles backend.
pub trait Backend {
    /// Updates the dotfiles.
    fn update(&mut self, verbose: bool) -> Result<(), Error>;
}

/// Creates a new backend from a source.
pub fn from_source<S>(dest: &Path, source: S) -> Result<Box<Backend>, Error>
    where S: Into<Source> {
    match source.into() {
        Source::Git { ref url } => {
            Ok(Box::new(git::Git::open_or_create(dest, url)?) as _)
        },
    }
}


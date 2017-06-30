pub mod git;

use {Source, Error};
use std::path::Path;

/// A dotfiles backend.
pub trait Backend {
    /// Updates the dotfiles.
    fn update(&mut self, verbose: bool) -> Result<(), Error>;
}

/// Initializes a new backend.
pub fn setup<S>(dest: &Path, source: S) -> Result<Box<Backend>, Error>
    where S: Into<Source> {
    match source.into() {
        Source::Git { ref url } => git::Git::setup(dest, url).map(|b| Box::new(b) as _),
    }
}

/// Opens an existing backend.
pub fn open<S>(path: &Path, source: S) -> Result<Box<Backend>, Error>
    where S: Into<Source> {
    match source.into() {
        Source::Git { .. } => git::Git::open(path).map(|b| Box::new(b) as _),
    }
}


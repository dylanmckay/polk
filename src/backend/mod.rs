pub mod git;

use Source;
use std::path::Path;
use std::io;

/// A dotfiles backend.
pub trait Backend {
    /// Updates the dotfiles.
    fn update(&mut self, verbose: bool) -> Result<(), io::Error>;
}

/// Creates a new backend from a source.
pub fn from_source<S>(dest: &Path, source: S) -> Result<Box<Backend>, io::Error>
    where S: Into<Source> {
    match source.into() {
        Source::Git { ref url } => {
            Ok(Box::new(git::Git::open_or_create(dest, url)?) as _)
        },
    }
}


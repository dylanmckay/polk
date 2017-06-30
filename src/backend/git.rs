use backend::Backend;

use git2::{self, Repository, Direction, AutotagOption, BranchType};
use git2::build::CheckoutBuilder;
use std::path::Path;
use std::io;

pub struct Git {
    /// The repository.
    repo: Repository,
}

impl Git {
    pub fn open(repo_path: &Path) -> Result<Git, io::Error> {
        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => fatal!("failed to clone: {}", e),
        };

        Ok(Git { repo: repo })
    }

    pub fn initialize(dest: &Path, source: &str) -> Result<Git, io::Error> {
        ilog!("cloning from Git repository at '{}' to '{}'", dest.display(), source);

        let repo = match Repository::clone(source, dest) {
            Ok(repo) => repo,
            Err(e) => fatal!("failed to clone: {}", e),
        };

        ilog!("successfully cloned Git repository");

        Ok(Git { repo: repo })
    }

    pub fn open_or_create(dest: &Path, source: &str) -> Result<Git, io::Error> {
        if dest.join(".git").exists() {
            Git::open(dest)
        } else {
            Git::initialize(dest, source)
        }
    }
}

impl Backend for Git {
    fn update(&mut self, _verbose: bool) -> Result<(), io::Error> {
        // FIXME: complain when worktree is dirty

        self::ensure_head_is_named_reference(&mut self.repo).unwrap();
        let mut original_head = self.repo.head().unwrap();
        assert!(original_head.is_branch(), "HEAD is not a branch");

        let branch_name = original_head.shorthand().unwrap().to_owned(); // TODO: Maybe use long version?

        let remote_name = self.repo.remotes().unwrap().iter().next().unwrap().unwrap().to_owned();
        let mut remote = self.repo.find_remote(&remote_name).unwrap();

        remote.connect(Direction::Fetch).unwrap();
        remote.download(&[], None).unwrap();
        remote.disconnect();

        remote.update_tips(None, true,
                           AutotagOption::Unspecified, None).unwrap();
        let remote_ref_name = format!("refs/remotes/{}/{}", remote_name, branch_name);
        let remote_ref = self.repo.find_reference(&remote_ref_name).unwrap();
        let current_head = original_head.set_target(remote_ref.target().unwrap(), "updating branch for new dotfiles").unwrap();

        let current_oid = &current_head.target().unwrap().to_string()[0..7];
        if original_head == current_head {
            ilog!("already up-to-date with {} at {}", branch_name, current_oid);
        } else {
            ilog!("updated `{}` to {}", branch_name, current_oid);
        }

        Ok(())
    }
}

fn ensure_head_is_named_reference(repo: &mut Repository) -> Result<(), git2::Error> {
    let head = repo.head()?;

    if head.is_branch() {
        Ok(())
    } else if head.is_note() {
        panic!("HEAD is a note!");
    } else if head.is_remote() {
        panic!("HEAD is a remote!");
    } else if head.is_tag() {
        panic!("HEAD is a tag!");
    } else {
        panic!("an arbitrary commit is checked out");
    }
}


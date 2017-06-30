use Error;
use backend::Backend;

use git2::{self, Repository, Direction, AutotagOption};
use std::path::Path;

pub struct Git {
    /// The repository.
    repo: Repository,
}

impl Git {
    pub fn open(repo_path: &Path) -> Result<Git, Error> {
        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => fatal!("failed to clone: {}", e),
        };

        Ok(Git { repo: repo })
    }

    pub fn initialize(dest: &Path, source: &str) -> Result<Git, Error> {
        ilog!("cloning from Git repository at '{}' to '{}'", dest.display(), source);

        let repo = match Repository::clone(source, dest) {
            Ok(repo) => repo,
            Err(e) => fatal!("failed to clone: {}", e),
        };

        ilog!("successfully cloned Git repository");

        Ok(Git { repo: repo })
    }

    pub fn open_or_create(dest: &Path, source: &str) -> Result<Git, Error> {
        if dest.join(".git").exists() {
            Git::open(dest)
        } else {
            Git::initialize(dest, source)
        }
    }
}

impl Backend for Git {
    fn update(&mut self, _verbose: bool) -> Result<(), Error> {
        // FIXME: complain when worktree is dirty

        self::ensure_head_is_named_reference(&mut self.repo).unwrap();
        let mut original_head = self.repo.head().unwrap();
        assert!(original_head.is_branch(), "HEAD is not a branch");

        let branch_name = original_head.shorthand().unwrap().to_owned();

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
        let original_oid = original_head.target().unwrap();
        let current_oid = current_head.target().unwrap();

        let current_oid_label = &current_oid.to_string()[0..7];
        if original_head == current_head {
            ilog!("already up-to-date with {} at {}", branch_name, current_oid_label);
        } else {
            // Build a revwalk over all new commits.
            let mut revwalk = self.repo.revwalk().unwrap();
            revwalk.push(current_oid).unwrap();
            revwalk.hide(original_oid).unwrap();

            ilog!("");
            ilog!("Commits");
            ilog!("-------");

            // Print a diff.
            for oid in revwalk {
                let oid = oid.unwrap();
                let commit = self.repo.find_commit(oid).unwrap();
                let oid_label = &oid.to_string()[..7];

                ilog!("{} {}", oid_label, commit.message().unwrap().trim());
            }

            ilog!("");

            ilog!("updated `{}` to {}", branch_name, current_oid_label);
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


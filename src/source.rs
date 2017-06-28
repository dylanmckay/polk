use std::str::FromStr;

/// The full URL to GitHub.
pub const GITHUB_URL: &'static str = "https://github.com";

/// The assumed name of a repository containing dotfiles.
pub const DEFAULT_GIT_REPOSITORY_NAME: &'static str = "dotfiles";

mod spec_matchers {
    use regex::Regex;

    lazy_static! {
        /// GitHub spec
        /// `github:<username>[/repository]`
        pub static ref GITHUB: Regex = Regex::new("github:(\\w+)/?(\\w+)?").unwrap();
    }
}

/// A source of dotfiles.
#[derive(Clone, Debug)]
pub enum SourceSpec {
    /// A GitHub dotfiles repository.
    GitHub {
        /// The username of the user that owns the dotfiles repository.
        username: String,
        /// The repository name that contains the dotfiles.
        /// If `None`, we will assume that the repository is named `dotfiles`.
        repository: Option<String>,
    },
    /// An arbitrary URL.
    Url(String),
}

/// A source of dotfiles.
#[derive(Clone, Debug)]
pub enum Source {
    Git {
        url: String,
    }
}

impl SourceSpec
{
    /// Gets the canonical source.
    pub fn canonical(&self) -> Source {
        match *self {
            SourceSpec::GitHub { ref username, ref repository } => {
                let repository = repository.as_ref().map(|r| r.to_owned()).
                    unwrap_or(DEFAULT_GIT_REPOSITORY_NAME.to_owned());

                let url = format!("{}/{}/{}.git", GITHUB_URL, username, repository);
                Source::Git { url: url }
            },
            SourceSpec::Url(ref url) => {
                Source::Git { url: url.clone() }
            },
        }
    }
}

impl FromStr for SourceSpec {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, &'static str> {
        if spec_matchers::GITHUB.is_match(s) {
            let captures = spec_matchers::GITHUB.captures(s).unwrap();
            let username = captures.get(1).unwrap().as_str().to_owned();
            let repository = captures.get(2).map(|m| m.as_str().to_owned());

            Ok(SourceSpec::GitHub { username: username, repository: repository })
        } else {
            // Assume URL if nothing else.
            Ok(SourceSpec::Url(s.to_owned()))
        }
    }
}


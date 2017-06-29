use serde;

use std::fmt::{self, Write};
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

    /// Gets a human readable description of the spec.
    pub fn description(&self) -> String {
        let mut d = String::new();

        match *self {
            SourceSpec::GitHub { ref username, ref repository } => {
                write!(d, "the GitHub repository owned by '{}' ", username).unwrap();

                if let Some(ref repo) = *repository {
                    write!(d, "named '{}'", repo).unwrap();
                } else {
                    write!(d, ", assuming repository named '{}'", DEFAULT_GIT_REPOSITORY_NAME).unwrap();
                }
            },
            SourceSpec::Url(ref url) => {
                write!(d, "the url at {}", url).unwrap();
            },
        }

        d
    }
}

impl fmt::Display for SourceSpec {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SourceSpec::GitHub { ref username, ref repository } => {
                write!(fmt, "github:{}", username)?;

                if let Some(ref repo) = *repository {
                    write!(fmt, "/{}", repo)?;
                }
            },
            SourceSpec::Url(ref url) => {
                write!(fmt, "{}", url)?;
            },
        }

        Ok(())
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

impl serde::Serialize for SourceSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SourceSpec {
    fn deserialize<S>(deserializer: S) -> Result<Self, S::Error>
        where S: serde::Deserializer<'de> {
        use serde::de::Error;
        let spec_str = String::deserialize(deserializer)?;

        match spec_str.parse() {
            Ok(spec) => Ok(spec),
            Err(e) => Err(S::Error::custom(e.to_string())),
        }

    }
}


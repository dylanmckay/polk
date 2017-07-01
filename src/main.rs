extern crate clap;
extern crate git2;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate walkdir;
extern crate term;
extern crate toml;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;

pub use self::cache::{Cache, UserCache};
pub use self::source::{Source, SourceSpec};
pub use self::feature::FeatureSet;
pub use self::errors::{Error, ErrorKind, ResultExt};

#[macro_use]
pub mod log;
pub mod cache;
pub mod source;
pub mod symlink;
pub mod feature;
pub mod backend;
pub mod shell;
pub mod errors;

/// A single dotfile.
pub struct Dotfile
{
    /// The full on-disk path of the dotfile.
    pub full_path: PathBuf,
    /// The path of the dotfile relative to the users home directory.
    pub relative_path: PathBuf,
}

use clap::{Arg, App, SubCommand};

use std::path::PathBuf;
use std::env;

fn open_cache() -> Result<Cache, Error> {
    let path = env::home_dir().expect("user does not have home directory").
        join(".polk");

    Cache::at(path.to_owned())
}

/// Gets the username of the current user.
fn system_username() -> String {
    match env::var("USER") {
        Ok(username) => username,
        Err(e) => fatal_error!(e, "could not get username"),
    }
}

mod arg {
    use clap::Arg;

    pub fn dotfile_source<'a,'b>() -> Arg<'a,'b> {
        Arg::with_name("SOURCE")
            .help("Sets the source of the dotfiles")
            .required(true)
            .index(1)
    }

    pub fn username<'a,'b>() -> Arg<'a,'b> {
        Arg::with_name("user")
            .short("u")
            .long("user")
            .value_name("USERNAME")
            .help("The username associated with the dotfiles\nDefaults to your computer username")
    }
}

fn polk() -> Result<(), Error> {
    let cache = open_cache()?;

    let matches = App::new("Polk")
                          .version(env!("CARGO_PKG_VERSION"))
                          .author(env!("CARGO_PKG_AUTHORS"))
                          .about(env!("CARGO_PKG_DESCRIPTION"))
                          .arg(Arg::with_name("verbose")
                               .short("v")
                               .long("verbose")
                               .help("Enables verbose output"))
                          .subcommand(SubCommand::with_name("grab")
                                      .arg(arg::username())
                                      .arg(arg::dotfile_source())
                                      .about("Downloads dotfiles but does not create symlinks to them"))
                          .subcommand(SubCommand::with_name("setup")
                                      .arg(arg::username())
                                      .arg(arg::dotfile_source())
                                      .about("Fetches dotfiles and creates symlinks to them"))
                          .subcommand(SubCommand::with_name("update")
                                      .arg(arg::username())
                                      .about("Updates dotfiles via the internet"))
                          .subcommand(SubCommand::with_name("link")
                                      .arg(arg::username())
                                      .about("Creates symbolic links to dotfiles"))
                          .subcommand(SubCommand::with_name("unlink")
                                      .about("Deletes all symbolic links"))
                          .subcommand(SubCommand::with_name("shell")
                                      .arg(arg::username())
                                      .about("Open up a shell with a temporary $HOME and the given users dotfiles"))
                          .subcommand(SubCommand::with_name("forget")
                                      .about("Deletes all symbolic links and cached dotfiles files"))
                          .subcommand(SubCommand::with_name("info")
                                      .about("List information"))
                          .get_matches();

    let verbose = matches.is_present("verbose");
    let mut term = term::stdout().expect("could not open stdout for term library");

    let username = if let Some(cmd_matches) = matches.subcommand().1 {
        cmd_matches.value_of("user").map(ToOwned::to_owned).unwrap_or_else(|| system_username())
    } else {
        system_username()
    };

    match matches.subcommand() {
        ("", None) => {
            fatal!("please enter a subcommand");
        },
        ("grab", Some(cmd_matches)) |
        ("setup", Some(cmd_matches)) => {
            let mut user_cache = cache.user(username);

            let subcommand = matches.subcommand().0;

            // Gets a value for config if supplied by user, or defaults to "default.conf"
            let source_str = cmd_matches.value_of("SOURCE").unwrap();
            let source_spec: SourceSpec = source_str.parse()?;

            vlog!(verbose => "Getting dotfiles from {}", source_spec.description());

            match subcommand {
                "grab" => user_cache.grab(&source_spec, verbose)?,
                "setup" => user_cache.setup(&source_spec, verbose)?,
                _ => unreachable!(),
            }
        },
        ("update", _) => {
            let mut user_cache = cache.user(username);
            user_cache.update(verbose)?;
        },
        ("link", _) => {
            let mut user_cache = cache.user(username);
            user_cache.link(verbose)?;
        },
        ("unlink", _) => {
            let mut user_cache = cache.user(username);
            user_cache.unlink(verbose)?;
        },
        ("shell", _) => {
            let mut user_cache = cache.user(username);
            let config = shell::Config::default();

            let shell = shell::Shell::create(&mut user_cache, config)?;
            shell.exec()?;
        },
        ("forget", _) => {
            cache.forget(verbose)?;
        },
        ("info", _) => {
            let user_cache = cache.user(username);
            let features = feature::FeatureSet::current_system();

            info::print_features(&features)?;
            info::print_configuration(&user_cache)?;
            info::print_dotfiles(user_cache.dotfiles()?, &mut *term)?;
        },
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    match polk() {
        Ok(..) => (),
        Err(e) => {
            fatal_error!(e);
        },
    }
}

mod info {
    use {Dotfile, FeatureSet, UserCache, Error};
    use {symlink, feature};

    use term::StdoutTerminal;
    use term;

    pub fn print_features(features: &FeatureSet) -> Result<(), Error> {
        let mut enabled_features: Vec<_> = features.enabled_features.iter().cloned().collect();
        let mut disabled_features: Vec<_> = features.disabled();
        enabled_features.sort();
        disabled_features.sort();

        println!("Enabled features\n----------------");
        for feature in enabled_features {
            println!("  + {}", feature);
        }
        println!();

        println!("Disabled features\n-----------------");
        for feature in disabled_features {
            println!("  - {}", feature);
        }
        println!();
        Ok(())
    }

    pub fn print_configuration(user_cache: &UserCache) -> Result<(), Error> {
        println!("Configuration\n-------------");
        println!("  user cache: {}", user_cache.base_path().display());
        println!();

        Ok(())
    }

    pub fn print_dotfiles<I>(dotfiles: I, term: &mut StdoutTerminal) -> Result<(), Error>
        where I: IntoIterator<Item=Dotfile> {
        println!("Dotfiles\n--------");

        let mut dotfiles: Vec<_> = dotfiles.into_iter().collect();
        dotfiles.sort_by_key(|d| d.relative_path.clone());
        let symlink_config = symlink::Config::default();

        for dotfile in dotfiles {
            let symlink_path = symlink::path(&dotfile, &symlink_config);
            let symlink_exists = symlink::exists(&dotfile, &symlink_config)?;
            let required_features: Vec<_> = feature::required_features(&dotfile).into_iter().collect();

            let bullet = if symlink_exists {
                term.fg(term::color::GREEN)?;
                "+"
            } else {
                term.fg(term::color::RED)?;
                "-"
            };

            print!("  {} ", bullet);

            term.reset()?;

            print!("{}", dotfile.full_path.display());

            if symlink_exists {
                term.fg(term::color::GREEN)?;
                print!(" -> {}", symlink_path.display());
                term.reset()?;
            }

            if !required_features.is_empty() {
                term.fg(term::color::YELLOW)?;
                print!(" requires: [{}]", required_features.join(", "));
                term.reset()?;
            }

            println!();
        }

        Ok(())
    }
}


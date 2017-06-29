extern crate clap;
extern crate git2;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate walkdir;
extern crate term;

pub use self::cache::Cache;
pub use self::source::{Source, SourceSpec};
pub use self::feature::FeatureSet;

#[macro_use]
pub mod log;
pub mod cache;
pub mod source;
pub mod symlink;
pub mod feature;

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

fn open_cache() -> Cache {
    let path = env::home_dir().expect("user does not have home directory").
        join(".dotty").join("cache");

    Cache::at(path.to_owned()).unwrap()
}

fn dotty() -> Result<(), ::std::io::Error> {
    let cache = open_cache();
    let mut user_cache = cache.user("dylan");

    let matches = App::new("Dotty")
                          .version(env!("CARGO_PKG_VERSION"))
                          .author(env!("CARGO_PKG_AUTHORS"))
                          .about(env!("CARGO_PKG_DESCRIPTION"))
                          .arg(Arg::with_name("verbose")
                               .short("v")
                               .long("verbose")
                               .help("Enables verbose output"))
                          .subcommand(SubCommand::with_name("setup")
                                      .about("Sets up dotfiles")
                                      .arg(Arg::with_name("SOURCE")
                                           .help("Sets the source of the dotfiles")
                                           .required(true)
                                           .index(1)))
                          .subcommand(SubCommand::with_name("rehash")
                                      .about("Recreates symbolic links to dotfiles"))
                          .subcommand(SubCommand::with_name("clean")
                                      .about("Clears all symbolic links"))
                          .subcommand(SubCommand::with_name("info")
                                      .about("List information"))
                          .get_matches();

    let verbose = matches.is_present("verbose");
    let mut term = term::stdout().unwrap();

    match matches.subcommand() {
        ("", None) => {
            fatal!("please enter a subcommand");
        },
        ("setup", Some(setup_matches)) => {
            // Gets a value for config if supplied by user, or defaults to "default.conf"
            let source_str = setup_matches.value_of("SOURCE").unwrap();
            let source_spec: SourceSpec = source_str.parse().unwrap();

            vlog!(verbose => "Getting dotfiles from {}", source_spec.description());

            user_cache.initialize(&source_spec, verbose)?;
        },
        ("rehash", _) => {
            user_cache.rehash(verbose)?;
        },
        ("clean", _) => {
            user_cache.clean(verbose)?;
        },
        ("info", _) => {
            let features = feature::FeatureSet::current_system();

            info::print_features(&features)?;
            info::print_dotfiles(user_cache.dotfiles()?, &mut *term)?;
        },
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    match dotty() {
        Ok(..) => (),
        Err(e) => {
            fatal!("{}", e);
        },
    }
}

mod info {
    use {Dotfile, FeatureSet};
    use {symlink, feature};

    use term::StdoutTerminal;
    use term;
    use std::io;

    pub fn print_features(features: &FeatureSet) -> Result<(), io::Error> {
        println!("Enabled features\n----------------");
        for feature in features.enabled_features.iter() {
            println!("  + {}", feature);
        }
        println!();

        println!("Disabled features\n-----------------");
        for feature in features.disabled() {
            println!("  - {}", feature);
        }
        println!();
        Ok(())
    }

    pub fn print_dotfiles<I>(dotfiles: I, term: &mut StdoutTerminal) -> Result<(), io::Error>
        where I: IntoIterator<Item=Dotfile> {
        println!("Dotfiles\n--------");

        for dotfile in dotfiles {
            let symlink_path = symlink::path(&dotfile);
            let symlink_exists = symlink::exists(&dotfile)?;
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


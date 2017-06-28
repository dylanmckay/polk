extern crate clap;
extern crate git2;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate walkdir;

pub use self::cache::Cache;
pub use self::source::{Source, SourceSpec};

pub mod cache;
pub mod source;
pub mod symlink;

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

    match matches.subcommand() {
        ("", None) => {
            eprintln!("please enter a subcommand");
        },
        ("setup", Some(setup_matches)) => {
            // Gets a value for config if supplied by user, or defaults to "default.conf"
            let source_str = setup_matches.value_of("SOURCE").unwrap();
            let source_spec: SourceSpec = source_str.parse().unwrap();

            println!("Setting up");
            println!("{:?}", source_spec);

            user_cache.initialize(&source_spec)?;
        },
        ("rehash", _) => {
            user_cache.rehash()?;
        },
        ("clean", _) => {
            user_cache.clean()?;
        },
        ("info", _) => {
            for dotfile in user_cache.dotfiles()? {
                println!("{}", dotfile.relative_path.display());
            }
        },
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    match dotty() {
        Ok(..) => (),
        Err(e) => {
            eprintln!("error: {}", e);
        },
    }
}


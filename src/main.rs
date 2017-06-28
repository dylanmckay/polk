extern crate clap;
extern crate git2;

pub use self::cache::Cache;
pub use self::source::{Source, SourceSpec};

pub mod cache;
pub mod source;

use clap::{Arg, App, SubCommand};
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
                          .get_matches();

    match matches.subcommand() {
        ("", None) => {
            eprintln!("please enter a subcommand");
        },
        ("setup", Some(setup_matches)) => {
            // Gets a value for config if supplied by user, or defaults to "default.conf"
            let source_str = setup_matches.value_of("SOURCE").unwrap();
            let source_spec: SourceSpec = source_str.parse().unwrap();

            user_cache.initialize(&source_spec).unwrap();
            println!("set up!");
        },
        ("rehash", _) => {
            user_cache.rehash()?;
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


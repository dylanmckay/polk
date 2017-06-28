extern crate git2;

pub use self::cache::Cache;
pub use self::source::{Source, SourceSpec};

pub mod cache;
pub mod source;

use std::env;

fn open_cache() -> Cache {
    let path = env::home_dir().expect("user does not have home directory").
        join(".dotty").join("cache");

    Cache::at(path.to_owned()).unwrap()
}

fn main() {
    let cache = open_cache();

    cache.user("dylan").initialize(&SourceSpec::Url("file:///Users/dylan/projects/dotfiles".to_owned())).unwrap();

    println!("Hello, world!");
}


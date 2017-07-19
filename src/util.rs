use std::path::PathBuf;
use std::env;

/// Gets the user's home directory.
pub fn home_dir() -> PathBuf {
    #[cfg(not(test))]
    {
        env::home_dir().expect("user does not have home directory")
    }

    #[cfg(test)]
    {
        use rand::random;

        lazy_static! {
            /// This is an example for using doc comment attributes
            static ref FAKE_HOME_DIR: PathBuf = {
                let (a,b): (u16,u16) = (random(), random());
                env::temp_dir().join(format!("home-{}-{}", a,b))
            };
        }

        FAKE_HOME_DIR.clone()
    }
}

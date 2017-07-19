use Dotfile;

use std::collections::HashSet;
use std::env::consts;

/// All architecture names.
///
/// Taken from the documentation of the `std::env::consts::OS` constant.
pub const OS_NAMES: &'static [&'static str] = &[
    "linux",
    "macos",
    "ios",
    "freebsd",
    "dragonfly",
    "bitrig",
    "netbsd",
    "openbsd",
    "solaris",
    "android",
    "windows",
];

/// All family names.
///
/// Taken from the documentation of the `std::env::consts::FAMILY` constant.
pub const FAMILIES: &'static [&'static str] = &[
    "unix",
    "windows",
];

/// All architecture names.
///
/// Taken from the documentation of the `std::env::consts::ARCH` constant.
pub const ARCH_NAMES: &'static [&'static str] = &[
    "x86",
    "x86_64",
    "arm",
    "aarch64",
    "mips",
    "mips64",
    "powerpc",
    "powerpc64",
    "s390x",
    "sparc64",
];

/// All features defined across all platforms.
pub const ALL_FEATURES: &'static [&'static [&'static str]] = &[
    OS_NAMES, FAMILIES, ARCH_NAMES,
];

/// A set of features.
#[derive(Debug)]
pub struct FeatureSet {
    pub enabled_features: HashSet<&'static str>,
}

impl FeatureSet {
    /// Gets the feature set for the current system.
    pub fn current_system() -> Self {
        let mut enabled_features = HashSet::new();

        enabled_features.insert(consts::OS);
        enabled_features.insert(consts::FAMILY);
        enabled_features.insert(consts::ARCH);

        FeatureSet::new(enabled_features)
    }

    /// Creates a new feature set.
    pub fn new(enabled_features: HashSet<&'static str>) -> Self {
        for feature in enabled_features.iter() {
            validate_feature(feature);
        }

        FeatureSet { enabled_features: enabled_features }
    }

    /// Checks if a dotfile is supported.
    pub fn supports(&self, dotfile: &Dotfile) -> bool {
        let required_features = self::required_features(dotfile);
        self.enabled_features.is_superset(&required_features)
    }

    /// Substitutes all features in a dotfile's relative path with
    /// the feature names.
    ///
    /// For example, `.tmux.linux.conf` would get resolved to `.tmux.os.conf`.
    pub fn substitute_enabled_feature_names(&self, dotfile: &mut Dotfile) {
        let mut file_name = dotfile.relative_path.file_name().unwrap().to_str().unwrap().to_owned();

        for feature_value in self.enabled_features.iter() {
            let feature_name = self::feature_name(feature_value);
            println!("s/{}/{}", feature_value, feature_name);
            file_name = file_name.replace(feature_value, feature_name);
        }

        dotfile.relative_path = dotfile.relative_path.with_file_name(file_name);
    }

    /// Gets a list of all disabled features.
    pub fn disabled(&self) -> Vec<&'static str> {
        ALL_FEATURES.iter().flat_map(|fs| fs.iter()).cloned().filter(|feature| {
            !self.enabled_features.contains(feature)
        }).collect()
    }
}

/// Builds a list of all required features for a dotfile.
pub fn required_features(dotfile: &Dotfile) -> HashSet<&'static str> {
    let file_name: String = dotfile.relative_path.file_name().unwrap().to_str().unwrap().to_owned();

    file_name.split('.').filter_map(|part| {
        for feature_set in ALL_FEATURES {
            if let Some(&feature) = feature_set.iter().find(|&e| e == &part) {
                return Some(feature)
            }
        }

        None
    }).collect()
}

/// Gets the name of a feature given its value
/// For example, `.tmux.linux.conf` -> `.tmux.os.conf`.
fn feature_name(value: &str) -> &'static str {
    if OS_NAMES.iter().any(|&o| o == value) { return "os" };
    if FAMILIES.iter().any(|&o| o == value) { return "family" };
    if ARCH_NAMES.iter().any(|&o| o == value) { return "arch" };

    panic!("unknown feature: '{}'", value);
}

/// Panics if a feature name isn't known to this module.
fn validate_feature(feature: &'static str) {
    for feature_set in ALL_FEATURES.iter() {
        if let Some(..) = feature_set.iter().find(|&f| f == &feature) {
            return;
        }
    }

    panic!("feature '{}' does not exist in the global feature set", feature);
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    static ENABLED_FEATURES: &'static [&'static str] = &["linux", "unix", "x86"];

    fn feature_set(features: &'static [&'static str]) -> FeatureSet {
        FeatureSet::new(features.into_iter().cloned().collect())
    }

    fn substitute(relative_path: &'static str) -> String {
        let feature_set = feature_set(ENABLED_FEATURES);

        let mut dotfile = Dotfile {
            full_path: Path::new("unused").to_owned(),
            relative_path: Path::new(relative_path).to_owned(),
        };
        feature_set.substitute_enabled_feature_names(&mut dotfile);

        dotfile.relative_path.to_str().unwrap().to_owned()
    }

    #[test]
    fn substitute_enabled_features_works() {
        assert_eq!(substitute("foo.bar"), "foo.bar");
        assert_eq!(substitute(".tmux.linux.conf"), ".tmux.os.conf");
        assert_eq!(substitute(".tmux.linux.x86.conf"), ".tmux.os.arch.conf");
        assert_eq!(substitute(".tmux.linux.unix.x86.conf"), ".tmux.os.family.arch.conf");
    }
}


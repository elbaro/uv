use std::str::FromStr;

use pep508_rs::PackageName;

#[derive(Debug, Clone)]
pub enum PackageNameSpecifier {
    All,
    None,
    Package(PackageName),
}

impl FromStr for PackageNameSpecifier {
    type Err = uv_normalize::InvalidNameError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            ":all:" => Ok(Self::All),
            ":none:" => Ok(Self::None),
            _ => Ok(Self::Package(PackageName::from_str(name)?)),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PackageNameSpecifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PackageNameSpecifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a package name or `:all:` or `:none:`")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Accept the special values `:all:` and `:none:`.
                match value {
                    ":all:" => Ok(PackageNameSpecifier::All),
                    ":none:" => Ok(PackageNameSpecifier::None),
                    _ => {
                        // Otherwise, parse the value as a package name.
                        match PackageName::from_str(value) {
                            Ok(name) => Ok(PackageNameSpecifier::Package(name)),
                            Err(err) => Err(E::custom(err)),
                        }
                    }
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Package name specification.
///
/// Consumes both package names and selection directives for compatibility with pip flags
/// such as `--no-binary`.
#[derive(Debug, Clone)]
pub enum PackageNameSpecifiers {
    All,
    None,
    Packages(Vec<PackageName>),
}

impl PackageNameSpecifiers {
    pub(crate) fn from_iter(specifiers: impl Iterator<Item = PackageNameSpecifier>) -> Self {
        let mut packages = Vec::new();
        let mut all: bool = false;

        for specifier in specifiers {
            match specifier {
                PackageNameSpecifier::None => {
                    packages.clear();
                    all = false;
                }
                PackageNameSpecifier::All => {
                    all = true;
                }
                PackageNameSpecifier::Package(name) => {
                    packages.push(name);
                }
            }
        }

        if all {
            Self::All
        } else if packages.is_empty() {
            Self::None
        } else {
            Self::Packages(packages)
        }
    }
}

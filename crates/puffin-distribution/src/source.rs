use std::path::PathBuf;

use anyhow::{anyhow, Error, Result};
use url::Url;

use puffin_git::Git;
use pypi_types::{ArchiveInfo, DirectUrl, VcsInfo, VcsKind};

use crate::RemoteDistributionRef;

/// The source of a distribution.
#[derive(Debug)]
pub enum Source<'a> {
    /// The distribution is available at a URL in a registry, like PyPI.
    RegistryUrl(Url),
    /// The distribution is available at an arbitrary remote URL, like a GitHub Release.
    RemoteUrl(&'a Url, Option<PathBuf>),
    /// The distribution is available in a remote Git repository.
    Git(Git, Option<PathBuf>),
}

impl<'a> TryFrom<&'a RemoteDistributionRef<'_>> for Source<'a> {
    type Error = Error;

    fn try_from(value: &'a RemoteDistributionRef<'_>) -> Result<Self, Self::Error> {
        match value {
            // If a distribution is hosted on a registry, it must be available at a URL.
            RemoteDistributionRef::Registry(_, _, file) => {
                Ok(Self::RegistryUrl(Url::parse(&file.url)?))
            }

            // If a distribution is specified via a direct URL, it could be a URL to a hosted file,
            // or a URL to a Git repository.
            RemoteDistributionRef::Url(_, url) => Self::try_from(*url),
        }
    }
}

impl<'a> TryFrom<&'a Url> for Source<'a> {
    type Error = Error;

    fn try_from(url: &'a Url) -> Result<Self, Self::Error> {
        // If the URL points to a subdirectory, extract it, as in:
        //   `https://git.example.com/MyProject.git@v1.0#subdirectory=pkg_dir`
        //   `https://git.example.com/MyProject.git@v1.0#egg=pkg&subdirectory=pkg_dir`
        let subdirectory = url.fragment().and_then(|fragment| {
            fragment
                .split('&')
                .find_map(|fragment| fragment.strip_prefix("subdirectory=").map(PathBuf::from))
        });

        // If a distribution is specified via a direct URL, it could be a URL to a hosted file,
        // or a URL to a Git repository.
        if let Some(url) = url.as_str().strip_prefix("git+") {
            let url = Url::parse(url)?;
            let git = Git::try_from(url)?;
            Ok(Self::Git(git, subdirectory))
        } else {
            Ok(Self::RemoteUrl(url, subdirectory))
        }
    }
}

impl From<Source<'_>> for Url {
    fn from(value: Source) -> Self {
        match value {
            Source::RegistryUrl(url) => url,
            Source::RemoteUrl(url, subdirectory) => {
                if let Some(subdirectory) = subdirectory {
                    let mut url = (*url).clone();
                    url.set_fragment(Some(&format!("subdirectory={}", subdirectory.display())));
                    url
                } else {
                    url.clone()
                }
            }
            Source::Git(git, subdirectory) => {
                let mut url = Url::parse(&format!("{}{}", "git+", Url::from(git).as_str()))
                    .expect("git url is valid");
                if let Some(subdirectory) = subdirectory {
                    url.set_fragment(Some(&format!("subdirectory={}", subdirectory.display())));
                }
                url
            }
        }
    }
}

impl TryFrom<Source<'_>> for DirectUrl {
    type Error = Error;

    fn try_from(value: Source<'_>) -> Result<Self, Self::Error> {
        match value {
            Source::RegistryUrl(_) => Err(anyhow!("Registry dependencies have no direct URL")),
            Source::RemoteUrl(url, subdirectory) => Ok(DirectUrl::ArchiveUrl {
                url: url.to_string(),
                archive_info: ArchiveInfo {
                    hash: None,
                    hashes: None,
                },
                subdirectory,
            }),
            Source::Git(git, subdirectory) => Ok(DirectUrl::VcsUrl {
                url: git.url().to_string(),
                vcs_info: VcsInfo {
                    vcs: VcsKind::Git,
                    // TODO(charlie): In `pip-sync`, we should `.precise` our Git dependencies,
                    // even though we expect it to be a no-op.
                    commit_id: git.precise().map(|oid| oid.to_string()),
                    requested_revision: git.reference().map(ToString::to_string),
                },
                subdirectory,
            }),
        }
    }
}
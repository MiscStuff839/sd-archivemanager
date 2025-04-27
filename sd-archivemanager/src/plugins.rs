use std::{io::Write, path::PathBuf};

use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt, SeekFrom},
};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use tokio::task;

use crate::{error::{Error, InvalidConfigSnafu, IoSnafu, TokioSnafu, XdgSnafu}, regex::Target};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub name: String,
    pub author: String,
    pub pre: bool,
    pub post: bool,
    pub target: Target,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManager {
    pub plugins: Option<Vec<PluginManifest>>,
}

impl PluginManager {
    pub fn get_plugins(&self, user: &str, target: Target) -> Vec<&PluginManifest> {
        match &self.plugins {
            None => vec![],
            Some(p) => {
                let mut vec: Vec<&PluginManifest> = p
                    .iter()
                    .filter(|p| {
                        (p.author
                            .split(',')
                            .map(|y| y.trim())
                            .collect::<Vec<_>>()
                            .contains(&user)
                            || &p.author == "*")
                            && target == p.target
                    })
                    .collect();
                vec.sort_by(|x, y| x.name.cmp(&y.name));
                vec
            }
        }
    }
    /// Saves current regex profiles to regex.toml
    pub async fn save(&self) -> Result<(), Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg
            .place_data_file("regex.toml")
            .context(IoSnafu {
                file: xdg
                    .get_data_home()
                    .join("regex.toml")
                    .to_str()
                    .unwrap()
                    .to_string(),
            })?
            .as_path()
            .to_owned();
        let file = task::spawn({
            let path = path.clone();
            async move {
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(path)
                    .await
                    .unwrap()
            }
        });
        if self.plugins.is_some() {
            file.await
                .context(TokioSnafu)?
                .write_all(toml::to_string(self).unwrap().as_bytes())
                .await
                .context(IoSnafu {
                    file: path.to_string_lossy().to_string(),
                })?;
        } else {
            let mut file = file.await.context(TokioSnafu)?;
            file.seek(SeekFrom::Start(1)).await.unwrap();
            file.write_all(&[0]).await.unwrap();
        }
        Ok(())
    }
    /// Loads regex profiles from regex.toml
    /// Should be used with [Result::unwrap_or_default]
    pub async fn load() -> Result<Self, Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg
            .find_data_file("regex.toml")
            .whatever_context("unable to find file")?;
        let profiles = toml::from_str::<Self>(
            fs::read_to_string(&path)
                .await
                .map_err(|e: std::io::Error| Error::IoError {
                    source: e,
                    file: path,
                })?
                .as_str(),
        )
        .context(InvalidConfigSnafu)?;
        Ok(profiles)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let path = xdg
            .place_data_file("plugins.toml")
            .context(IoSnafu {
                file: xdg
                    .get_data_home()
                    .join("plugins.toml")
                    .to_str()
                    .unwrap()
                    .to_string(),
            })
            .unwrap();
        let plgs = PluginManager { plugins: None };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        file.write_all(toml::to_string(&plgs).unwrap().as_bytes())
            .unwrap();
        plgs
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStage {
    Post,
    Pre,
}

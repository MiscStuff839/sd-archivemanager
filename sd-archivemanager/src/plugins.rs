use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use crate::error::{Error, InvalidConfigSnafu, IoSnafu, XdgSnafu};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub name: String,
    pub author: String,
    pub pre: bool,
    pub post: bool,
    pub target: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManager {
    pub plugins: Option<Vec<PluginManifest>>,
}

impl PluginManager {
    pub fn get_plugins(&self, user: &str) -> Vec<PluginManifest> {
        match &self.plugins {
            None => vec![],
            Some(p) => {
                let mut vec: Vec<PluginManifest> = p
                    .iter()
                    .filter(|p| {
                        p.author
                            .split(',')
                            .map(|y| y.trim())
                            .collect::<Vec<_>>()
                            .contains(&user)
                            || p.author == "*".to_string()
                    })
                    .cloned()
                    .collect();
                vec.sort_by(|x, y| x.name.cmp(&y.name));
                vec
            }
        }
    }
    pub fn save(&self) -> Result<(), Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg.place_data_file("plugins.toml").context(IoSnafu {
            file: xdg.get_data_home().join("plugins.toml"),
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .context(IoSnafu { file: &path })?;
        file.write_all(toml::to_string_pretty(self).unwrap().as_bytes())
            .context(IoSnafu { file: &path })?;
        Ok(())
    }
    pub fn load() -> Result<Self, Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg
            .find_data_file("plugins.toml")
            .whatever_context("unable to find file")?;
        let profiles = toml::from_str::<PluginManager>(
            fs::read_to_string(&path)
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
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        file.write_all(toml::to_string(&plgs).unwrap().as_bytes())
            .unwrap();
        plgs
    }
}

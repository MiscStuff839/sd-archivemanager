use crate::error::*;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use snafu::{ensure, ResultExt};
use std::{fs::OpenOptions, io::{self, Write}, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    wiki: PathBuf,
    token: String,
}

impl Config {
    pub fn set_wiki(&mut self, wiki: PathBuf) {
        self.wiki = wiki;
    }
    pub fn set_token(&mut self, token: String) {
        self.token = token;
    }
    pub fn get_wiki(&self) -> &PathBuf {
        &self.wiki
    }
    pub fn get_token(&self) -> &String {
        &self.token
    }
    pub fn save(&self) -> Result<(), Error> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let conf_file =
            xdg_dirs
                .place_config_file("config.toml")
                .map_err(|err| Error::IoError {
                    source: err,
                    file: xdg_dirs
                        .get_config_home()
                        .join("config.toml")
                        .to_string_lossy()
                        .to_string(),
                })?;
        let mut conf_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(conf_file)
            .map_err(|err| Error::IoError {
                source: err,
                file: xdg_dirs
                    .get_config_home()
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            })?;
        conf_file
            .write_all(toml::to_string(self).unwrap().as_bytes())
            .map_err(|err| Error::IoError {
                source: err,
                file: xdg_dirs
                    .get_config_home()
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            })?;
        Ok(())
    }
    pub fn load() -> Result<Config, Error> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let conf_file = xdg_dirs.find_config_file("config.toml");
        if conf_file.is_none() {
            return Err(Error::IoError {
                source: io::Error::new(io::ErrorKind::NotFound, "config.toml not found"),
                file: xdg_dirs
                    .get_config_home()
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            });
        }
        let conf_file = conf_file.unwrap();
        Ok(toml::from_str::<Config>(
            &std::fs::read_to_string(conf_file).map_err(|err| Error::IoError {
                source: err,
                file: xdg_dirs
                    .get_config_home()
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            })?,
        ).context(InvalidConfigSnafu)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let conf_file = xdg_dirs
            .place_config_file("config.toml")
            .expect("unable to place config file");
        let mut conf_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(conf_file)
            .expect("unable to open config file");
        let data_dir = xdg_dirs
            .create_data_directory("wiki")
            .expect("unable to create data directory");
        conf_file
            .write_all(
                format!(
                    "wiki = \"{}\"\ntoken = \"TO BE FILLED\"",
                    data_dir.to_string_lossy()
                )
                .as_bytes(),
            )
            .expect("unable to write to config file");
        println!(
            "Insert token in {}",
            xdg_dirs
                .find_config_file("config.toml")
                .unwrap()
                .to_string_lossy()
                .red()
        );
        Config {
            wiki: data_dir,
            token: "TO BE FILLED".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, str::FromStr};

    use super::*;
    #[test]
    fn create_default() {
        let config = super::Config::default();
        assert!(config.wiki.exists());
    }
    #[test]
    fn parse_config() {
        let config = super::Config::default();
        let config = toml::to_string(&config).unwrap();
        let config: super::Config = toml::from_str(&config).unwrap();
        assert_eq!(
            config,
            Config {
                wiki: PathBuf::from_str("/root/.local/share/sd-archivemanager/wiki").unwrap(),
                token: "TO BE FILLED".to_string()
            }
        );
        let config = Config::load().unwrap();
        assert_eq!(
            config,
            Config {
                wiki: PathBuf::from_str("/root/.local/share/sd-archivemanager/wiki").unwrap(),
                token: "TO BE FILLED".to_string()
            }
        );
        fs::remove_dir_all(config.wiki).unwrap();
        fs::remove_file("/root/.config/sd-archivemanager/config.toml").unwrap();
    }
}

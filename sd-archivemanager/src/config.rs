use crate::error::*;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub token: String,
    pub login: String,
    pub passwd: String,
    pub endpoint: String,
    pub bot: bool,
}

impl Config {
    pub fn set_token(&mut self, token: String) {
        self.token = token;
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
                    file: xdg_dirs.get_config_home().join("config.toml"),
                })?;
        let mut conf_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(conf_file)
            .map_err(|err| Error::IoError {
                source: err,
                file: xdg_dirs.get_config_home().join("config.toml"),
            })?;
        conf_file
            .write_all(toml::to_string(self).unwrap().as_bytes())
            .map_err(|err| Error::IoError {
                source: err,
                file: xdg_dirs.get_config_home().join("config.toml"),
            })?;
        Ok(())
    }
    pub fn load() -> Result<Self, Error> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let conf_file = xdg_dirs.find_config_file("config.toml");
        if conf_file.is_none() {
            return Err(Error::IoError {
                source: io::Error::new(io::ErrorKind::NotFound, "config.toml not found"),
                file: xdg_dirs.get_config_home().join("config.toml"),
            });
        }
        let conf_file = conf_file.unwrap();
        let mut config =
            toml::from_str::<Config>(&std::fs::read_to_string(conf_file).map_err(|err| {
                Error::IoError {
                    source: err,
                    file: xdg_dirs.get_config_home().join("config.toml"),
                }
            })?)
            .context(InvalidConfigSnafu)?;
        if config.token == "TO BE FILLED" {
            config.set_token(env::var("DISCORD_TOKEN").unwrap_or("TO BE FILLED".to_string()));
        }
        Ok(config)
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
        let config = Config {
            token: env::var("DISCORD_TOKEN").unwrap_or("TO BE FILLED".to_string()),
            passwd: env::var("PASSWD").unwrap_or("TO BE FILLED".to_string()),
            login: env::var("LOGIN").unwrap_or("TO BE FILLED".to_string()),
            endpoint: "https://qwrky.dev/mediawiki/api.php".to_string(),
            bot: true,
        };
        conf_file
            .write_all(
                toml::to_string(&config)
                    .expect("unable to serialize default config")
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
        config
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn create_default() {
        let config = super::Config::default();
        assert_eq!(
            config,
            Config {
                token: env::var("DISCORD_TOKEN").unwrap_or("TO BE FILLED".to_string()),
                passwd: env::var("PASSWD").unwrap_or("TO BE FILLED".to_string()),
                login: env::var("LOGIN").unwrap_or("TO BE FILLED".to_string()),
                endpoint: "https://qwrky.dev/mediawiki/api.php".to_string(),
                bot: true
            }
        );
    }
    #[test]
    fn parse_config() {
        let config = super::Config::default();
        let config = toml::to_string(&config).unwrap();
        let config: super::Config = toml::from_str(&config).unwrap();
        assert_eq!(
            config,
            Config {
                token: env::var("DISCORD_TOKEN").unwrap_or("TO BE FILLED".to_string()),
                passwd: env::var("PASSWD").unwrap_or("TO BE FILLED".to_string()),
                login: env::var("LOGIN").unwrap_or("TO BE FILLED".to_string()),
                endpoint: "https://qwrky.dev/mediawiki/api.php".to_string(),
                bot: true
            }
        );
        let config = Config::load().unwrap();
        assert_eq!(
            config,
            Config {
                token: env::var("DISCORD_TOKEN").unwrap_or("TO BE FILLED".to_string()),
                passwd: env::var("PASSWD").unwrap_or("TO BE FILLED".to_string()),
                login: env::var("LOGIN").unwrap_or("TO BE FILLED".to_string()),
                endpoint: "https://qwrky.dev/mediawiki/api.php".to_string(),
                bot: true
            }
        );
        fs::remove_file("/root/.config/sd-archivemanager/config.toml").unwrap();
    }
}

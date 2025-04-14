use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use xdg::BaseDirectories;

use crate::error::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub author: String,
    pub regex: String,
    pub for_title: bool,
    pub name: String,
    pub replace: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegexManager {
    pub profile: Option<Vec<Profile>>,
}
impl RegexManager {
    pub fn save_regex(
        &mut self,
        exp: String,
        user: &str,
        name: &str,
        for_title: bool,
        target: &str,
        replace: String,
    ) -> Result<(), Error> {
        if let Some(ref mut profiles_vec) = self.profile {
            profiles_vec.push(Profile {
                author: user.to_string(),
                regex: exp,
                for_title,
                name: name.to_string(),
                replace,
                target: target.to_string(),
            });
        } else {
            self.profile = Some(vec![Profile {
                author: user.to_string(),
                regex: exp,
                for_title,
                name: name.to_string(),
                replace,
                target: target.to_string(),
            }]);
        }
        Ok(())
    }
    pub fn save(&self) -> Result<(), Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg.place_data_file("regex.toml").context(IoSnafu {
            file: xdg
                .get_data_home()
                .join("regex.toml")
                .to_str()
                .unwrap()
                .to_string(),
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        if self.profile.is_some() {
            file.write_all(toml::to_string(self).unwrap().as_bytes())
                .context(IoSnafu {
                    file: path.clone().to_string_lossy().to_string(),
                })?;
        } else {
            file.seek(SeekFrom::Start(1)).unwrap();
            file.write_all(&[0]).unwrap();
        }
        Ok(())
    }
    pub fn load() -> Result<Self, Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg
            .find_data_file("regex.toml")
            .whatever_context("unable to find file")?;
        let profiles = toml::from_str::<RegexManager>(
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
    pub fn get_regexs(&self, user: &str) -> Vec<Profile> {
        match &self.profile {
            None => vec![],
            Some(p) => {
                let mut vec = p
                    .iter()
                    .filter(|x| {
                        x.author
                            .split(',')
                            .map(|y| y.trim())
                            .collect::<Vec<_>>()
                            .contains(&user)
                            || x.author == "*"
                    })
                    .cloned()
                    .collect::<Vec<Profile>>();
                vec.sort_by(|x, y| x.name.cmp(&y.name));
                vec
            }
        }
    }
    pub fn new() -> Self {
        RegexManager { profile: None }
    }
}

impl Default for RegexManager {
    fn default() -> Self {
        let xdg = BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let path = xdg
            .place_data_file("regex.toml")
            .context(IoSnafu {
                file: xdg
                    .get_data_home()
                    .join("regex.toml")
                    .to_str()
                    .unwrap()
                    .to_string(),
            })
            .unwrap();
        let rgx = RegexManager { profile: None };
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        file.write_all(toml::to_string(&rgx).unwrap().as_bytes())
            .unwrap();
        rgx
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use super::*;

    #[test]
    fn test_save_regex() {
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
        let mut rgx = RegexManager::default();
        rgx.save_regex(
            "xyz".to_string(),
            "testuser",
            "Testing",
            false, "eo",
            "abc".to_string(),
        )
        .unwrap();
        rgx.save().unwrap();
        let content =
            fs::read_to_string("/root/.local/share/sd-archivemanager/regex.toml").unwrap();
        assert_eq!(
            content,
            "[[profile]]\nauthor = \"testuser\"\nregex = \"xyz\"\nfor_title = false\nname = \"Testing\"\nreplace = \"abc\"\ntarget = \"eo\"\n"
        );
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
    #[test]
    fn save_rgx_file() {
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
        let mut rgx = RegexManager::default();
        rgx.save_regex(
            r#"\s"#.to_string(),
            "f3rri5_",
            "001-Replace spaces",
            true, "eo",
            r#":"#.to_string(),
        )
        .unwrap();
        rgx.save().unwrap();
        assert!(
            PathBuf::from_str("/root/.local/share/sd-archivemanager/regex.toml")
                .unwrap()
                .is_file()
        );
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
}

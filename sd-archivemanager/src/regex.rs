use std::io::Write;

use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use tokio::task;
use xdg::BaseDirectories;

use crate::error::*;

/// This struct is used to store the regex profiles
/// Is used as a generic struct to represent regex application for EOs, legislation and case law
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub author: String,
    pub regex: String,
    pub for_title: bool,
    pub name: String,
    pub replace: String,
    pub target: Target,
}

/// Representation of the regex.toml file which contains regex profiles
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegexManager {
    pub profile: Option<Vec<Profile>>,
}

impl RegexManager {
    /// Save regex profile to manager. NOTE: It does not save to file.
    /// ```no_run
    /// let mut rgx = RegexManager::default();
    /// rgx.save_regex(
    ///        "xyz".to_string(),
    ///        "testuser",
    ///        "Testing",
    ///         false, "eo",
    ///         "abc".to_string(),
    ///         ).unwrap();
    /// ```
    pub fn save_regex(
        &mut self,
        exp: String,
        user: &str,
        name: &str,
        for_title: bool,
        target: Target,
        replace: String,
    ) -> Result<(), Error> {
        if let Some(ref mut profiles_vec) = self.profile {
            profiles_vec.push(Profile {
                author: user.to_string(),
                regex: exp,
                for_title,
                name: name.to_string(),
                replace,
                target,
            });
        } else {
            self.profile = Some(vec![Profile {
                author: user.to_string(),
                regex: exp,
                for_title,
                name: name.to_string(),
                replace,
                target,
            }]);
        }
        Ok(())
    }
    /// Saves current regex profiles to regex.toml
    pub async fn save(&self) -> Result<(), Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
        let path = xdg.place_data_file("regex.toml").context(IoSnafu {
            file: xdg
                .get_data_home()
                .join("regex.toml")
                .to_str()
                .unwrap()
                .to_string(),
        })?.as_path().to_owned();
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
        if self.profile.is_some() {
            file.await.context(TokioSnafu)?.write_all(toml::to_string(self).unwrap().as_bytes()).await
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
        let profiles = toml::from_str::<RegexManager>(
            fs::read_to_string(&path).await
                .map_err(|e: std::io::Error| Error::IoError {
                    source: e,
                    file: path,
                })?
                .as_str(),
        )
        .context(InvalidConfigSnafu)?;
        Ok(profiles)
    }
    /// Return all regex expressions valid for a given user. Accepts users via a csv or the glob oberator (*)
    /// for examples,
    /// ```no_run
    /// config = r#"[profile]
    /// author = "*"
    /// ..."#
    /// let regex = RegexManager::default();
    /// assert_eq!(vec![Profile {author = "*", ...}], regex.get_regexs())
    /// ```
    pub fn get_regexs(&self, user: &str, target: Target) -> Vec<&Profile> {
        match &self.profile {
            None => vec![],
            Some(p) => {
                let mut vec = p
                    .iter()
                    .filter(|x| {
                        (x.author
                            .split(',')
                            .map(|y| y.trim())
                            .collect::<Vec<_>>()
                            .contains(&user)
                            || x.author == "*") && target == x.target
                    })
                    .collect::<Vec<&Profile>>();
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
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        file.write_all(toml::to_string(&rgx).unwrap().as_bytes())
            .unwrap();
        rgx
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Legislation,
    EO,
    CaseLaw,
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use super::*;

    #[tokio::test]
    async fn test_save_regex() {
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
        let mut rgx = RegexManager::default();
        rgx.save_regex(
            "xyz".to_string(),
            "testuser",
            "Testing",
            false,
            Target::EO,
            "abc".to_string(),
        )
        .unwrap();
        rgx.save().await.unwrap();
        let content =
            fs::read_to_string("/root/.local/share/sd-archivemanager/regex.toml").await.unwrap();
        assert_eq!(
            content,
            "[[profile]]\nauthor = \"testuser\"\nregex = \"xyz\"\nfor_title = false\nname = \"Testing\"\nreplace = \"abc\"\ntarget = \"eo\"\n"
        );
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
    #[tokio::test]
    async fn save_rgx_file() {
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
        let mut rgx = RegexManager::default();
        rgx.save_regex(
            r#"\s"#.to_string(),
            "f3rri5_",
            "001-Replace spaces",
            true,
            Target::EO,
            r#":"#.to_string(),
        )
        .unwrap();
        rgx.save().await.unwrap();
        assert!(
            PathBuf::from_str("/root/.local/share/sd-archivemanager/regex.toml")
                .unwrap()
                .is_file()
        );
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
}

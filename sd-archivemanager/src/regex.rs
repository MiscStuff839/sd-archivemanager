use std::fs::{self, File};

use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Profile {
    pub(crate) name: String,
    pub(crate) regex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RegexManager {
    pub(crate) profile: Option<Vec<Profile>>,
}

pub fn save_regex(exp: String, user: &str) -> Result<(), Error> {
    let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
    let _ = Regex::new(&exp).context(InvalidRegexSnafu)?;
    let path = xdg.place_data_file("regex.toml").context(IoSnafu {
        file: xdg
            .get_data_home()
            .join("regex.toml")
            .to_str()
            .unwrap()
            .to_string(),
    })?;
    let file = path.clone();
    if !file.is_file() {
        File::create_new(&file).unwrap();
    }
    let mut profiles: RegexManager = toml::from_str(
        fs::read_to_string(path.clone())
            .context(IoSnafu {
                file: file.clone().to_string_lossy().to_string(),
            })?
            .as_str(),
    )
    .context(InvalidConfigSnafu)?;
    if let Some(ref mut profiles_vec) = profiles.profile {
        profiles_vec.push(Profile {
            name: user.to_string(),
            regex: exp,
        });
    } else {
        profiles.profile = Some(vec![Profile {
            name: user.to_string(),
            regex: exp,
        }]);
    }
    fs::write(path.clone(), toml::to_string(&profiles).unwrap()).context(IoSnafu {
        file: path.clone().to_string_lossy().to_string(),
    })?;
    Ok(())
}

pub fn get_regex(user: &str) -> Result<Vec<String>, Error> {
    let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
    let path = xdg
        .place_data_file("regex.toml")
        .map_err(|e| Error::IoError {
            source: e,
            file: xdg
                .get_data_home()
                .join("regex.toml")
                .to_string_lossy()
                .to_string(),
        })?;
    let profiles = toml::from_str::<Vec<Profile>>(
        fs::read_to_string(path.clone())
            .map_err(|e: std::io::Error| Error::IoError {
                source: e,
                file: path.to_string_lossy().to_string(),
            })?
            .as_str(),
    ).context(InvalidConfigSnafu)?;
    Ok(profiles
        .iter()
        .cloned()
        .filter_map(|p| if p.name == user { Some(p.regex) } else { None })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_regex() {
        save_regex("xyz".to_string(), "testuser").unwrap();
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let path = xdg.get_data_file("regex.toml");
        let content = fs::read_to_string(&path).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(content, "[[profile]]\nname = \"testuser\"\nregex = \"xyz\"\n");
    }
}

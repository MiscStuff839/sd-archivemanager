use std::{fs::OpenOptions, io::Write};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use crate::error::{Error, FileNotFoundSnafu, IoSnafu};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Guilds {
    guilds: Vec<GuildInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GuildInfo {
    pub id: u64,
    pub name: String,
    pub eo_src: u64,
    pub leg_src: u64,
}

impl Guilds {
    pub fn get_guilds(&self) -> &Vec<GuildInfo> {
        &self.guilds
    }
    pub fn save(&self) -> Result<(), Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let guilds_file = xdg.place_data_file("guilds.toml").context(IoSnafu {
            file: xdg
                .get_data_home()
                .join("guilds.toml")
                .to_string_lossy()
                .to_string(),
        })?;
        let mut guilds_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(guilds_file)
            .context(IoSnafu {
                file: xdg
                    .get_data_home()
                    .join("guilds.toml")
                    .to_string_lossy()
                    .to_string(),
            })?;
        let content = toml::to_string(self).unwrap();
        guilds_file.write_all(content.as_bytes()).context(IoSnafu {
            file: xdg
                .get_data_home()
                .join("guilds.toml")
                .to_string_lossy()
                .to_string(),
        })?;
        Ok(())
    }
    pub fn load() -> Result<Self, Error> {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let guilds_file = xdg
            .find_data_file("guilds.toml")
            .context(FileNotFoundSnafu {
                file: xdg.get_data_home().join("guilds.toml"),
            })?;
        let content = std::fs::read_to_string(&guilds_file).context(IoSnafu {
            file: xdg.get_data_home().join("guilds.toml"),
        })?;
        let guilds: Guilds = toml::from_str(&content).unwrap();
        Ok(guilds)
    }
}

impl Default for Guilds {
    fn default() -> Self {
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let guilds_file = xdg.place_data_file("guilds.toml").expect("no permissions");
        let mut guilds_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(guilds_file)
            .expect("no permissions");
        let guilds = Guilds {
            guilds: vec![
                GuildInfo {
                    id: 554769523635650580,
                    name: "SimDemocracy".to_string(),
                    eo_src: 1232486399295619103,
                    leg_src: 1346304876581228554,
                },
                GuildInfo {
                    id: 1342703437371932712,
                    name: "National Archives".to_string(),
                    eo_src: 1348711268877467738,
                    leg_src: 1348710825619230750,
                },
            ],
        };
        let content = toml::to_string(&guilds).unwrap();
        guilds_file
            .write_all(content.as_bytes())
            .expect("no permissions");
        guilds
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn guilds_default() {
        let guilds = Guilds::default();
        assert_eq!(guilds.guilds.len(), 2);
        assert_eq!(guilds.guilds[0].id, 554769523635650580);
        assert_eq!(guilds.guilds[0].name, "SimDemocracy");
        assert_eq!(guilds.guilds[0].eo_src, 1232486399295619103);
        assert_eq!(guilds.guilds[0].leg_src, 1346304876581228554);
        assert_eq!(guilds.guilds[1].id, 1342703437371932712);
        assert_eq!(guilds.guilds[1].name, "National Archives");
        assert_eq!(guilds.guilds[1].eo_src, 1348711268877467738);
        assert_eq!(guilds.guilds[1].leg_src, 1348710825619230750);
        let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let path = xdg.find_data_file("guilds.toml").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            r#"[[guilds]]
id = 554769523635650580
name = "SimDemocracy"
eo_src = 1232486399295619103
leg_src = 1346304876581228554

[[guilds]]
id = 1342703437371932712
name = "National Archives"
eo_src = 1348711268877467738
leg_src = 1348710825619230750
"#,
            content
        );
        fs::remove_file(path).unwrap();
    }
}

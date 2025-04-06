use std::sync::MutexGuard;

use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use crate::{
    CONFIG,
    config::Config,
    error::{Error, ReqwestSnafu},
    format_auth,
    guilds::GuildInfo,
    regex::RegexManager,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EOInfo {
    pub name: String,
    pub author: String,
    pub content: String,
    pub date: DateTime<Utc>,
}

pub fn list_eos(guild: &GuildInfo) -> Result<Vec<EOInfo>, Error> {
    let client = Client::new();
    let cfg = CONFIG.lock().unwrap();
    let req = client
        .get(format!(
            "https://discord.com/api/v10/guilds/{}/threads/active",
            guild.id
        ))
        .header("Authorization", format_auth(&cfg));
    let mut threads = Vec::new();
    while {
        let json = req
            .try_clone()
            .unwrap()
            .send()
            .unwrap()
            .json::<Value>()
            .unwrap();
        for thread in json["threads"].as_array().unwrap() {
            if thread["parent_id"]
                .to_string()
                .trim_matches('"')
                .parse::<u64>()
                .whatever_context("invalid response from discord")?
                == guild.eo_src
            {
                threads.push(scrape_eo(
                    thread["id"]
                        .to_string()
                        .trim_matches('"')
                        .parse::<u64>()
                        .whatever_context("Invalid response from discord")?,
                    Some(&cfg),
                )?);
            }
        }
        json["has_more"].as_bool().unwrap()
    } {}
    Ok(threads)
}

fn format(eo: &EOInfo) -> EOInfo {
    let rgx = RegexManager::load().unwrap_or_else(|e| {
        eprintln!("{}", e);
        RegexManager::default()
    });
    let patterns = rgx.get_regexs(&eo.author).unwrap_or(vec![]);
    let mut eo = eo.to_owned();
    for pat in patterns {
        let reg = Regex::new(&pat.regex).unwrap();
        if pat.for_title {
            eo.name = reg.replace_all(&eo.name, pat.replace).to_string();
        } else {
            eo.content = reg.replace_all(&eo.content, pat.replace).to_string();
        }
    }
    eo
}

pub fn scrape_eo(id: u64, conf: Option<&MutexGuard<'_, Config>>) -> Result<EOInfo, Error> {
    let cfg;
    let lock;
    if conf.is_none() {
        lock = CONFIG.lock().unwrap();
        cfg = &lock;
    } else {
        cfg = conf.unwrap();
    }
    let client = Client::new();
    let resp = client
        .get(format!(
            "https://discord.com/api/v10/channels/{id}/messages/{id}"
        ))
        .header("Authorization", format_auth(&cfg))
        .send()
        .context(ReqwestSnafu)?;
    let json_message = resp.json::<serde_json::Value>().unwrap();
    let resp = Client::new()
        .get(format!("https://discord.com/api/v10/channels/{id}"))
        .header("Authorization", format_auth(&cfg))
        .send()
        .context(ReqwestSnafu)?;
    let json_channel = resp.json::<serde_json::Value>().unwrap();
    Ok(EOInfo {
        author: json_message["author"]["username"]
            .as_str()
            .unwrap()
            .to_string(),
        name: json_channel["name"].as_str().unwrap().to_string(),
        content: json_message["content"].as_str().unwrap().to_string(),
        date: json_message["timestamp"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::guilds::{self, Guilds};

    use super::*;

    #[test]
    fn test_scrape_eo() {
        let eo = scrape_eo(1348749282349416529, None).unwrap();
        assert_eq!(
            eo,
            EOInfo {
                author: "f3rri5_".to_string(),
                name: "EO 1239809 | Archive Supremacy".to_string(),
                content: "Lorum Ipsum".to_string(),
                date: "2025-03-10T20:08:07.462000+00:00"
                    .parse::<DateTime<Utc>>()
                    .unwrap(),
            }
        );
    }
    #[test]
    fn format_eo() {
        // setup regex
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
        let mut rgx = RegexManager::default();
        rgx.save_regex(
            r#"\s"#.to_string(),
            "f3rri5_",
            "001-Replace spaces",
            true,
            r#":"#.to_string(),
        )
        .unwrap();
        rgx.save().unwrap();
        let eo_info = scrape_eo(1348749282349416529, None).unwrap();
        let eo = format(&eo_info);
        assert_eq!(eo.name, "EO:1239809:|:Archive:Supremacy");
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
    #[test]
    fn list_eos_test() {
        let guilds = Guilds::default();
        let eos = list_eos(guilds.get_guilds().get(1).unwrap());
        let expected = vec![scrape_eo(1348749282349416529, None).unwrap()];
        assert_eq!(eos.unwrap(), expected);
    }
}

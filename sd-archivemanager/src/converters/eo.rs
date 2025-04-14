use std::sync::{Arc, MutexGuard};

use chrono::{DateTime, Utc};
use extism::{Manifest, Plugin, Wasm, convert::Json};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};
use tinytemplate::TinyTemplate;

use crate::{
    CONFIG,
    config::Config,
    error::{Error, ExtismSnafu, ReqwestSnafu},
    format_auth,
    guilds::GuildInfo,
    plugins::PluginManager,
    regex::RegexManager,
};

use super::{get_cookies, get_token, login, upload};

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
                .as_str()
                .unwrap()
                .parse::<u64>()
                .whatever_context("invalid response from discord")?
                == guild.eo_src
            {
                threads.push(scrape_eo(
                    thread["id"]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .whatever_context("Invalid response from discord")?,
                    Some(&cfg),
                )?);
            }
        }
        json["has_more"].as_bool().unwrap()
    } {}
    let req = client
        .get(format!(
            "https://discord.com/api/v10/channels/{}/threads/archived/public",
            guild.eo_src
        ))
        .header("Authorization", format_auth(&cfg));
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
                .as_str()
                .unwrap()
                .parse::<u64>()
                .whatever_context("invalid response from discord")?
                == guild.eo_src
            {
                threads.push(scrape_eo(
                    thread["id"]
                        .as_str()
                        .unwrap()
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

fn format_rgx(eo: &EOInfo) -> EOInfo {
    let rgx = RegexManager::load().unwrap_or_else(|e| {
        eprintln!("{}", e);
        RegexManager::default()
    });
    let patterns = rgx.get_regexs(&eo.author);
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

pub fn format_pre_plugins(eo: &EOInfo) -> Result<EOInfo, Error> {
    let manager = PluginManager::load().unwrap_or_default();
    let plugins = manager.get_plugins(&eo.author);
    let mut iter = plugins
        .iter()
        .filter(|p| p.pre && p.target == "eo")
        .peekable();
    let mut result = Json::from(eo.clone());
    if iter.peek().is_some() {
        while {
            let manifest = Manifest::new([Wasm::file(&iter.next().unwrap().path)]);
            let mut plugin = Plugin::new(&manifest, [], true).context(ExtismSnafu)?;
            result = plugin
                .call::<_, Json<EOInfo>>("pre", serde_json::to_vec(eo).unwrap())
                .context(ExtismSnafu)?;
            iter.peek().is_some()
        } {}
    }
    Ok(result.into_inner())
}

pub fn format_post_plugins(eo: &EOInfo) -> Result<EOInfo, Error> {
    let manager = PluginManager::load().unwrap_or_default();
    let plugins = manager.get_plugins(&eo.author);
    let mut iter = plugins
        .iter()
        .filter(|p| p.post && p.target == "eo")
        .peekable();
    let mut result = Json::from(eo.clone());
    if iter.peek().is_some() {
        while {
            let manifest = Manifest::new([Wasm::file(&iter.next().unwrap().path)]);
            let mut plugin = Plugin::new(&manifest, [], true).context(ExtismSnafu)?;
            result = plugin
                .call::<_, Json<EOInfo>>("post", serde_json::to_vec(eo).unwrap())
                .context(ExtismSnafu)?;
            iter.peek().is_some()
        } {}
    }
    Ok(result.into_inner())
}

#[inline]
pub fn format_eo(eo: &EOInfo) -> Result<EOInfo, Error> {
    Ok(format_post_plugins(&format_rgx(&format_pre_plugins(&eo)?))?)
}

pub fn handle_eo(eo_link: String, template: &str) -> Result<(), Error> {
    let id = eo_link
        .split('/')
        .nth(5)
        .whatever_context("invalid url")?
        .trim()
        .parse::<u64>()
        .whatever_context("invalid url")?;
    handle_eo_id(template, id)?;
    Ok(())
}

pub fn handle_eo_id(template: &str, eo_id: u64) -> Result<(), Error> {
    let eo = format_eo(&scrape_eo(eo_id, None)?)?;
    let mut templater = TinyTemplate::new();
    templater
        .add_template("template", &template)
        .whatever_context("invalid template")?;
    let cfg = CONFIG.lock().unwrap();
    let (mut file, cookie_store) = get_cookies()?;
    let cookies = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));
    login(&cfg)?;
    upload(
        &eo.name,
        get_token(&cfg, Some(&cookies))?,
        &cfg,
        templater
            .render("template", &eo)
            .whatever_context("failed to render")?,
    )?;
    cookies
        .lock()
        .unwrap()
        .save(&mut file, |s| serde_json::to_string(s))
        .expect("failed to save cookies");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::guilds::Guilds;

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
            "eo",
            r#":"#.to_string(),
        )
        .unwrap();
        rgx.save().unwrap();
        let eo_info = scrape_eo(1348749282349416529, None).unwrap();
        let eo = format_rgx(&eo_info);
        assert_eq!(eo.name, "EO:1239809:|:Archive:Supremacy");
        let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
    }
    #[test]
    fn list_eos_test() {
        let guilds = Guilds::default();
        let eos = list_eos(guilds.get_guilds().get(1).unwrap());
        let expected = vec![
            scrape_eo(1360569363140448326, None).unwrap(),
            scrape_eo(1348749282349416529, None).unwrap(),
        ];
        assert_eq!(eos.unwrap(), expected);
    }
    #[test]
    fn format_pre() {
        let eo = scrape_eo(1348749282349416529, None).unwrap();
        let eo = format_pre_plugins(&eo);
    }
}

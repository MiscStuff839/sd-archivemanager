use std::sync::{Arc, MutexGuard};

use chrono::{DateTime, Utc};
use extism::{convert::Json, Manifest, Plugin, Wasm};
use regex::Regex;
use reqwest::{blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};
use tinytemplate::TinyTemplate;
use url::Url;

use crate::{
    config::Config, error::{Error, ExtismSnafu, InvalidRegexSnafu, ReqwestSnafu}, format_auth, guilds::GuildInfo, plugins::PluginManager, regex::RegexManager, CONFIG
};

use super::{get_cookies, get_token, login, upload};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegInfo {
    pub author: String,
    pub date: DateTime<Utc>,
    pub name: String,
    pub content: String,
}

pub fn scrape_law(
    id: u64,
    conf: Option<&MutexGuard<Config>>,
    guild: &GuildInfo,
) -> Result<LegInfo, Error> {
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .build()
        .context(ReqwestSnafu)?;
    let cfg;
    let lock;
    if conf.is_none() {
        lock = CONFIG.lock().unwrap();
        cfg = &lock;
    } else {
        cfg = conf.unwrap();
    }
    let message = client
        .get(format!(
            "https://discord.com/api/v10/channels/{}/messages/{}",
            guild.leg_src, id
        ))
        .header("Authorization", format_auth(cfg))
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?;
    let url = Url::parse(
        message["embeds"]
            .as_array()
            .whatever_context("invalid response")?[0]["url"]
            .as_str()
            .whatever_context("invalid response")?,
    )
    .unwrap();
    let id = url
        .path_segments()
        .unwrap()
        .nth(url.path_segments().unwrap().position(|x| x == "d").unwrap() + 1)
        .unwrap();
    let content = client
        .get(format!(
            "https://docs.google.com/document/d/{}/export?format=md",
            id
        ))
        .send()
        .context(ReqwestSnafu)?
        .text()
        .context(ReqwestSnafu)?;
    let name = {
        let resp = client
            .get(format!("https://docs.google.com/document/d/{}/view", id))
            .send()
            .context(ReqwestSnafu)?
            .text()
            .context(ReqwestSnafu)?;
        let re = Regex::new(r"<title>(.*?)</title>").context(InvalidRegexSnafu)?;
        if let Some(captures) = re.captures(&resp) {
            let raw_title = captures.get(1).map_or("", |m| m.as_str());
            let cleaned_title = raw_title
                .strip_suffix(" - Google Docs")
                .unwrap_or(raw_title);
            cleaned_title.to_string()
        } else {
            "".to_string()
        }
    };
    Ok(LegInfo {
        author: message["author"]["username"]
            .as_str()
            .whatever_context("invalid response")?
            .to_string(),
        date: message["timestamp"]
            .as_str()
            .whatever_context("invalid response")?
            .parse::<DateTime<Utc>>()
            .unwrap(),
        name,
        content,
    })
}

fn format_rgx(law: &LegInfo) -> LegInfo {
    let rgx = RegexManager::load().unwrap_or_else(|e| {
        eprintln!("{}", e);
        RegexManager::default()
    });
    let patterns = rgx.get_regexs(&law.author);
    let mut law = law.to_owned();
    for pat in patterns {
        let reg = Regex::new(&pat.regex).unwrap();
        if pat.for_title {
            law.name = reg.replace_all(&law.name, pat.replace).to_string();
        } else {
            law.content = reg.replace_all(&law.content, pat.replace).to_string();
        }
    }
    law
}

pub fn format_pre_plugins(law: &LegInfo) -> Result<LegInfo, Error> {
    let manager = PluginManager::load().unwrap_or_default();
    let plugins = manager.get_plugins(&law.author);
    let mut iter = plugins
        .iter()
        .filter(|p| p.pre && p.target == "legislation")
        .peekable();
    let mut result = Json::from(law.clone());
    if iter.peek().is_some() {
        while {
            let manifest = Manifest::new([Wasm::file(&iter.next().unwrap().path)]);
            let mut plugin = Plugin::new(&manifest, [], true).context(ExtismSnafu)?;
            result = plugin
                .call::<_, Json<LegInfo>>("pre", serde_json::to_vec(law).unwrap())
                .context(ExtismSnafu)?;
            iter.peek().is_some()
        } {}
    }
    Ok(result.into_inner())
}

pub fn format_post_plugins(law: &LegInfo) -> Result<LegInfo, Error> {
    let manager = PluginManager::load().unwrap_or_default();
    let plugins = manager.get_plugins(&law.author);
    let mut iter = plugins
        .iter()
        .filter(|p| p.post && p.target == "legislation")
        .peekable();
    let mut result = Json::from(law.clone());
    if iter.peek().is_some() {
        while {
            let manifest = Manifest::new([Wasm::file(&iter.next().unwrap().path)]);
            let mut plugin = Plugin::new(&manifest, [], true).context(ExtismSnafu)?;
            result = plugin
                .call::<_, Json<LegInfo>>("post", serde_json::to_vec(law).unwrap())
                .context(ExtismSnafu)?;
            iter.peek().is_some()
        } {}
    }
    Ok(result.into_inner())
}

#[inline]
pub fn format_law(law: &LegInfo) -> Result<LegInfo, Error> {
    Ok(format_post_plugins(&format_rgx(&format_pre_plugins(&law)?))?)
}

pub fn handle_law(law_link: String, template: &str, guild: &GuildInfo) -> Result<(), Error> {
    let id = law_link
        .split('/')
        .nth(5)
        .whatever_context("invalid url")?
        .trim()
        .parse::<u64>()
        .whatever_context("invalid url")?;
    handle_law_id(template, id, guild)?;
    Ok(())
}

pub fn handle_law_id(template: &str, law_id: u64, guild: &GuildInfo) -> Result<(), Error> {
    let law = format_law(&scrape_law(law_id, None, guild)?)?;
    let mut templater = TinyTemplate::new();
    templater
        .add_template("template", &template)
        .whatever_context("invalid template")?;
    let cfg = CONFIG.lock().unwrap();
    let (mut file, cookie_store) = get_cookies()?;
    let cookies = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));
    login(&cfg)?;
    upload(
        &law.name,
        get_token(&cfg, Some(&cookies))?,
        &cfg,
        templater
            .render("template", &law)
            .whatever_context("failed to render")?,
    )?;
    cookies
        .lock()
        .unwrap()
        .save(&mut file, |s| serde_json::to_string(s))
        .expect("failed to save cookies");
    Ok(())
}
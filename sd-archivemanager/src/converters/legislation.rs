use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use reqwest::{Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};
use std::sync::Arc;
use tinytemplate::TinyTemplate;
use tokio::{join, sync::MutexGuard, task, try_join};
use url::Url;

use crate::{
    CONFIG,
    config::Config,
    error::{Error, InvalidRegexSnafu, ReqwestSnafu, TokioSnafu, XdgSnafu},
    format_auth,
    guilds::GuildInfo,
};

use super::{PageData, get_cookies, get_token, upload};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LawInfo {
    pub author: String,
    pub date: NaiveDate,
    pub name: String,
    pub content: String,
    pub link: String,
}

impl<'a> PageData<'a> for LawInfo {
    async fn scrape(
        id: u64,
        conf: Option<&MutexGuard<'_, Config>>,
        client: Arc<Client>,
        guild: Option<&GuildInfo>,
    ) -> Result<Self, Error> {
        let cfg;
        let lock;
        if conf.is_none() {
            lock = CONFIG.lock().await;
            cfg = &lock;
        } else {
            cfg = conf.unwrap();
        }
        let message = client
            .get(format!(
                "https://discord.com/api/v10/channels/{}/messages/{}",
                guild.unwrap().leg_src,
                id
            ))
            .header("Authorization", format_auth(cfg))
            .send()
            .await
            .context(ReqwestSnafu)?
            .json::<Value>()
            .await
            .context(ReqwestSnafu)?;
        let rgx = Regex::new(r#"(https:\/\/[^])\s]+)"#).context(InvalidRegexSnafu)?;
        let url = Url::parse(
            rgx.captures(message["content"].as_str().unwrap())
                .unwrap()
                .extract::<1>()
                .1[0],
        )
        .unwrap();
        let id = url
            .path_segments()
            .unwrap()
            .nth(url.path_segments().unwrap().position(|x| x == "d").unwrap() + 1)
            .unwrap();
        let (content, title) = try_join! {client
        .get(format!(
            "https://docs.google.com/document/d/{}/export?format=md",
            id
        ))
        .send(), client
            .get(format!("https://docs.google.com/document/d/{}/view", id))
            .send()}
        .context(ReqwestSnafu)?;
        let (content, title) = try_join!(content.text(), title.text()).context(ReqwestSnafu)?;
        let name = {
            let re = Regex::new(r"<title>(.*?)</title>").context(InvalidRegexSnafu)?;
            if let Some(captures) = re.captures(&title) {
                let raw_title = captures.get(1).map_or("", |m| m.as_str());
                let cleaned_title = raw_title
                    .strip_suffix(" - Google Docs")
                    .unwrap_or(raw_title);
                cleaned_title.to_string()
            } else {
                "".to_string()
            }
        };
        Ok(LawInfo {
            author: message["author"]["username"]
                .as_str()
                .whatever_context("invalid response")?
                .to_string(),
            date: message["timestamp"]
                .as_str()
                .whatever_context("invalid response")?
                .parse::<DateTime<Utc>>()
                .unwrap()
                .date_naive(),
            name,
            content,
            link: url.to_string(),
        })
    }

    fn get_content(&self) -> &str {
        &self.content
    }

    fn get_title(&self) -> &str {
        &self.name
    }

    fn get_date(&self) -> &NaiveDate {
        &self.date
    }

    fn get_author(&self) -> &str {
        &self.author
    }

    fn get_mut_content(&mut self) -> &mut String {
        &mut self.content
    }

    fn get_mut_title(&mut self) -> &mut String {
        &mut self.name
    }

    fn get_mut_date(&mut self) -> &mut NaiveDate {
        &mut self.date
    }

    fn get_mut_author(&mut self) -> &mut String {
        &mut self.author
    }
}

pub async fn handle_law(law_link: String, template: &str, guild: &GuildInfo) -> Result<(), Error> {
    let id = law_link
        .split('/')
        .nth(5)
        .whatever_context("invalid url")?
        .trim()
        .parse::<u64>()
        .whatever_context("invalid url")?;
    handle_law_id(template, id, guild).await?;
    Ok(())
}

pub async fn handle_law_id(template: &str, law_id: u64, guild: &GuildInfo) -> Result<(), Error> {
    let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
    let (file, cookies) = get_cookies(&xdg).await?;
    let client = Arc::new(
        Client::builder()
            .cookie_provider(Arc::clone(&cookies))
            .build()
            .unwrap(),
    );
    let law_client = Arc::clone(&client);
    let mut law = LawInfo::scrape(law_id, None, law_client, Some(guild))
        .await?;
    let mut templater: TinyTemplate<'_> = TinyTemplate::new();
    templater
        .add_template("template", &template)
        .whatever_context("invalid template")?;
    let cfg = CONFIG.lock().await;
    law.format("legislation").await?;
    upload(
        &law.name,
        &client,
        &get_token(&cfg, &client, &xdg).await?,
        &cfg,
        &templater
            .render("template", &law)
            .whatever_context("failed to render")?,
    ).await?;
    cookies
        .lock()
        .unwrap()
        .save(&mut file.into_std().await, |s| serde_json::to_string(s))
        .expect("failed to save cookies");
    Ok(())
}
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use std::sync::Arc;
use tinytemplate::TinyTemplate;
use tokio::{
    sync::MutexGuard,
    task::{self},
    try_join,
};
use xdg::BaseDirectories;

use crate::{
    config::Config, error::{Error, ReqwestSnafu, TokioSnafu, XdgSnafu}, format_auth, guilds::GuildInfo, regex::Target, CONFIG
};

use super::{PageData, get_cookies, get_token, upload};

/// Representation of an Executive Order.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EOInfo {
    pub name: String,
    pub author: String,
    pub content: String,
    pub date: NaiveDate,
}

/// List all EO threads in a guild.
// pub fn list_eos(guild: &GuildInfo) -> Result<Vec<EOInfo>, Error> {
//     let client = Client::new();
//     let cfg = CONFIG.lock().unwrap();
//     let req = client
//         .get(format!(
//             "https://discord.com/api/v10/guilds/{}/threads/active",
//             guild.id
//         ))
//         .header("Authorization", format_auth(&cfg));
//     let mut threads = Vec::new();
//     while {
//         let json = req
//             .try_clone()
//             .unwrap()
//             .send()
//             .unwrap()
//             .json::<Value>()
//             .unwrap();
//         for thread in json["threads"].as_array().unwrap() {
//             if thread["parent_id"]
//                 .as_str()
//                 .unwrap()
//                 .parse::<u64>()
//                 .whatever_context("invalid response from discord")?
//                 == guild.eo_src
//             {
//                 threads.push(scrape_eo(
//                     thread["id"]
//                         .as_str()
//                         .unwrap()
//                         .parse::<u64>()
//                         .whatever_context("Invalid response from discord")?,
//                     Some(&cfg),
//                     &client,
//                 )?);
//             }
//         }
//         json["has_more"].as_bool().unwrap()
//     } {}
//     let req = client
//         .get(format!(
//             "https://discord.com/api/v10/channels/{}/threads/archived/public",
//             guild.eo_src
//         ))
//         .header("Authorization", format_auth(&cfg));
//     while {
//         let json = req
//             .try_clone()
//             .unwrap()
//             .send()
//             .unwrap()
//             .json::<Value>()
//             .unwrap();
//         for thread in json["threads"].as_array().unwrap() {
//             if thread["parent_id"]
//                 .as_str()
//                 .unwrap()
//                 .parse::<u64>()
//                 .whatever_context("invalid response from discord")?
//                 == guild.eo_src
//             {
//                 threads.push(scrape_eo(
//                     thread["id"]
//                         .as_str()
//                         .unwrap()
//                         .parse::<u64>()
//                         .whatever_context("Invalid response from discord")?,
//                     Some(&cfg),
//                     &client,
//                 )?);
//             }
//         }
//         json["has_more"].as_bool().unwrap()
//     } {}
//     Ok(threads)
// }
impl<'a> PageData<'a> for EOInfo {
    /// Create an EO object by scraping a message
    async fn scrape(
        id: u64,
        conf: Option<&MutexGuard<'_, Config>>,
        client: Arc<Client>,
        _: Option<&GuildInfo>,
    ) -> Result<EOInfo, Error> {
        let cfg;
        let lock;
        if conf.is_none() {
            lock = CONFIG.lock().await;
            cfg = &lock;
        } else {
            cfg = conf.unwrap();
        }
        let auth = format_auth(&cfg);
        #[derive(Debug, Deserialize)]
        struct MsgInfo {
            author: String,
            content: String,
            timestamp: String,
        }
        #[derive(Debug, Deserialize)]
        struct ChannelInfo {
            name: String,
        }
        let (message, channel) = try_join! {
            client
            .get(format!(
                "https://discord.com/api/v10/channels/{id}/messages/{id}"
            ))
            .header("Authorization", &auth)
            .send(), client
            .get(format!("https://discord.com/api/v10/channels/{id}"))
            .header("Authorization", &auth)
            .send()
        }
        .context(ReqwestSnafu)?;
        let (message, channel) =
            try_join!(message.json::<MsgInfo>(), channel.json::<ChannelInfo>()).context(ReqwestSnafu)?;
        Ok(EOInfo {
            author: message.author,
            name: channel.name,
            content: message.content,
            date: message.timestamp
                .parse::<DateTime<Utc>>()
                .unwrap()
                .date_naive(),
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

/// Wrapper around [handle_eo_id]. Extracts id from the link and passes it on
pub async fn handle_eo(eo_link: String, template: &str) -> Result<(), Error> {
    let id = eo_link
        .split('/')
        .nth(5)
        .whatever_context("invalid url")?
        .trim()
        .parse::<u64>()
        .whatever_context("invalid url")?;
    handle_eo_id(template, id).await?;
    Ok(())
}

/// Formats the EO, puts it in a template and uploads
pub async fn handle_eo_id(template: &str, eo_id: u64) -> Result<(), Error> {
    let xdg = BaseDirectories::with_prefix("sd-archivemanager").context(XdgSnafu)?;
    let (file, cookies) = get_cookies(&xdg).await?;
    let client = Arc::new(
        Client::builder()
            .cookie_provider(Arc::clone(&cookies))
            .build()
            .unwrap(),
    );
    let eo_client = Arc::clone(&client);
    let eo_handle = task::spawn(async move { EOInfo::scrape(eo_id, None, eo_client, None) });
    let mut templater = TinyTemplate::new();
    templater
        .add_template("template", &template)
        .whatever_context("invalid template")?;
    let cfg = CONFIG.lock().await;
    let mut eo = eo_handle.await.context(TokioSnafu)?.await?;
    eo.format(Target::EO, &cfg).await?;
    upload(
        &eo.name,
        &client,
        &get_token(&cfg, &client, &xdg).await?,
        &cfg,
        &templater
            .render("template", &eo)
            .whatever_context("failed to render")?,
    )
    .await?;
    cookies
        .lock()
        .unwrap()
        .save(&mut file.into_std().await, |s| serde_json::to_string(s))
        .expect("failed to save cookies");
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use std::fs;

//     use crate::guilds::Guilds;

//     use super::*;

//     #[test]
//     fn test_scrape_eo() {
//         let eo = scrape_eo(1348749282349416529, None, &Client::new(), None).unwrap();
//         assert_eq!(
//             eo,
//             EOInfo {
//                 author: "f3rri5_".to_string(),
//                 name: "EO 1239809 | Archive Supremacy".to_string(),
//                 content: "Lorum Ipsum".to_string(),
//                 date: "2025-03-10T20:08:07.462000+00:00"
//                     .parse::<DateTime<Utc>>()
//                     .unwrap()
//                     .date_naive(),
//             }
//         );
//     }
//     #[test]
//     fn format_eo() {
//         // setup regex
//         let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
//         let mut rgx = RegexManager::default();
//         rgx.save_regex(
//             r#"\s"#.to_string(),
//             "f3rri5_",
//             "001-Replace spaces",
//             true,
//             "eo",
//             r#":"#.to_string(),
//         )
//         .unwrap();
//         rgx.save().unwrap();
//         let eo_info = scrape_eo(1348749282349416529, None, &Client::new()).unwrap();
//         let eo = format_rgx(eo_info, &rgx);
//         assert_eq!(eo.name, "EO:1239809:|:Archive:Supremacy");
//         let _ = fs::remove_file("/root/.local/share/sd-archivemanager/regex.toml");
//     }
//     #[test]
//     fn list_eos_test() {
//         let guilds = Guilds::default();
//         let eos = list_eos(guilds.get_guilds().get(1).unwrap());
//         let expected = vec![
//             scrape_eo(1360569363140448326, None, &Client::new()).unwrap(),
//             scrape_eo(1348749282349416529, None, &Client::new()).unwrap(),
//         ];
//         assert_eq!(eos.unwrap(), expected);
//     }
//     #[test]
//     fn format_pre() {
//         let eo = scrape_eo(1348749282349416529, None, &Client::new()).unwrap();
//         let eo = format_plugins(eo, &vec![], PluginStage::Pre).unwrap();
//     }
// }

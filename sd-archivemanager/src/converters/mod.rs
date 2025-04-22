use std::{collections::HashMap, sync::Arc};

use tokio::{
    fs::{File, OpenOptions},
    io::AsyncReadExt,
    sync::MutexGuard,
    task,
};

use chrono::NaiveDate;
use cookie_store::CookieStore;
use extism::{Manifest, Plugin, Wasm, convert::Json};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure_whatever};
use xdg::BaseDirectories;

use crate::{
    CONFIG,
    config::Config,
    error::{Error, ExtismSnafu, IoSnafu, ReqwestSnafu, TokioSnafu},
    guilds::GuildInfo,
    plugins::{PluginManager, PluginManifest, PluginStage},
    regex::RegexManager,
};

pub mod case_law;
pub mod eo;
pub mod legislation;

pub async fn get_token(
    cfg: &MutexGuard<'_, Config>,
    client: &Client,
    xdg: &BaseDirectories,
) -> Result<String, Error> {
    let mut form = HashMap::new();
    form.insert("action", "query");
    form.insert("meta", "tokens");
    form.insert("type", "login");
    form.insert("format", "json");
    let login = client
        .get(&cfg.endpoint)
        .query(&form)
        .send()
        .await
        .context(ReqwestSnafu)?
        .json::<Value>()
        .await
        .context(ReqwestSnafu)?["query"]["tokens"]["logintoken"]
        .as_str()
        .whatever_context("invalid response")?
        .to_string();
    let mut form = HashMap::new();
    form.insert("format", "json");
    form.insert("action", "clientlogin");
    form.insert("loginreturnurl", &cfg.endpoint);
    form.insert("logintoken", &login);
    form.insert("username", &cfg.login);
    form.insert("password", &cfg.passwd);
    let login = client
        .post(&cfg.endpoint)
        .form(&form)
        .send()
        .await
        .context(ReqwestSnafu)?
        .json::<Value>()
        .await
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        login["clientlogin"]["status"].as_str() == Some("PASS"),
        "Login failed: {}",
        &login
    );
    drop(login);
    let mut form = HashMap::new();
    form.insert("action", "query");
    form.insert("meta", "tokens");
    form.insert("format", "json");
    let response = client
        .get(&cfg.endpoint)
        .query(&form)
        .send()
        .await
        .context(ReqwestSnafu)?
        .json::<Value>()
        .await
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        response["query"]["tokens"]["csrftoken"].as_str() != Some(r#"+\\"#),
        "failed to get token: {}",
        &response
    );
    Ok(response["query"]["tokens"]["csrftoken"]
        .as_str()
        .unwrap()
        .to_string())
}

pub async fn upload<'a>(
    name: &'a str,
    client: &Client,
    token: &'a str,
    cfg: &'a MutexGuard<'a, Config>,
    content: &str,
) -> Result<(), Error> {
    let mut form = HashMap::new();
    form.insert("action", "edit");
    form.insert("title", name);
    form.insert("token", token);
    form.insert("format", "json");
    form.insert("text", &content);
    form.insert("assertuser", &cfg.login);
    client
        .post(&cfg.endpoint)
        .form(&form)
        .send()
        .await
        .context(ReqwestSnafu)?
        .json::<Value>()
        .await
        .context(ReqwestSnafu)?;
    Ok(())
}

async fn get_cookies(
    xdg: &BaseDirectories,
) -> Result<(File, Arc<reqwest_cookie_store::CookieStoreMutex>), Error> {
    let cookie_file = xdg.place_data_file("cookies.json").unwrap();
    if cookie_file.is_file() {
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&cookie_file)
            .await
            .context(IoSnafu {
                file: cookie_file.to_string_lossy().to_string(),
            })?;
        let mut buf = vec![];
        let cookie_store = CookieStore::load(buf.as_slice(), |s| serde_json::from_str(s))
            .expect("cookies are broken");
        file.read_to_end(&mut buf).await;
        Ok((
            file,
            Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store)),
        ))
    } else {
        Ok((
            File::create_new(&cookie_file).await.unwrap(),
            Arc::new(reqwest_cookie_store::CookieStoreMutex::new(
                CookieStore::default(),
            )),
        ))
    }
}

pub trait PageData<'a>: Serialize + for<'de> Deserialize<'de> + Clone {
    fn get_content(&self) -> &str;
    fn get_title(&self) -> &str;
    fn get_date(&self) -> &NaiveDate;
    fn get_author(&self) -> &str;
    fn get_mut_content(&mut self) -> &mut String;
    fn get_mut_title(&mut self) -> &mut String;
    fn get_mut_date(&mut self) -> &mut NaiveDate;
    fn get_mut_author(&mut self) -> &mut String;
    async fn scrape(
        id: u64,
        conf: Option<&MutexGuard<'_, Config>>,
        client: Arc<Client>,
        guild: Option<&GuildInfo>,
    ) -> Result<Self, Error>;
    async fn format_plugins(
        &mut self,
        plugins: &Vec<&PluginManifest>,
        stage: PluginStage,
    ) -> Result<(), Error>
    where
        Self: Sized,
    {
        let cfg = task::spawn(async { CONFIG.lock().await });
        let mut iter = plugins
            .iter()
            .filter(|p| match stage {
                PluginStage::Post => p.post,
                PluginStage::Pre => p.pre,
            })
            .peekable();
        let mut res = Json::from(self.clone());
        if iter.peek().is_some() {
            let cfg = cfg.await.unwrap();
            while {
                let manifest = Manifest::new([Wasm::file(&iter.next().unwrap().path)])
                    .with_config_key("token", &cfg.token);
                let mut plugin = Plugin::new(&manifest, [], true).context(ExtismSnafu)?;
                res = plugin
                    .call::<_, Json<Self>>("pre", serde_json::to_vec(&self).unwrap())
                    .context(ExtismSnafu)?;
                iter.peek().is_some()
            } {} // do while loop
        }
        *self = res.into_inner();
        Ok(())
    }
    fn format_rgx(&mut self, rgx: &RegexManager) {
        let patterns = rgx.get_regexs(&self.get_author());
        for pat in patterns {
            let reg = Regex::new(&pat.regex).unwrap();
            if pat.for_title {
                *self.get_mut_title() =
                    reg.replace_all(&self.get_title(), &pat.replace).to_string();
            } else {
                *self.get_mut_content() = reg
                    .replace_all(&self.get_content(), &pat.replace)
                    .to_string();
            }
        }
    }
    async fn format(&mut self, target: &str) -> Result<&Self, Error> {
        let plugin_manager = task::spawn(async { PluginManager::load() });
        let rgx = task::spawn(async move { RegexManager::load() });
        let plugin_manager = plugin_manager.await.unwrap().await.unwrap_or_default();
        let plugins = plugin_manager.get_plugins(self.get_author(), target);
        self.format_plugins(&plugins, PluginStage::Pre).await?;
        self.format_rgx(&rgx.await.unwrap().await.unwrap_or_default());
        self.format_plugins(&plugins, PluginStage::Post).await?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::CONFIG;

    use super::*;

    #[tokio::test]
    async fn test_get_login() {
        let client = Client::builder().cookie_store(true).build().unwrap();
        let xdg = BaseDirectories::with_prefix("sd-archivemanager").unwrap();
        let token = get_token(&CONFIG.lock().await, &client, &xdg)
            .await
            .unwrap();
        fs::remove_file(xdg.place_data_file("cookies.json").unwrap()).unwrap();
    }
}

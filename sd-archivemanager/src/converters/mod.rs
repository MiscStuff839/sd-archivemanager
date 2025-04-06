use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Read,
    sync::MutexGuard,
};

use cookie_store::CookieStore;
use reqwest::blocking::Client;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure_whatever};

use crate::{
    config::Config,
    error::{Error, IoSnafu, ReqwestSnafu},
};

mod case_law;
mod eo;
mod legislation;

pub fn get_token(cfg: &MutexGuard<'_, Config>) -> Result<String, Error> {
    let mut form = HashMap::new();
    form.insert("action", "query");
    form.insert("meta", "tokens");
    form.insert("type", "login");
    form.insert("format", "json");
    let client = Client::new();
    let login = &client
        .post(&cfg.endpoint)
        .form(&form)
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?["query"]["tokens"]["logintoken"]
        .as_str()
        .whatever_context("invalid response")?
        .to_owned();
    let mut form = HashMap::new();
    form.insert("format", "json");
    form.insert("action", "clientlogin");
    form.insert("loginreturnurl", &cfg.endpoint);
    form.insert("logintoken", &login);
    form.insert("username", &cfg.login);
    form.insert("password", &cfg.passwd);
    let login = client.post(&cfg.endpoint).form(&form);
    let response = login
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        response["clientlogin"]["status"].as_str() == Some("PASS"),
        "Login failed: {}",
        &response
    );
    drop(response);
    let mut form = HashMap::new();
    form.insert("format", "json");
    form.insert("action", "query");
    form.insert("meta", "tokens");
    let csrf = &client
        .get(&cfg.endpoint)
        .query(&form)
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?["query"]["tokens"]["csrftoken"];
    Ok(csrf
        .as_str()
        .whatever_context("invalid response")?
        .to_string())
}

fn get_cookies() -> Result<(File, CookieStore), Error> {
    let xdg = xdg::BaseDirectories::with_prefix("sd-archivemanager").unwrap();
    let cookie_file = xdg.place_data_file("cookies.json").unwrap();
    if cookie_file.is_file() {
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&cookie_file)
            .context(IoSnafu {
                file: cookie_file.clone().to_string_lossy().to_string(),
            })?;
        let mut buf = vec![];
        file.read_to_end(&mut buf).context(IoSnafu {
            file: cookie_file.clone().to_string_lossy().to_string(),
        })?;
        let cookie_store = CookieStore::load(buf.as_slice(), |s| serde_json::from_str(s))
            .expect("cookies are broken");
        Ok((file, cookie_store))
    } else {
        Ok((
            File::create_new(&cookie_file).unwrap(),
            CookieStore::default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::CONFIG;

    use super::*;

    #[test]
    fn test_get_login() {
        let token = get_token(&CONFIG.lock().unwrap()).unwrap();
        dbg!(token);
    }
}

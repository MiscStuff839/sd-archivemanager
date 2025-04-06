use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Read,
    sync::{Arc, MutexGuard},
};

use cookie_store::CookieStore;
use reqwest::blocking::Client;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure_whatever};

use crate::{
    config::Config,
    error::{CookieStoreSnafu, Error, IoSnafu, ReqwestSnafu},
};

mod case_law;
mod eo;
mod legislation;

pub fn login(cfg: &MutexGuard<'_, Config>) -> Result<(), Error> {
    let (mut file, cookie_store) = get_cookies()?;
    let cookies = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));
    let mut form = HashMap::new();
    form.insert("action", "query");
    form.insert("meta", "tokens");
    form.insert("type", "login");
    form.insert("format", "json");
    let client = Client::builder()
        .cookie_provider(Arc::clone(&cookies))
        .build()
        .context(ReqwestSnafu)?;
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
    form.insert("logintoken", login);
    form.insert("username", &cfg.login);
    form.insert("password", &cfg.passwd);
    let login = client
        .post(&cfg.endpoint)
        .form(&form)
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        login["clientlogin"]["status"].as_str() == Some("PASS"),
        "Login failed: {}",
        &login
    );
    cookies
        .lock()
        .unwrap()
        .save(&mut file, |s| serde_json::to_string(s))
        .expect("failed to save cookies");
    Ok(())
}

pub fn get_token(cfg: &MutexGuard<'_, Config>) -> Result<String, Error> {
    let (_, cookie_store) = get_cookies()?;
    let cookies = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookie_store));
    let client = Client::builder()
        .cookie_provider(Arc::clone(&cookies))
        .build()
        .context(ReqwestSnafu)?;
    let mut form = HashMap::new();
    form.insert("action", "query");
    form.insert("meta", "tokens");
    form.insert("type", "csrf");
    form.insert("format", "json");
    let response = client
        .post(&cfg.endpoint)
        .form(&form)
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        response["query"]["tokens"]["csrftoken"].as_str().is_some(),
        "failed to get token: {}",
        &response
    );
    Ok(response["query"]["tokens"]["csrftoken"]
        .as_str()
        .unwrap()
        .to_owned())
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
    use std::fs;

    use crate::CONFIG;

    use super::*;

    #[test]
    fn test_get_login() {
        login(&CONFIG.lock().unwrap()).unwrap();
        let _ = get_token(&CONFIG.lock().unwrap()).unwrap();
        fs::remove_file(
            xdg::BaseDirectories::with_prefix("sd-archivemanager")
                .unwrap()
                .place_data_file("cookies.json")
                .unwrap(),
        )
        .unwrap();
    }
}

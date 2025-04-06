use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufWriter, Read},
    sync::{Arc, MutexGuard},
};

use reqwest::{blocking::Client, header::HeaderMap};
use reqwest_cookie_store::CookieStore;
use serde::Serialize;
use serde_json::Value;
use snafu::{ResultExt, ensure_whatever};

use crate::{
    config::Config,
    cookies_to_string,
    error::{CookieStoreSnafu, Error, IoSnafu, ReqwestSnafu, XdgSnafu},
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
    let (cookie_file, cookies) = get_cookies()?;
    let cookies = Arc::new(reqwest_cookie_store::CookieStoreMutex::new(cookies));
    let client = Client::builder()
        .cookie_provider(Arc::clone(&cookies))
        .build()
        .context(ReqwestSnafu)?;
    let login = &client
        .post(&cfg.endpoint)
        .form(&form)
        .header("Cookie", cookies_to_string!(cookies))
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?
        .get("query")
        .unwrap()
        .get("tokens")
        .unwrap()
        .get("logintoken")
        .unwrap()
        .to_string();
    dbg!(&login.as_bytes());
    let mut form = HashMap::new();
    form.insert("format", "json");
    form.insert("action", "clientlogin");
    form.insert("loginreturnurl", &cfg.endpoint);
    form.insert("logintoken", &login);
    form.insert("username", &cfg.login);
    form.insert("password", &cfg.passwd);
    let login = client.post(&cfg.endpoint).form(&form);
    dbg!(&login.try_clone().unwrap().build().unwrap().body());
    let response = login
        .header("Cookie", cookies_to_string!(cookies))
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?;
    ensure_whatever!(
        response["clientlogin"]["status"].to_string() == "PASS",
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
        .header("Cookie", cookies_to_string!(cookies))
        .query(&form)
        .send()
        .context(ReqwestSnafu)?
        .json::<Value>()
        .context(ReqwestSnafu)?["query"]["tokens"]["csrftoken"];
    let store = cookies.lock().unwrap();
    store
        .save(&mut BufWriter::new(cookie_file), |c| {
            serde_json::to_string_pretty(c)
        })
        .context(CookieStoreSnafu)?;
    Ok(csrf.as_str().unwrap().to_string())
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

use std::sync::{Mutex, MutexGuard};

use config::Config;
pub use once_cell::sync::Lazy;

pub mod config;
pub mod converters;
pub mod error;
pub mod guilds;
pub mod regex;
pub mod plugins;

#[macro_export]
macro_rules! context_opt {
    ($v:ident = $s:stmt, $err:expr) => {
        let $v = $s;
        if $v.is_none() {
            return Err($err);
        }
    };
}

pub static CONFIG: Lazy<Mutex<Config>> =
    Lazy::new(|| Mutex::new(Config::load().unwrap_or_default()));

#[inline(always)]
fn format_auth(cfg: &MutexGuard<'_, Config>) -> String {
    if cfg.bot {
        format!("Bot {}", cfg.token)
    } else {
        cfg.token.clone()
    }
}

#[macro_export]
macro_rules! cookies_to_string {
    ($c:ident) => {{
        let store = $c.lock().unwrap();
        let mut cookie_string = String::new();
        for cookie in store.iter_any() {
            cookie_string.push_str(format!("{}={};", cookie.name(), cookie.value()).as_str());
        }
        cookie_string
    }};
}

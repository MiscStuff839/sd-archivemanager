use tokio::sync::{Mutex, MutexGuard};

use config::Config;
pub use once_cell::sync::Lazy;

pub mod config;
pub mod converters;
pub mod error;
pub mod guilds;
pub mod regex;
pub mod plugins;

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

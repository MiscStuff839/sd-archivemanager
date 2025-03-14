use lazy_static::lazy_static;

lazy_static! {
    // pub static ref CONFIG: config::Config = config::Config::load().unwrap_or_default();
}

pub mod error;
pub mod config;
mod converters;
pub mod regex;
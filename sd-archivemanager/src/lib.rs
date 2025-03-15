use lazy_static::lazy_static;

lazy_static! {
    // pub static ref CONFIG: config::Config = config::Config::load().unwrap_or_default();
}

pub mod error;
pub mod config;
mod converters;
pub mod regex;
pub mod guilds;

#[macro_export]
macro_rules! context_opt {
    ($v:ident = $s:stmt, $err:expr) => {
        let $v = $s;
        if $v.is_none() {
            return Err($err);
        }
    };
}
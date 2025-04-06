use std::path::PathBuf;

use snafu::Snafu;
use xdg::BaseDirectoriesError;

#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("I/O error for {}: {source}", file.to_string_lossy()))]
    IoError {
        source: std::io::Error,
        file: PathBuf,
    },
    #[snafu(display("Invalid configuration: {}", source))]
    InvalidConfig { source: toml::de::Error },
    #[snafu(display("Invalid regex: {}", source))]
    InvalidRegex { source: regex::Error },
    #[snafu(display("Invalid path: {}", source))]
    XdgError { source: BaseDirectoriesError },
    #[snafu(whatever, display("{}\nunderlying: {:#?}", message, source))]
    Whatever {
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
        message: String,
    },
    #[snafu(display("File not found: {}", file.to_string_lossy()))]
    FileNotFound { file: PathBuf },
    #[snafu(display("Reqwest error: {}", source))]
    ReqwestError { source: reqwest::Error },
    #[snafu(display("Invalid JSON: {}", source))]
    JSONError { source: serde_json::Error },
    #[snafu(display("Cookie store error: {}", source))]
    CookieStoreError { source: cookie_store::Error}
}
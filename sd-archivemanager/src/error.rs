
use snafu::Snafu;
use xdg::BaseDirectoriesError;
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("I/O error for {file}: {source}"))]
    IoError {
        source: std::io::Error,
        file: String,
    },
    #[snafu(display("Invalid configuration: {}", source))]
    InvalidConfig {
        source: toml::de::Error,
    },
    #[snafu(display("Invalid regex: {}", source))]
    InvalidRegex {
        source: regex::Error,
    },
    #[snafu(display("Invalid path: {}", source))]
    XdgError {
        source: BaseDirectoriesError,
    },
}

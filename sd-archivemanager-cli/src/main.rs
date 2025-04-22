use std::fs;

use clap::Parser;
use sd_archivemanager::{
    converters::{eo::handle_eo_id, legislation::handle_law_id},
    guilds::Guilds,
};
use xdg::BaseDirectories;

#[derive(Debug, Parser)]
#[command(
    name = "sd-arch",
    about = "A CLI for managing the simdem archives",
    rename_all = "kebab-case"
)]
struct Args {
    /// Specify the guild to refer to
    #[clap(short, long)]
    guild: String,

    /// Subcommand to execute
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum LawCommand {
    /// Upload a law document
    Upload { id: u64 },
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// EO-related commands
    EO {
        #[clap(subcommand)]
        subcommand: EOCommand,
    },
    /// Law-related commands
    Law {
        #[clap(subcommand)]
        subcommand: LawCommand,
    },
}
#[derive(Debug, clap::Subcommand)]
enum EOCommand {
    /// Upload a link or ID
    Upload { id: u64 },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let xdg = BaseDirectories::with_prefix("sd-archivemanager").unwrap();
    match args.command {
        Command::EO { subcommand } => match subcommand {
            EOCommand::Upload { id } => {
                handle_eo_id(
                    fs::read_to_string(xdg.place_config_file("eo_template").unwrap())
                        .unwrap()
                        .as_str(),
                    id,
                )
                .await
                .unwrap();
            }
        },
        Command::Law { subcommand } => {
            let guildman = Guilds::load().unwrap_or_default();
            let guild = guildman
                .get_guilds()
                .iter()
                .find(|g| g.name == args.guild)
                .unwrap_or_else(|| panic!("Guild {} not found", args.guild));
            match subcommand {
                LawCommand::Upload { id } => handle_law_id(
                    fs::read_to_string(xdg.place_config_file("law_template").unwrap())
                        .unwrap_or("{content}".to_string())
                        .as_str(),
                    id,
                    guild,
                )
                .await
                .unwrap(),
            };
        }
    }
}

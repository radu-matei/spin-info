use anyhow::Result;
use clap::{Parser, Subcommand};
use info::InfoCommand;

pub mod app_source;
pub mod info;

#[derive(Parser, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    // The default command is `info`.
    #[command(flatten)]
    info: InfoCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    // Hide the command from help for now, since it's the default.
    #[clap(hide = true)]
    Info(InfoCommand),
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Some(c) => match c {
                Command::Info(cmd) => cmd.run().await,
            },
            None => {
                let cmd = InfoCommand::parse();
                cmd.run().await
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Cli::parse();
    app.run().await
}

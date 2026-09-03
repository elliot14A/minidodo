use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "minidodo", version = "0.1.0", about = "minidodo invoice & payment service")]
#[command(propagate_version = true)]
pub struct MinidodoCli {
    #[command(subcommand)]
    pub command: Option<MinidodoCommands>,
}

#[derive(Subcommand, Debug, Default)]
pub enum MinidodoCommands {
    #[default]
    Server,
    Migrate,
    Psp,
    Worker,
}

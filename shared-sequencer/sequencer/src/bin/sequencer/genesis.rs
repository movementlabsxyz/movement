use clap::{arg, Command};

pub const NAME: &str = "genesis";

#[must_use]
pub fn command() -> Command {
    Command::new(NAME)
        .about("Write a genesis file")
        .arg(arg!(<DATA> "Genesis message data"))
        .arg_required_else_help(true)
}

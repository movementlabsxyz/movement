use clap::{arg, Command};

pub const NAME: &str = "vm-id";

#[must_use]
pub fn command() -> Command {
    Command::new(NAME)
        .about("Converts a given Vm name string to Vm Id")
        .arg(arg!(<VM_NAME> "A name of the Vm"))
        .arg_required_else_help(true)
}

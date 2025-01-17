use clap::{Parser, Subcommand};

pub mod rotate_key;

#[derive(Parser, Debug)]
#[clap(name = "signing-admin", about = "CLI for managing signing keys")]
pub struct CLI {
        #[clap(subcommand)]
        pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
        RotateKey {
                #[clap(long, help = "Canonical string of the key (alias for the backend key)")]
                canonical_string: String,

                #[clap(long, help = "Application URL to notify about the key rotation")]
                application_url: String,

                #[clap(long, help = "Backend to use (e.g., 'vault', 'aws')")]
                backend: String,
        },
}

use clap::{Parser, Subcommand};
use super::install::Install;
use util::util::util::Version;
use semver::Version as SemverVersion;
use util::cli::Command;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Manage the Movement CLI and artifacts"
)]
pub enum Manage {
    #[clap(subcommand)]
    Install(Install)
}

#[async_trait::async_trait]
impl Command<String> for Manage {

    async fn get_name(&self) -> String {
        "manage".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            Manage::Install(install) => {
                install.execute().await?;
            }
        }

        Ok("SUCCESS".to_string())
    }

}

#[derive(Debug, Clone, Parser)]
pub struct VersionArgs {
    #[clap(
        long,
        default_value = "latest"
    )]
    pub ver : String
}

#[derive(Debug, Clone, Parser)]
pub struct InstallationArgs {
    #[clap(
        long,
        default_value_t = false
    )]
    pub build : bool
}

impl TryInto<Version> for VersionArgs {

    type Error = anyhow::Error;
    fn try_into(self) -> Result<Version, anyhow::Error> {

        if self.ver == "latest" {
            Ok(Version::Latest)
        } else {
            let semver = SemverVersion::parse(&self.ver)?;
            Ok(Version::Version(semver))
        }
        
    }
}
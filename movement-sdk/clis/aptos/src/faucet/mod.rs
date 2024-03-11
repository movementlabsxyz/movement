// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliTypedResult}
};
use async_trait::async_trait;
use crate::common::types::{FaucetOptions, ProfileOptions, RestOptions};
use crate::common::utils::{fund_pub_key, wait_for_transactions};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct FaucetTool {
    #[clap(long)]
    pub_key: Option<String>,
    #[clap(flatten)]
    pub(crate) faucet_options: FaucetOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

impl FaucetTool {
    fn pub_key(&self, profile: &ProfileOptions) -> CliTypedResult<String> {
        match &self.pub_key {
            Some(pub_key) => Ok(pub_key.clone()),
            None => Ok((profile.public_key()?).to_string()),
        }
    }
}

#[async_trait]
impl CliCommand<String> for FaucetTool {
    fn command_name(&self) -> &'static str {
        "Faucet"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let profile = &self.profile_options;
        let hashes = fund_pub_key(
            self.faucet_options.faucet_url(&profile)?,
            self.pub_key(&profile)?,
        ).await?;
        let client = self.rest_options.client_raw(self.faucet_options.faucet_url(&profile)?)?;
        wait_for_transactions(&client, hashes).await?;
        return Ok(format!(
            "Added 10 MOV to account {}", self.pub_key(&profile)?
        ));
    }
}


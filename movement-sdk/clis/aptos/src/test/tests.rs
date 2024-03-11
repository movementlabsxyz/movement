// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_tool::{ArgWithType, FunctionArgType},
    CliResult, Tool,
};
use clap::Parser;
use std::str::FromStr;

/// In order to ensure that there aren't duplicate input arguments for untested CLI commands,
/// we call help on every command to ensure it at least runs
#[tokio::test]
async fn ensure_every_command_args_work() {
    assert_cmd_not_panic(&["movement"]).await;

    assert_cmd_not_panic(&["movement", "account"]).await;
    assert_cmd_not_panic(&["movement", "account", "create", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "create-resource-account", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "fund-with-faucet", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "list", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "lookup-address", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "rotate-key", "--help"]).await;
    assert_cmd_not_panic(&["movement", "account", "transfer", "--help"]).await;

    assert_cmd_not_panic(&["movement", "config"]).await;
    assert_cmd_not_panic(&["movement", "config", "generate-shell-completions", "--help"]).await;
    assert_cmd_not_panic(&["movement", "config", "init", "--help"]).await;
    assert_cmd_not_panic(&["movement", "config", "set-global-config", "--help"]).await;
    assert_cmd_not_panic(&["movement", "config", "show-global-config"]).await;
    assert_cmd_not_panic(&["movement", "config", "show-profiles"]).await;

    assert_cmd_not_panic(&["movement", "genesis"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "generate-genesis", "--help"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "generate-keys", "--help"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "generate-layout-template", "--help"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "set-validator-configuration", "--help"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "setup-git", "--help"]).await;
    assert_cmd_not_panic(&["movement", "genesis", "generate-admin-write-set", "--help"]).await;

    assert_cmd_not_panic(&["movement", "governance"]).await;
    assert_cmd_not_panic(&["movement", "governance", "execute-proposal", "--help"]).await;
    assert_cmd_not_panic(&["movement", "governance", "generate-upgrade-proposal", "--help"]).await;
    assert_cmd_not_panic(&["movement", "governance", "propose", "--help"]).await;
    assert_cmd_not_panic(&["movement", "governance", "vote", "--help"]).await;

    assert_cmd_not_panic(&["movement", "info"]).await;

    assert_cmd_not_panic(&["movement", "init", "--help"]).await;

    assert_cmd_not_panic(&["movement", "key"]).await;
    assert_cmd_not_panic(&["movement", "key", "generate", "--help"]).await;
    assert_cmd_not_panic(&["movement", "key", "extract-peer", "--help"]).await;

    assert_cmd_not_panic(&["movement", "move"]).await;
    assert_cmd_not_panic(&["movement", "move", "clean", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "compile", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "compile-script", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "download", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "init", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "list", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "prove", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "publish", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "run", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "run-script", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "test", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "transactional-test", "--help"]).await;
    assert_cmd_not_panic(&["movement", "move", "view", "--help"]).await;

    assert_cmd_not_panic(&["movement", "node"]).await;
    assert_cmd_not_panic(&["movement", "node", "check-network-connectivity", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "get-stake-pool", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "analyze-validator-performance", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "bootstrap-db-from-backup", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "initialize-validator", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "join-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "leave-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "run-local-testnet", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "show-validator-config", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "show-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "show-validator-stake", "--help"]).await;
    assert_cmd_not_panic(&["movement", "node", "update-consensus-key", "--help"]).await;
    assert_cmd_not_panic(&[
        "movement",
        "node",
        "update-validator-network-addresses",
        "--help",
    ])
    .await;

    assert_cmd_not_panic(&["movement", "stake"]).await;
    assert_cmd_not_panic(&["movement", "stake", "add-stake", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "increase-lockup", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "initialize-stake-owner", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "set-delegated-voter", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "set-operator", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "unlock-stake", "--help"]).await;
    assert_cmd_not_panic(&["movement", "stake", "withdraw-stake", "--help"]).await;
}

/// Ensure we can parse URLs for args
#[tokio::test]
async fn ensure_can_parse_args_with_urls() {
    let result = ArgWithType::from_str("string:https://aptoslabs.com").unwrap();
    matches!(result._ty, FunctionArgType::String);
    assert_eq!(
        result.arg,
        bcs::to_bytes(&"https://aptoslabs.com".to_string()).unwrap()
    );
}

async fn assert_cmd_not_panic(args: &[&str]) {
    // When a command fails, it will have a panic in it due to an improperly setup command
    // thread 'main' panicked at 'Command propose: Argument names must be unique, but 'assume-yes' is
    // in use by more than one argument or group', ...

    match run_cmd(args).await {
        Ok(inner) => assert!(
            !inner.contains("panic"),
            "Failed to not panic cmd {}: {}",
            args.join(" "),
            inner
        ),
        Err(inner) => assert!(
            !inner.contains("panic"),
            "Failed to not panic cmd {}: {}",
            args.join(" "),
            inner
        ),
    }
}

async fn run_cmd(args: &[&str]) -> CliResult {
    let tool: Tool = Tool::try_parse_from(args).map_err(|msg| msg.to_string())?;
    tool.execute().await
}

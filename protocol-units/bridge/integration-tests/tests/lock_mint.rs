use bridge_config::Config;
use std::process::{Command, Stdio};
use anyhow::{Result, Context};

#[tokio::test]
async fn test_lock_mint() -> Result<()> {
    // Define bridge config path
    let mock_config = Config::default();

    // Transfer 1 coin to 0xdead
    let transfer_output = Command::new("movement")
        .args(&[
            "move",
            "run",
            "--function-id",
            "0x1::aptos_account::transfer",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
            "u64:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    println!("transfer_output: {:?}", transfer_output);

    if !transfer_output.stdout.is_empty() {
        println!(
            "transfer stdout: {}",
            String::from_utf8_lossy(&transfer_output.stdout)
        );
    }
    if !transfer_output.stderr.is_empty() {
        eprintln!(
            "transfer stderr: {}",
            String::from_utf8_lossy(&transfer_output.stderr)
        );
    }

    // Get 0xdead balance
    let dead_balance_output = Command::new("movement")
        .args(&[
            "move",
            "view",
            "--function-id",
            "0x1::coin::coin_balance",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !dead_balance_output.stdout.is_empty() {
        println!(
            "dead_balance stdout: {}",
            String::from_utf8_lossy(&dead_balance_output.stdout)
        );
    }
    if !dead_balance_output.stderr.is_empty() {
        eprintln!(
            "dead_balance stderr: {}",
            String::from_utf8_lossy(&dead_balance_output.stderr)
        );
    }

    let dead_balance_str = String::from_utf8(dead_balance_output.stdout)?
        .trim()
        .to_string();

    let dead_balance: u64 = dead_balance_str.parse()?;

    // Burn 0xdead balance
    let burn_output = Command::new("movement")
        .args(&[
            "move",
            "run",
            "--function-id",
            "0x1::coin::burn_from",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x000000000000000000000000000000000000000000000000000000000000dead",
            &format!("u64:{}", dead_balance),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !burn_output.stdout.is_empty() {
        println!(
            "burn stdout: {}",
            String::from_utf8_lossy(&burn_output.stdout)
        );
    }
    if !burn_output.stderr.is_empty() {
        eprintln!(
            "burn stderr: {}",
            String::from_utf8_lossy(&burn_output.stderr)
        );
    }

    // Get the L2 balance of the bridge relayer account
    let bridge_relayer_output = Command::new("movement")
        .args(&[
            "move",
            "view",
            "--function-id",
            "0x1::native_bridge::get_bridge_relayer",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !bridge_relayer_output.stdout.is_empty() {
        println!(
            "bridge_relayer stdout: {}",
            String::from_utf8_lossy(&bridge_relayer_output.stdout)
        );
    }
    if !bridge_relayer_output.stderr.is_empty() {
        eprintln!(
            "bridge_relayer stderr: {}",
            String::from_utf8_lossy(&bridge_relayer_output.stderr)
        );
    }

    let bridge_relayer = String::from_utf8(bridge_relayer_output.stdout)?
        .trim()
        .to_string();

    // Get the bridge relayer's balance
    let balance_output = Command::new("movement")
        .args(&[
            "move",
            "view",
            "--function-id",
            "0x1::coin::coin_balance",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            &format!("address:{}", bridge_relayer),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !balance_output.stdout.is_empty() {
        println!(
            "balance stdout: {}",
            String::from_utf8_lossy(&balance_output.stdout)
        );
    }
    if !balance_output.stderr.is_empty() {
        eprintln!(
            "balance stderr: {}",
            String::from_utf8_lossy(&balance_output.stderr)
        );
    }

    let balance_str = String::from_utf8(balance_output.stdout)?
        .trim()
        .to_string();

    let balance: u64 = balance_str.parse()?;
    let desired_balance = 1_000_000_000_000_000;
    let burn_balance = balance.saturating_sub(desired_balance);

    if burn_balance > 0 {
        let excess_burn_output = Command::new("movement")
            .args(&[
                "move",
                "run",
                "--function-id",
                "0x1::coin::burn_from",
                "--type-args",
                "0x1::aptos_coin::AptosCoin",
                "--args",
                &format!("address:{}", bridge_relayer),
                &format!("u64:{}", burn_balance),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !excess_burn_output.stdout.is_empty() {
            println!(
                "excess_burn stdout: {}",
                String::from_utf8_lossy(&excess_burn_output.stdout)
            );
        }
        if !excess_burn_output.stderr.is_empty() {
            eprintln!(
                "excess_burn stderr: {}",
                String::from_utf8_lossy(&excess_burn_output.stderr)
            );
        }
    }

    Ok(())
}

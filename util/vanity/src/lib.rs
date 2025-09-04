pub mod miner {
    use aptos_sdk::types::{
        account_address::{create_resource_address, AccountAddress},
        LocalAccount,
    };
    use rand::thread_rng;
    use rand::Rng;
    use rayon::prelude::*;
    use rayon::ThreadPoolBuilder;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    use std::time::Instant;
    
    fn matches(addr: &AccountAddress, start: &str, end: &str) -> bool {
        let addr_hex = hex::encode(addr);
    
        (start.is_empty() || addr_hex.starts_with(start)) && (end.is_empty() || addr_hex.ends_with(end))
    }
    
    pub fn mine_move_address(starts: &str, ends: &str, threads: usize) -> LocalAccount {
        let found = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(None));
        let start_time = Instant::now();
    
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Failed to build thread pool");
    
        pool.install(|| {
            (0u64..=u64::MAX).into_par_iter().find_any(|_| {
                if found.load(Ordering::Relaxed) {
                    return false;
                }
                let mut rng = thread_rng();
                let account = LocalAccount::generate(&mut rng);
                if matches(&account.address(), starts, ends) {
                    let mut lock = result.lock().unwrap();
                    *lock = Some(account);
                    found.store(true, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            });
        });
    
        println!("Mining completed in {:?}", start_time.elapsed());
    
        let final_result = std::mem::take(&mut *result.lock().unwrap());
        final_result.expect("No matching account found")
    }
    
    pub fn mine_resource_address(
        address: &AccountAddress,
        starts: &str,
        ends: &str,
        threads: usize,
    ) -> (AccountAddress, Vec<u8>) {
        let found = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(None));
        let start_time = Instant::now();
    
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Failed to build thread pool");
    
        pool.install(|| {
            (0u64..=u64::MAX).into_par_iter().find_any(|_| {
                if found.load(Ordering::Relaxed) {
                    return false;
                }
                let mut rng = thread_rng();
                let mut seed_bytes = [0u8; 32]; // or any size you need
                rng.fill(&mut seed_bytes);
                let resource_address = create_resource_address(*address, &seed_bytes);
                if matches(&resource_address, starts, ends) {
                    let mut lock = result.lock().unwrap();
                    *lock = Some((resource_address, seed_bytes.to_vec()));
                    found.store(true, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            });
        });
    
        println!("Mining completed in {:?}", start_time.elapsed());
    
        let final_result = std::mem::take(&mut *result.lock().unwrap());
        final_result.expect("No matching account found")
    }    
}

pub mod cli {
    use clap::{Parser, Subcommand};
    use std::{ops::Deref, str::FromStr};

    /// Pattern type for hex string filtering.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Pattern(Box<str>);

    impl Deref for Pattern {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Pattern {
        /// Return the raw hex string without trying to decode it.
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    /// Pattern parsing errors.
    #[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
    pub enum PatternError {
        #[error("the pattern's length exceeds 39 characters or the pattern is empty")]
        InvalidPatternLength,
        #[error("the pattern is not in hexadecimal format")]
        NonHexPattern,
    }

    impl FromStr for Pattern {
        type Err = PatternError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.len() >= 40 || s.is_empty() {
                return Err(PatternError::InvalidPatternLength);
            }
            if s.chars().any(|c| !c.is_ascii_hexdigit()) {
                return Err(PatternError::NonHexPattern);
            }
            Ok(Self(s.into()))
        }
    }

    /// CLI definition for the `vanity` tool.
    #[derive(Parser, Debug)]
    #[command(name = "vanity", about = "Vanity is a fast vanity move & resource address miner.")]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Vanity,
    }

    /// Supported CLI subcommands.
    #[derive(Debug, Subcommand)]
    pub enum Vanity {
        /// Generate a Move address with a specific pattern.
        Move {
            #[clap(long)]
            starts: Option<String>,
            #[clap(long)]
            ends: Option<String>,
        },
        /// Generate a Resource address with a specific pattern.
        Resource {
            #[clap(long)]
            address: Option<String>,
            #[clap(long)]
            starts: Option<String>,
            #[clap(long)]
            ends: Option<String>,
        },
    }

    #[cfg(test)]
    mod tests {
        use super::{Cli, Pattern, Vanity};
        use assert_cmd::Command;
        use clap::Parser;
        use std::str::FromStr;

        #[test]
        fn test_valid_pattern() {
            let p = Pattern::from_str("abcd").unwrap();
            assert_eq!(&*p, "abcd");
        }

        #[test]
        fn test_cli_parsing_starts() {
            let cli = Cli::parse_from(["vanity", "move", "--starts", "abcd"]);
            match cli.command {
                Vanity::Move { starts, ends } => {
                    assert_eq!(starts, Some("abcd".to_string()));
                    assert_eq!(ends, None);
                }
                _ => panic!("Expected Move variant"),
            }
        }

        #[test]
        fn test_resource_parsing_starts() {
            let cli = Cli::parse_from([
                "vanity",
                "resource",
                "--address",
                "0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f",
                "--starts",
                "abcd",
            ]);
            match cli.command {
                Vanity::Resource { address, starts, ends } => {
                    assert_eq!(
                        address,
                        Some(
                            "0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f"
                                .to_string()
                        )
                    );
                    assert_eq!(starts, Some("abcd".to_string()));
                    assert_eq!(ends, None);
                }
                _ => panic!("Expected Resource variant"),
            }
        }

        #[test]
        fn test_cli_runs_with_starts() {
            let mut cmd = Command::cargo_bin("vanity").unwrap();
            let assert = cmd.args(&["move", "--starts", "de"]).assert().success();

            let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
            let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
            let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

            assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
        }

        #[test]
        fn test_cli_runs_with_ends() {
            let mut cmd = Command::cargo_bin("vanity").unwrap();
            let assert = cmd.args(&["move", "--ends", "f0"]).assert().success();

            let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
            let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
            let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

            assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
        }

        #[test]
        fn test_cli_runs_with_starts_and_ends() {
            let mut cmd = Command::cargo_bin("vanity").unwrap();
            let assert = cmd.args(&["move", "--starts", "de", "--ends", "f0"]).assert().success();

            let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
            let address_line = output.lines().find(|l| l.contains("Found Move address:")).unwrap();
            let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

            assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
            assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
        }

        #[test]
        fn test_resource_runs_with_starts_and_ends() {
            let mut cmd = Command::cargo_bin("vanity").unwrap();
            let assert = cmd
                .args(&[
                    "resource",
                    "--address",
                    "0x5e04c2f5bf1a89d3431d2c047626acbba41c900a69b10e7c24e5b919802c531f",
                    "--starts",
                    "de",
                    "--ends",
                    "f0",
                ])
                .assert()
                .success();

            let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
            let address_line = output.lines().find(|l| l.contains("Found Resource Address:")).unwrap();
            let address = address_line.split(':').nth(1).unwrap().trim().trim_start_matches("0x");

            assert!(address.starts_with("de"), "Address does not start with 'de': {}", address);
            assert!(address.ends_with("f0"), "Address does not end with 'f0': {}", address);
        }
    }

}
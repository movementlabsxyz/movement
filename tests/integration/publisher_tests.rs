use std::sync::Arc;
use bonsai_sdk::alpha_async::upload_img;
use eth_contracts::{Counter, Settlement};
use ethers::prelude::*;
use ethers::types::{Bytes, H256 as ethers_H256, U256};
use integration_tests::{get_anvil, get_bonsai_key, get_bonsai_url, get_ethers_client_config, get_bonsai_client};
use risc0_ethereum_contracts::{BonsaiRelay, BonsaiTestRelay};
use risc0_ethereum_relay::Relayer;
use eth_relay_test_methods::{SLICE_IO_ELF, SLICE_IO_ID};
use tokio::time::{sleep, Duration};

async fn test_publisher() {
    let anvil = get_anvil();

    let ethers_client_config = get_ethers_client_config(anvil.as_ref())
        .await
        .expect("Failed to get ethers client config");
    let ethers_client = Arc::new(
        ethers_client_config
            .get_client()
            .await
            .expect("Failed to get ethers client"),
    );

    let relay_contract = match risc0_zkvm::is_dev_mode() {
        true => BonsaiTestRelay::deploy(ethers_client.clone(), ethers_client.signer().chain_id())
            .expect("unable to deploy the BonsaiTestRelay contract")
            .send()
            .await
            .expect("unable to send the BonsaiTestRelay contract")
            .address(),
        false => {
            let control_id_0 = U256::from_str_radix("0x447d7e12291364db4bc5421164880129", 16)
                .expect("unable to parse control_id_0");
            let control_id_1 = U256::from_str_radix("0x12c49ad247d28a32147e13615c6c81f9", 16)
                .expect("unable to parse control_id_1");

            let verifier = Settlement::deploy(ethers_client.clone(), (control_id_0, control_id_1))
                .expect("unable to deploy the Settlement contract")
                .send()
                .await
                .expect("unable to send the Settlement contract")
                .address();

            BonsaiRelay::deploy(ethers_client.clone(), verifier)
                .expect("unable to deploy the BonsaiRelay contract")
                .send()
                .await
                .expect("unable to send the BonsaiRelay contract")
                .address()
        }
    };

    let counter = Counter::deploy(ethers_client.clone(), ())
        .expect("unable to deploy the Counter contract")
        .send()
        .await
        .expect("unable to send the Counter contract")
        .address();

    // run the bonsai relayer
    let relayer = Relayer {
        rest_api: false,
        dev_mode: risc0_zkvm::is_dev_mode(),
        rest_api_port: "8080".to_string(),
        bonsai_api_key: get_bonsai_url(),
        bonsai_api_url: get_bonsai_key(),
        relay_contract_address: relay_contract,
    };

    dbg!("starting relayer");
    tokio::spawn(relayer.run(ethers_client_config.clone()));

    // Wait for relayer to start
    sleep(Duration::from_secs(2)).await;

    // register elf
    let bonsai_client = get_bonsai_client(get_api_key()).await;
    // crfeate the memoryImg, upload it and return theimageId
    let image_id_bytes: [u8; 32] = bytemuck::cast(SLICE_IO_ID);
    let image_id = hex::encode(image_id_bytes);
    upload_img(
        bonsai_client.clone(), 
        image_id.clone(), 
        SLICE_IO_ELF.to_vec())
        .await
        .expect("Failed to upload elf");
}

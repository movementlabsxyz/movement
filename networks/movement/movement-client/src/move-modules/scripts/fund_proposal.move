script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;

    fun main(core_resources: &signer, beneficiary_address: address, amount: u64) {
        // Get the framework signer
        let framework_signer = aptos_governance::get_signer_testnet_only(
            core_resources,
            @0x1  // Address of the Aptos Framework on the testnet
        );

        // Deposit tokens into the governed gas pool for the beneficiary account
        governed_gas_pool::deposit_from(
            &framework_signer,
            beneficiary_address,
            amount
        );
    }
}


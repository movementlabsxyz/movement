script {
    use aptos_framework::aptos_governance;
    use aptos_framework::governed_gas_pool;

    /// A script to deposit tokens from the governed gas pool to a beneficiary account.
    /// 
    /// Parameters:
    /// - `core_resources`: The signer of the Aptos Framework.
    /// - `beneficiary_address`: The address of the account to deposit tokens into.
    /// - `amount`: The amount of tokens to deposit.
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


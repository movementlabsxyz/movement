script {
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_governance;
    use aptos_framework::coin;
    use aptos_framework::coin::{BurnCapability};
    use aptos_framework::aptos_coin::AptosCoin;


    fun burn_from(core_resources: &signer, account: address, amount: u64, burn_cap: &BurnCapability<AptosCoin>) {
        coin::burn_from<AptosCoin>(account, amount, burn_cap);
    }
} 
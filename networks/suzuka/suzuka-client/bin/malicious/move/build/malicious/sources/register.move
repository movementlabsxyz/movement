script {
    fun register(account: &signer) {
        aptos_framework::managed_coin::register<malicious_test::moon_coin::MoonCoin>(account)
    }
}


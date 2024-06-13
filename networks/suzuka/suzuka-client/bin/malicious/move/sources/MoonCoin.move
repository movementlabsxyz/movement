module malicious_test::moon_coin {
    struct MoonCoin {}

    fun init_module(sender: &signer) {
        aptos_framework::managed_coin::initialize<MoonCoin>(
            sender,
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
    }

    //to test that everything works
    public entry fun test() { //account: &signer
    }
}

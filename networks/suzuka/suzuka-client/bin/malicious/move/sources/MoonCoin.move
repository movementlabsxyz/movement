module malicious_test::moon_coin {
    struct MoonCoin {}

    fun init_module(admin: &signer) {
        aptos_framework::managed_coin::initialize<MoonCoin>(
            admin,
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
        aptos_framework::managed_coin::register<MoonCoin>(admin);
    }

}

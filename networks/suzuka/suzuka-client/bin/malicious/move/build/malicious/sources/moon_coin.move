//:!:>moon
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

    public entry fun test() { //account: &signer
    }
}
//<:!:moon

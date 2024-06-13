module malicious_test::moon_coin {
    struct MoonCoin {}

<<<<<<< HEAD
    fun init_module(admin: &signer) {
        aptos_framework::managed_coin::initialize<MoonCoin>(
            admin,
=======
    fun init_module(sender: &signer) {
        aptos_framework::managed_coin::initialize<MoonCoin>(
            sender,
>>>>>>> 48a6a25e (implement move fct call in rust. Raw version)
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
<<<<<<< HEAD
        aptos_framework::managed_coin::register<MoonCoin>(admin);
    }

=======
    }

    //to test that everything works
    public entry fun test() { //account: &signer
    }
>>>>>>> 48a6a25e (implement move fct call in rust. Raw version)
}

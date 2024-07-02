module moveth::moveth_tests {
    use aptos_framework::account;
    use aptos_framework::aptos_account;
    use std::signer;
    use aptos_framework::fungible_asset;
    use moveth::moveth;

    #[test]
    fun test_init_module() {
        let root_address = 0xA550C18;
        let root_signer = signer::address_of(root_address);
        moveth::init_for_test(&root_signer);
        assert!(moveth::moveth_address() != account::zero_address(), 100);
    }

    #[test]
    fun test_mint() {
        let root_address = 0xA550C18;
        let root_signer = signer::address_of(root_address);
        moveth::init_for_test(&root_signer);

        let receiver_address = 0x1;
        let receiver_store = moveth::primary_store(receiver_address, moveth::metadata());
        let balance = moveth::balance(receiver_store);
        assert!(balance == 0, 101);

        moveth::mint(&root_signer, receiver_address, 100);

        let balance = moveth::balance(receiver_store);
        assert!(balance == 100, 102);
    }

    #[test]
    fun test_pause() {
        let root_address = 0xA550C18;
        let root_signer = signer::address_of(root_address);
        moveth::init_for_test(&root_signer);

        let pauser_address = 0x2;
        let pauser_signer = signer::address_of(pauser_address);
        moveth::set_pause(&pauser_signer, true);

        let state = borrow_global<moveth::State>(moveth::moveth_address());
        assert!(state.paused, 102);

        moveth::set_pause(&pauser_signer, false);
        let state_unpaused = borrow_global<moveth::State>(moveth::moveth_address());
        assert!(!state_unpaused.paused, 103);
    }

    #[test]
    fun test_denylist() {
        let root_address = 0xA550C18;
        let root_signer = signer::address_to_signer(root_address);
        moveth::init_for_test(&root_signer);

        let denylister_address = 0x3;
        let denylister_signer = signer::address_of(denylister_address);
        let account_to_denylist = 0x4;
        moveth::denylist(&denylister_signer, account_to_denylist);

        let freeze_ref = &borrow_global<moveth::Management>(moveth::moveth_address()).transfer_ref;
        let is_frozen: bool = fungible_asset::is_frozen(freeze_ref, account_to_denylist);
        assert!(is_frozen, 104);

        moveth::undenylist(&denylister_signer, account_to_denylist);
        let is_unfrozen: bool = !fungible_asset::is_frozen(freeze_ref, account_to_denylist);
        assert!(is_unfrozen, 105);
    }
}

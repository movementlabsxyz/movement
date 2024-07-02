module moveth::moveth_resource_account {
    use aptos_framework::account;
    use aptos_framework::signer;
    use aptos_framework::resource_account;

    struct ResourceAccount has key {
        admin: address,
        signer_cap: account::SignerCapability,
    }

    public entry fun create(admin: &signer, seed: vector<u8>) {
        let admin_addr = signer::address_of(admin);
        let (resource_account_address, resource_signer_cap) = account::create_resource_account(admin, seed);
        move_to(&resource_account_address, ResourceAccount {
            admin: admin_addr,
            signer_cap: resource_signer_cap,
        });
    }

    public fun is_admin(admin: &signer): bool acquires ResourceAccount {
        let resource_account = borrow_global<ResourceAccount>(signer::address_of(admin));
        signer::address_of(admin) == resource_account.admin
    }

    public fun assert_is_admin(admin: &signer) acquires ResourceAccount {
        assert!(is_admin(admin), 403);
    }

}
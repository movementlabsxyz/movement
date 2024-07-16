/// Basic faucet, allows to request coins between intervals.
module mock::faucet {
    use std::signer;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin};

    const ERR_FAUCET_EXISTS: u64 = 100;
    const ERR_FAUCET_NOT_EXISTS: u64 = 101;
    const ERR_RESTRICTED: u64 = 102;

    struct Faucet<phantom CoinType> has key {
        deposit: Coin<CoinType>,
        per_request: u64,
        period: u64,
    }

    struct Restricted<phantom Faucet> has key {
        since: u64,
    }
    
    public fun create_faucet_internal<CoinType>(account: &signer, deposit: Coin<CoinType>, per_request: u64, period: u64) {
        let account_addr = signer::address_of(account);

        assert!(!exists<Faucet<CoinType>>(account_addr), ERR_FAUCET_EXISTS);

        move_to(account, Faucet<CoinType> {
            deposit,
            per_request,
            period
        });
    }

    public fun change_settings_internal<CoinType>(account: &signer, per_request: u64, period: u64) acquires Faucet {
        let account_addr = signer::address_of(account);

        assert!(exists<Faucet<CoinType>>(account_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(account_addr);
        faucet.per_request = per_request;
        faucet.period = period;
    }

    /// Deposist more coins `CoinType` to faucet.
    public fun deposit_internal<CoinType>(faucet_addr: address, deposit: Coin<CoinType>) acquires Faucet {
        assert!(exists<Faucet<CoinType>>(faucet_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(faucet_addr);
        coin::merge(&mut faucet.deposit, deposit);
    }

    public fun request_internal<CoinType>(account: &signer, faucet_addr: address): Coin<CoinType> acquires Faucet, Restricted {
        let account_addr = signer::address_of(account);

        assert!(exists<Faucet<CoinType>>(faucet_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(faucet_addr);
        let coins = coin::extract(&mut faucet.deposit, faucet.per_request);

        let now = timestamp::now_seconds();

        if (exists<Restricted<CoinType>>(account_addr)) {
            let restricted = borrow_global_mut<Restricted<CoinType>>(account_addr);
            assert!(restricted.since + faucet.period <= now, ERR_RESTRICTED);
            restricted.since = now;
        } else {
            move_to(account, Restricted<CoinType> {
                since: now,
            });
        };

        coins
    }

    public entry fun create_faucet<CoinType>(account: &signer, amount_to_deposit: u64, per_request: u64, period: u64) {
        let coins = coin::withdraw<CoinType>(account, amount_to_deposit);

        create_faucet_internal(account, coins, per_request, period);
    }

    public entry fun change_settings<CoinType>(account: &signer, per_request: u64, period: u64) acquires Faucet {
        change_settings_internal<CoinType>(account, per_request, period);
    }

    public entry fun deposit<CoinType>(account: &signer, amount: u64) acquires Faucet {
        let coins = coin::withdraw<CoinType>(account, amount);

        deposit_internal<CoinType>(@mock, coins);
    }

    /// "Mints" coins of `CoinType` to `account` address.
    public entry fun mint<CoinType>(account: &signer) acquires Faucet, Restricted {
        let account_addr = signer::address_of(account);

        if (!coin::is_account_registered<CoinType>(account_addr)) {
            coin::register<CoinType>(account);
        };

        let coins = request_internal<CoinType>(account, @mock);

        coin::deposit(account_addr, coins);
    }
}
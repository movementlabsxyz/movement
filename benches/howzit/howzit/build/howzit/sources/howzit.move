module howzit::howzit {
  use std::signer;

  struct Counter has key {
    count: u64
  }

  // Initialization function for the ResourceRoulette contract
  fun init_module(account: &signer) {

    move_to(account, Counter {
        count: 0
    });

  }

    // Simply updates the count in the Counter struct
    public entry fun probe_1(account: &signer) acquires Counter {
        let counter = borrow_global_mut<Counter>(signer::address_of(account));
        counter.count = counter.count + 1;
    }

    // Medium loop over the Counter struct
    public entry fun probe_2(account: &signer) acquires Counter {
        let counter = borrow_global_mut<Counter>(signer::address_of(account));
        let i = 0;
        while (i < 100) {
            counter.count = counter.count + 1;
            i = i + 1;
        }
    }

    // Big loop over the Counter struct
    public entry fun probe_3(account: &signer) acquires Counter {
        let counter = borrow_global_mut<Counter>(signer::address_of(account));
        let i = 0;
        while (i < 10000) {
            counter.count = counter.count + 1;
            i = i + 1;
        }
    }

}
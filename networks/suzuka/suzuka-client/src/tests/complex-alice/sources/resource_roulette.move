module resource_roulette::resource_roulette {
  use std::vector;
  use std::signer;

  const ENO_UNAUTHORIZED_ADDRESS : u64 = 0;

  // ResourceRoulette struct representing the contract state
  struct ResourceRoulette has key {
    bids: vector<vector<address>>,
    owner: address,
    state : u64,
    total_bid : u64
  }

  struct RouletteWinnings has key {
    amount : u64
  }

  // Initialization function for the ResourceRoulette contract
  fun init_module(account: &signer) {

    assert!(signer::address_of(account) == @resource_roulette, ENO_UNAUTHORIZED_ADDRESS);

    let bids = vector::empty<vector<address>>();
    let i = 0;
    while (i < 32) {
      vector::push_back(&mut bids, vector::empty<address>());
      i = i + 1;
    };

    move_to(account, ResourceRoulette {
      bids,
      owner: @resource_roulette,
      state : 17203943403948,
      total_bid : 0
    });

  }

  // Initializes winnings for a signer
  public fun init_module_winnings(account: &signer) {
    move_to(account, RouletteWinnings {
      amount: 0,
    });
  }

  // Bid function to allow signers to bid on a specific slot
  public entry fun bid(account : &signer, slot: u8) acquires ResourceRoulette {

    if (!exists<RouletteWinnings>(signer::address_of(account))) {
      init_module_winnings(account);
    };

    let self = borrow_global_mut<ResourceRoulette>(@resource_roulette);
    roll_state(self);
    let bids_size = vector::length(&self.bids);
    assert!(slot < (bids_size as u8), 99);

    let slot_bids = vector::borrow_mut(&mut self.bids, (slot as u64));
    vector::push_back(slot_bids, signer::address_of(account));
    self.total_bid = self.total_bid + 1;
  }
  
  #[view]
  public fun total_bids() : u64 acquires ResourceRoulette {
    // Make this more complex to support actual bidding
    borrow_global<ResourceRoulette>(@resource_roulette).total_bid
  }

  // rolls state using xoroshiro prng
  fun roll_state(self :&mut ResourceRoulette) {
    let state = (self.state as u256);
    let x = state;
    let y = state >> 64;

    let t = x ^ y;
    state = ((x << 55) | (x >> 9)) + y + t;

    y = y ^ x;
    state = state + ((y << 14) | (y >> 50)) + x + t;
    
    state = state + t;
    state = state % (2^128 - 1);
    self.state = (state as u64);

  }

  #[view]
  public fun get_noise() : u64 {
    1
  }


  fun empty_bids(self : &mut ResourceRoulette){
    // empty the slots
    let bids = vector::empty<vector<address>>();
    let i = 0;
    while (i < 32) {
      vector::push_back(&mut bids, vector::empty<address>());
      i = i + 1;
    };
    self.bids = bids;

  }

  // Roll function to select a pseudorandom slot and pay out all signers who selected that slot
  public entry fun spin() acquires ResourceRoulette, RouletteWinnings {
    // assert!(signer::address_of(account) == @resource_roulette, ENO_UNAUTHORIZED_ADDRESS);
    let self = borrow_global_mut<ResourceRoulette>(@resource_roulette);

    // get the winning slot
    let bids_size = vector::length(&self.bids);
    roll_state(self);
    let winning_slot = (get_noise() * self.state % (bids_size as u64)) ;

    // pay out the winners
    let winners = vector::borrow(&self.bids, winning_slot);
    let num_winners = vector::length(winners);

    if (num_winners > 0){
      let balance_per_winner = self.total_bid/( num_winners as u64);
      let i = 0;
      while (i < num_winners) {
        let winner = vector::borrow(winners, i);
        let winnings = borrow_global_mut<RouletteWinnings>(*winner);
        winnings.amount = winnings.amount + balance_per_winner;
        i = i + 1;
      };
    };

    empty_bids(self);

  }

  #[test(account = @resource_roulette)]
  public fun test_init_moduleializes(account : &signer) acquires ResourceRoulette {
    init_module(account);
    let self = borrow_global<ResourceRoulette>(@resource_roulette);
    let bids_size = vector::length(&self.bids);
    assert!(bids_size == 32, 99);
  }

  #[test(account = @0x1)]
  #[expected_failure(abort_code = ENO_UNAUTHORIZED_ADDRESS)]
  public fun test_init_moduleialization_fails(account : &signer) acquires ResourceRoulette {
    init_module(account);
    let self = borrow_global<ResourceRoulette>(@resource_roulette);
    let bids_size = vector::length(&self.bids);
    assert!(bids_size == 32, 99);
  }

  #[test(account = @resource_roulette, bidder_one = @0x3)]
  public fun test_bids_and_empties(account : &signer, bidder_one : &signer) acquires ResourceRoulette {
    
    init_module(account);
    bid(bidder_one, 10);
    empty_bids(borrow_global_mut<ResourceRoulette>(@resource_roulette));

    // bids get pulled off the table
    let i = 0;
    while (i < 32){
      let slot = vector::borrow(&borrow_global<ResourceRoulette>(@resource_roulette).bids, i);
      let slot_size = vector::length(slot);
      assert!(slot_size < 1, i);
      i = i + 1;
    }

  }

  #[test(account = @resource_roulette, bidder_one = @0x3)]
  public fun test_plays(account : &signer, bidder_one : &signer) acquires ResourceRoulette, RouletteWinnings {
    
    init_module(account);
    bid(bidder_one, 10);
    spin();

    // spin empties the bids
    let i = 0;
    while (i < 32){
      let slot = vector::borrow(&borrow_global<ResourceRoulette>(@resource_roulette).bids, i);
      let slot_size = vector::length(slot);
      assert!(slot_size < 1, i);
      i = i + 1;
    }

  }

  #[test(account = @resource_roulette)]
  public fun test_rolls_state(account : &signer) acquires ResourceRoulette {
    init_module(account);
    let self = borrow_global_mut<ResourceRoulette>(@resource_roulette);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
    let state = self.state;
    roll_state(self);
    assert!(state != self.state, 99);
  }

  #[test_only]
  const BOUNDARY_WINNER : u64 = 1;

  // Under the current state rolling implementation this will work
  // More robust testing would calculate system dynamics
  #[test(account = @resource_roulette, bidder_one = @0x3)]
  #[expected_failure(abort_code = BOUNDARY_WINNER)]
  public fun test_wins(account : &signer, bidder_one : &signer) acquires ResourceRoulette, RouletteWinnings {
    
    init_module(account);
    let i : u64 = 0;
    while (i < 1_000) {
      bid(bidder_one, 7);
      spin();

      let winnings = borrow_global<RouletteWinnings>(signer::address_of(bidder_one));
      if (winnings.amount > 0) {
        abort BOUNDARY_WINNER
      };

      i = i + 1;
    };

  }

  // Under the current state rolling implementation this will work
  // More robust testing would calculate system dynamics
  #[test(account = @resource_roulette, bidder_one = @0x3, bidder_two = @0x4, bidder_three = @0x5)]
  #[expected_failure(abort_code = BOUNDARY_WINNER)]
  public fun test_multi_wins(account : &signer, bidder_one : &signer, bidder_two : &signer, bidder_three : &signer) acquires ResourceRoulette, RouletteWinnings {
    
    init_module(account);
    let i : u64 = 0;

    while (i < 1_000) {
      bid(bidder_one, 2);
      bid(bidder_two, 2);
      bid(bidder_three, 4);
      spin();

      let winnings_one = borrow_global<RouletteWinnings>(signer::address_of(bidder_one));
      let winnings_two = borrow_global<RouletteWinnings>(signer::address_of(bidder_two));
      let winnings_three = borrow_global<RouletteWinnings>(signer::address_of(bidder_three));
      if (winnings_one.amount > 0 && winnings_two.amount > 0 && winnings_three.amount > 0) {
        abort BOUNDARY_WINNER
      };

      i = i + 1;

    };

  }

}
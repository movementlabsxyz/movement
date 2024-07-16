/// Module: mock_tokens
module mock_tokens::btc {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct WBTC has drop {}

  fun init(witness: BTC, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<BTC>(
            witness, 
            9, 
            b"BTC",
            b"Bitcoin", 
            b"The first cryptocurrency!", 
            option::some(new_unsafe_from_bytes(b"https://imagedelivery.net/cBNDGgkrsEA-b_ixIp9SkQ/btc.png/public")), 
            ctx
        );

      transfer::public_share_object(treasury_cap);
      transfer::public_freeze_object(metadata);
  }
}

module mock_tokens::usdc {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct USDC has drop {}

  fun init(witness: USDC, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<USDC>(
            witness, 
            6, 
            b"USDC",
            b"USD Coin", 
            b"USD Stable Coin by Circle", 
            option::some(new_unsafe_from_bytes(b"https://imagedelivery.net/cBNDGgkrsEA-b_ixIp9SkQ/usdc.png/public")), 
            ctx
        );

      transfer::public_share_object(treasury_cap);
      transfer::public_freeze_object(metadata);
  }
}

module mock_tokens::usdt {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct USDT has drop {}

  fun init(witness: USDT, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<USDT>(
            witness, 
            9, 
            b"USDT",
            b"USD Tether", 
            b"Stable coin", 
            option::some(new_unsafe_from_bytes(b"https://imagedelivery.net/cBNDGgkrsEA-b_ixIp9SkQ/usdt.png/public")), 
            ctx
        );

      transfer::public_share_object(treasury_cap);
      transfer::public_freeze_object(metadata);
  }
}

module mock_tokens::weth {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct WETH has drop {}

  fun init(witness: WETH, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<WETH>(
            witness, 
            9, 
            b"WETH",
            b"WETH", 
            b"Wrapped Ethereum", 
            option::some(new_unsafe_from_bytes(b"https://imagedelivery.net/cBNDGgkrsEA-b_ixIp9SkQ/eth.png/public")), 
            ctx
        );

      transfer::public_share_object(treasury_cap);
      transfer::public_freeze_object(metadata);
  }
}
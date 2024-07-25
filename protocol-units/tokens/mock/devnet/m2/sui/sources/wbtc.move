module mock_tokens::wbtc {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct WBTC has drop {}

  fun init(witness: WBTC, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<WBTC>(
            witness, 
            9, 
            b"WBTC",
            b"Bitcoin", 
            b"The first cryptocurrency!", 
            option::some(new_unsafe_from_bytes(b"https://imagedelivery.net/cBNDGgkrsEA-b_ixIp9SkQ/btc.png/public")), 
            ctx
        );

      transfer::public_share_object(treasury_cap);
      transfer::public_freeze_object(metadata);
  }
}
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
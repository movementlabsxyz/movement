module mock_tokens::usdt {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct USDT has drop {}

  fun init(witness: USDT, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<USDT>(
            witness, 
            6, 
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
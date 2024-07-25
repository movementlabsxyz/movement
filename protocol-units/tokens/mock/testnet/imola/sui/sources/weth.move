module mock_tokens::weth {
  use sui::coin;
  use sui::url::new_unsafe_from_bytes;

  public struct WETH has drop {}

  fun init(witness: WETH, ctx: &mut TxContext) {
      let (treasury_cap, metadata) = coin::create_currency<WETH>(
            witness, 
            8, 
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
# Atomic Bridge Move Modules 


## `moveth.move`
This module offers a reference implementation of a managed stablecoin with the following functionalities:
1. Upgradable smart contract. The module can be upgraded to update existing functionalities or add new ones.
2. Minting and burning of stablecoins. The module allows users to mint and burn stablecoins. Minter role is required to mint or burn
3. Denylisting of accounts. The module allows the owner to denylist (freeze) and undenylist accounts.
denylist accounts cannot transfer or get minted more.
4. Pausing and unpausing of the contract. The owner can pause the contract to stop all mint/burn/transfer and unpause it to resume.

# Running tests
aptos move test


Philippe Delrieu & Richard Melkonian

## Security Notes 
1st step. Initiator account has to sign the bridgeTransferId we get the hash of signature.

function gen_id()
let `bridgeTransferId` =
            keccak256(abi.encodePacked(originator, recipient, signaturOfhashLock, nonce));
Sign this ID with 

ETH Initiator 

1. let `key` = random anything.
2. let `ethSignatureOfKey` = Sign(key).

-> 
function initiateBridgeTransfer (uint256 wethAmount, bytes32 recipient, bytes32  `signatureOfKey`, uint256 timeLock) {
let bridgeTransferId =
            keccak256(abi.encodePacked(originator, recipient, signatureOfKey, eth_nonce));
return bridgeTransferId
}

// verifiy the sender has signed the bridgeTrasnferID
3. Offchain do
let to_verify = keccak256(bridgeTransferId + move_nonce);
let eth_signed_to_verify = Sign(to_verify) // With Eth signature;
let move_signed_to_verify = Sign(to_verify) // With Move signature;

# MoveCounterparty

4. call lock_bridge_assets(key, eth_signature_of_key, etheruem_public_address, recipient, timeLock, block.number, eth_nonce, eth_signed_to_verify, move_signed_to_verify) {

  1st Verification: that signature_of_key has been signed by etheruem_public_address
  build bridgeTransferId
            keccak256(abi.encodePacked(originator, recipient, signatureOfKey, timeLock, block.number, eth_nonce));

  2nd Verification;
  // rebuild to_verify
  let to_verify = keccak256(bridgeTransferId + move_nonce); //move_nonce from the current call / contract state

  Verify that `eth_signed_to_verify` is the signature of `to_verify` by `etheruem_public_address`, 
  Verify that `move_signed_to_verify` is the signature of `to_verify` by `move_sender`, 
  
}

Therefor we are sure this is the same entity / owner of the initiator account because the key. On the move part he is able to sign the `key` to generate the `signature_of_key`.
- We prove that we know the key, we verify that the signature on ethereum is signed by the same account on move. And we add this signature to the `bridgeTransferId`. We prove that signature was signed
On the eth Initiator because its hashed inside the `bridgeTransferId`.

- And after we prove that we are able to sign both the `key` and the `nonce` which ties the nonce to this specific swap. 




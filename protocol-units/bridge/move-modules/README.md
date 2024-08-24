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

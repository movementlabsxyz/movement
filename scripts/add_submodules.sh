#!/bin/bash

# Run this script to initialize and update all submodules.

# Settlement MCR Submodules
git submodule add --force https://github.com/foundry-rs/forge-std protocol-units/settlement/mcr/contracts/lib/forge-std
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-contracts protocol-units/settlement/mcr/contracts/lib/openzeppelin-contracts
git submodule add --force https://github.com/dmfxyz/murky protocol-units/settlement/mcr/contracts/lib/murky
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-foundry-upgrades protocol-units/settlement/mcr/contracts/lib/openzeppelin-foundry-upgrades
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-contracts-upgradeable protocol-units/settlement/mcr/contracts/lib/openzeppelin-contracts-upgradeable
git submodule add --force https://github.com/safe-global/safe-smart-account protocol-units/settlement/mcr/contracts/lib/safe-smart-account

# Bridge Submodules
git submodule add --force https://github.com/foundry-rs/forge-std protocol-units/bridge/contracts/lib/forge-std
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-contracts protocol-units/bridge/contracts/lib/openzeppelin-contracts
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-contracts-upgradeable protocol-units/bridge/contracts/lib/openzeppelin-contracts-upgradeable

# Dispute Submodules
git submodule add --force https://github.com/foundry-rs/forge-std protocol-units/dispute/lib/forge-std
git submodule add --force https://github.com/OpenZeppelin/openzeppelin-contracts protocol-units/dispute/lib/openzeppelin-contracts
git submodule add --force https://github.com/dmfxyz/murky protocol-units/dispute/lib/murky

# Initialize and update all submodules
echo "Initializing and updating submodules..."
git submodule init
git submodule update --remote --recursive

echo "All submodules have been added and updated!"


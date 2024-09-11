// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract MOVEToken is ERC20Upgradeable, AccessControlUpgradeable {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    /**
     * @dev Initialize the contract
     */
    function initialize(address multisig) public initializer {
        __ERC20_init("Movement", "MOVE");
        _mint(multisig, 10000000000 * 10 ** decimals());  // Initial mint to multisig
        _grantRole(MINTER_ROLE, multisig);  // Grant minter role to multisig
    }

    /**
     * @dev Mint new tokens
     */
    function mint(address to, uint256 amount) public onlyRole(MINTER_ROLE) {
        _mint(to, amount);
    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}
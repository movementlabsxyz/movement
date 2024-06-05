// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../base/BaseToken.sol";

contract MintableToken is BaseToken {

    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant MINTER_ADMIN_ROLE = keccak256("MINTER_ADMIN_ROLE");

    /**
     * @dev Initialize the contract
     * @param initialOwner The address to set as the owner
     */
    function initialize(
        string memory name,
        string memory symbol
    ) initializer public {
        super.initialize(name, symbol);
        _setupRole(MINTER_ADMIN_ROLE, msg.sender);
        _setupRole(MINTER_ROLE, msg.sender);
    }
    
    /**
     * @dev Set minter role
     * @param account The address to set minter role
     */
    function setMinterRole(address account) external onlyRole(MINTER_ADMIN_ROLE) {
        grantRole(MINTER_ROLE, account);
    }

    /**
     * @dev Revoke minter role
     * @param account The address to revoke minter role from
     */
    function revokeMinterRole(address account) external onlyRole(MINTER_ADMIN_ROLE) {
        revokeRole(MINTER_ROLE, account);
    }

    /**
     * @dev Mint new tokens
     * @param to The address to mint tokens to
     * @param amount The amount of tokens to mint
     */
    function publicMint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _mint(to, amount);
    }

}
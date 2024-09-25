// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {BaseToken} from "./BaseToken.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import "forge-std/console.sol";

interface IMintableToken is IERC20 {
    function mint(address to, uint256 amount) external;
    function grantMinterRole(address account) external;
    function revokeMinterRole(address account) external;
}

contract MintableToken is IMintableToken, BaseToken {
    using SafeERC20 for IERC20;

    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant MINTER_ADMIN_ROLE = keccak256("MINTER_ADMIN_ROLE");

    /**
     * @dev Initialize the contract
     * @param name The name of the token
     * @param symbol The symbol of the token
     */
    function initialize(
        string memory name,
        string memory symbol
    ) public virtual override initializer {
        __MintableToken_init(name, symbol);
    }

    function __MintableToken_init(
        string memory name,
        string memory symbol
    ) internal onlyInitializing {
        __ERC20_init_unchained(name, symbol);
        __BaseToken_init_unchained();
        __MintableToken_init_unchained();
    }

    function __MintableToken_init_unchained() internal onlyInitializing {
        _grantRole(MINTER_ADMIN_ROLE, msg.sender);
        _grantRole(MINTER_ROLE, msg.sender);
    }

    /**
     * @dev Set minter role
     * @param account The address to set minter role
     */
    function grantMinterRole(
        address account
    ) public onlyRole(MINTER_ADMIN_ROLE) {
        _grantRole(MINTER_ROLE, account);
    }

    /**
     * @dev Check if an account has minter role
     * @param account The address to check
     * @return True if the account has minter role
     */
    function hasMinterRole(
        address account
    ) public view returns (bool) {
        return hasRole(MINTER_ROLE, account);
    }

    /**
     * @dev Revoke minter admin role
     * @param account The address to revoke minter admin role from
     */
    function revokeMinterAdminRole(
        address account
    ) public onlyRole(MINTER_ADMIN_ROLE) {
        _revokeRole(MINTER_ADMIN_ROLE, account);
    }

    /**
     * @dev Revoke minter role
     * @param account The address to revoke minter role from
     */
    function revokeMinterRole(
        address account
    ) public onlyRole(MINTER_ADMIN_ROLE) {
        _revokeRole(MINTER_ROLE, account);
    }

    /**
     * @dev Mint new tokens
     * @param to The address to mint tokens to
     * @param amount The amount of tokens to mint
     */
    function mint(
        address to,
        uint256 amount
    ) public virtual onlyRole(MINTER_ROLE) {
        _mint(to, amount);
    }
}

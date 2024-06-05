// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../governance/MintableToken.sol";

contract CustodianToken is MintableToken {
    using SafeMathUpgradeable for uint256;
    using SafeERC20Upgradeable for IERC20Upgradeable;

    bytes32 public constant TRANSFER_ROLE = keccak256("TRANSFER_ROLE");
    bytes32 public constant TRANSFER_ADMIN_ROLE = keccak256("TRANSFER_ADMIN_ROLE");

    IERC20 public underlyingToken;

    function initialize(
        string memory name, 
        string memory symbol, 
        IERC20 _underlyingToken
    ) public initializer {
        super.initialize(name, symbol);
        _setupRole(TRANSFER_ADMIN_ROLE, msg.sender);
        _setupRole(TRANSFER_ROLE, msg.sender);
        underlyingToken = _underlyingToken;
    }

    /**
     * @dev Transfer tokens
     * @param from The address to transfer tokens from
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     */
    function _transfer(address from, address to, uint256 amount) internal override {
        // Transfers are restricted to be to or from TRANSFER_ROLE accounts.
        // This allows moving into contracts such as the staking contract, but not out.
        require(
            hasRole(TRANSFER_ROLE, from) || hasRole(TRANSFER_ROLE, to),
            "Transfer restricted to accounts with TRANSFER_ROLE"
        );

        // perform a normal transfer
        super._transfer(from, to, amount);

        // also perform a safe transfer from this contract to the recipient
        underlyingToken.safeTransfer(to, amount);
    }

    /**
     * @dev Approve tokens
     * @param spender The address to approve tokens for
     * @param amount The amount of tokens to approve
     * @return A boolean indicating whether the approval was successful
     */
    function approve(address spender, uint256 amount) public override returns (bool) {
        require(hasRole(TRANSFER_ROLE, msg.sender), "Approval restricted to accounts with TRANSFER_ROLE");
        return underlyingToken.approve(spender, amount);
    }

    /** 
     * @dev Transfer tokens from
     * @param sender The address to transfer tokens from
     * @param recipient The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transferFrom(address sender, address recipient, uint256 amount) public override returns (bool) {
        _transfer(from, to, amount);
        return true;
    }

    /**
     * @dev Transfer tokens
     * @param recipient The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transfer(address recipient, uint256 amount) public override returns (bool) {
        return _transfer(this, to, amount);
        return true;
    }

    /** 
    * @dev Mint new tokens
    * @param account The address to mint tokens to
    * @param amount The amount of tokens to mint
    */
    function _mint(address account, uint256 amount) internal override {
        super._mint(account, amount);
        underlyingToken.mint(this, amount);
    }

}

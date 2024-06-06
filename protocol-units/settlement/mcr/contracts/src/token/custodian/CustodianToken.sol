// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../base/MintableToken.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import "../base/WrappedToken.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

interface ICustodianToken is IERC20 {
    function grantTransferSinkRole(address account) external;
    function revokeTransferSinkRole(address account) external;

    function grantBuyerRole(address account) external;
    function revokeBuyerRole(address account) external;

    function buyCustodialTokenFor(address account, uint256 amount) external;
}

contract CustodianToken is ICustodianToken, WrappedToken {
    using SafeERC20 for IERC20;

    bytes32 public constant TRANSFER_SINK_ROLE = keccak256("TRANSFER_SINK_ROLE");
    bytes32 public constant TRANSFER_SINK_ADMIN_ROLE = keccak256("TRANSFER_SINK_ADMIN_ROLE");

    bytes32 public constant BUYER_ROLE = keccak256("BUYER_ROLE");
    bytes32 public constant BUYER_ADMIN_ROLE = keccak256("BUYER_ADMIN_ROLE");

    /**
     * @dev Initialize the contract
     * @param name The name of the token
     * @param symbol The symbol of the token
     * @param _underlyingToken The underlying token to wrap
     */
    function initialize(string memory name, string memory symbol, IMintableToken _underlyingToken)
        public
        virtual
        override
        initializer
    {
        __CustodianToken_init(name, symbol, _underlyingToken);
    }

    function __CustodianToken_init(string memory name, string memory symbol, IMintableToken _underlyingToken)
        internal
        onlyInitializing
    {
        __ERC20_init_unchained(name, symbol);
        __AccessControl_init_unchained();
        __UUPSUpgradeable_init_unchained();
        __BaseToken_init_unchained();
        __MintableToken_init_unchained();
        __WrappedToken_init_unchained(_underlyingToken);
        __CustodianToken_init_unchained();
    }

    function __CustodianToken_init_unchained() internal onlyInitializing {
        _grantRole(TRANSFER_SINK_ADMIN_ROLE, msg.sender);
        _grantRole(TRANSFER_SINK_ROLE, msg.sender);
        _grantRole(BUYER_ADMIN_ROLE, msg.sender);
        _grantRole(BUYER_ROLE, msg.sender);
    }

    function grantTransferSinkRole(address account) public onlyRole(TRANSFER_SINK_ADMIN_ROLE) {
        _grantRole(TRANSFER_SINK_ROLE, account);
    }

    function revokeTransferSinkRole(address account) public onlyRole(TRANSFER_SINK_ADMIN_ROLE) {
        _revokeRole(TRANSFER_SINK_ROLE, account);
    }

    /**
     * @dev Approve tokens
     * @param spender The address to approve tokens for
     * @param amount The amount of tokens to approve
     * @return A boolean indicating whether the approval was successful
     */
    function approve(address spender, uint256 amount) public override(IERC20, ERC20Upgradeable) returns (bool) {
        // require the spender is a transfer sink
        require(hasRole(TRANSFER_SINK_ROLE, spender), "Approval restricted to accounts with TRANSFER_SINK_ROLE");

        return underlyingToken.approve(spender, amount);
    }

    /**
     * @dev Transfer tokens from
     * @param from The address to transfer tokens from
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transferFrom(address from, address to, uint256 amount)
        public
        virtual
        override(IERC20, ERC20Upgradeable)
        returns (bool)
    {
        // require the destination is a transfer sink
        require(hasRole(TRANSFER_SINK_ROLE, to), "Transfer restricted to accounts with TRANSFER_SINK_ROLE");

        // burn the tokens from the sender
        super.transferFrom(from, address(this), amount);

        // also perform a safe transfer from this contract to the recipient
        return underlyingToken.transfer(to, amount);
    }

    /**
     * @dev Transfer tokens
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return A boolean indicating whether the transfer was successful
     */
    function transfer(address to, uint256 amount) public override(IERC20, ERC20Upgradeable) returns (bool) {
        // require the destination is a transfer sink
        require(hasRole(TRANSFER_SINK_ROLE, to), "Transfer restricted to accounts with TRANSFER_SINK_ROLE");

        // burn the tokens from the sender
        super.transfer(address(this), amount);

        // also perform a safe transfer from this contract to the recipient
        return underlyingToken.transfer(to, amount);
    }

    function grantBuyerRole(address account) public onlyRole(BUYER_ADMIN_ROLE) {
        _grantRole(BUYER_ROLE, account);
    }

    function revokeBuyerRole(address account) public onlyRole(BUYER_ADMIN_ROLE) {
        _revokeRole(BUYER_ROLE, account);
    }

    function buyCustodialTokenFor(address account, uint256 amount) public override {
        require(hasRole(BUYER_ROLE, msg.sender), "Caller must have BUYER_ROLE");

        // transfer the approved value from the buyer to this contract
        underlyingToken.transferFrom(msg.sender, address(this), amount);

        // mint the custodial token for the buyer at their desired address
        // ! maybe this should also be managed through the minter role, so the buyer would have to be buyer and minter
        super._mint(account, amount);
    }
}

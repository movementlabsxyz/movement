// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./BaseToken.sol";

contract WrappedToken is BaseToken {

    IERC20Upgradeable public underlyingToken;

    /**
     * @dev Initialize the contract
     * @param initialOwner The address to set as the owner
     */
    function initialize(
        string memory name, 
        string memory symbol, 
        IERC20Upgradeable _underlyingToken
    ) initializer public {
        super.initialize(name, symbol);
        underlyingToken = _underlyingToken;
    }

}
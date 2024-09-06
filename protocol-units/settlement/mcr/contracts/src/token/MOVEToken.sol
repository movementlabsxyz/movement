// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVEToken is MintableToken {
    /**
     * @dev Initialize the contract
     */
    function initialize() public initializer {
        __MintableToken_init("Move Token", "MOVE");
        _mint(address(msg.sender), 10000000000 * 10 ** decimals());
    }
}

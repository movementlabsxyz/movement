// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVEToken is MintableToken {

    /**
     * @dev Initialize the contract
     */
    function initialize() public {
        super.initialize("Move Token", "MOVE");
    }

}
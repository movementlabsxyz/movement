// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./governance/MintableToken.sol";

contract MOVEToken is MintableToken {

    /**
     * @dev Initialize the contract
     */
    function initialize() public initializer {
        super.initialize("Move Token", "MOVE");
    }

}
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {MintableToken} from "./base/MintableToken.sol";

contract MOVEToken is MintableToken {
    /**
     * @dev Initialize the contract
     */
    function initialize() public initializer {
        __MintableToken_init("Move Token", "MOVE");
    }
}

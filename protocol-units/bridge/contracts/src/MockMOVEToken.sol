// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract MockMOVEToken is ERC20Upgradeable {

    /**
     * @dev Initialize the contract
     */
    function initialize(address multisig) public initializer {
        __ERC20_init("Movement", "MOVE");
        _mint(multisig, 10000000000 * 10 ** decimals()); 
    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}
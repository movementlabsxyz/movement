// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./BaseToken.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20".sol";
import "openzeppelin-contracts-upgradeable/contracts/token/ERC20/ERC20Upgradeable.sol";

contract WrappedToken is BaseToken {

    using SafeERC20 for IERC20;

    /**
     * @dev Initialize the contract
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
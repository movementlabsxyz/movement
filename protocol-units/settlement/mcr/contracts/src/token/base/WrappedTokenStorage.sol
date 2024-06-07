// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import "./MintableToken.sol";

contract WrappedTokenStorage {
    using SafeERC20 for IERC20;

    IMintableToken public underlyingToken;

    uint256[50] internal __gap;
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../base/BaseToken.sol";
import "../base/MintableToken.sol";
import "../base/WrappedToken.sol";
import "../custodian/CustodianToken.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import { Math } from "@openzeppelin/contracts/utils/math/Math.sol";

contract LockedTokenStorage is CustodianToken {

    struct Lock {
        uint256 amount;
        uint256 releaseTime;
    }

    mapping(address => Lock[]) public locks;

    uint256[50] internal __gap;
    
}

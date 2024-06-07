// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./locked/LockedToken.sol";
import "./base/MintableToken.sol";
import "./custodian/CustodianToken.sol";
import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";

contract stlkMOVEToken is LockedToken {

    using SafeERC20 for IERC20;

    /**
    * @dev Initialize the contract
    * @param underlyingToken The underlying token to wrap
     */
    function initialize(
        IMintableToken underlyingToken
    ) public {
    
        super.initialize("Stakable Locked Move Token", "stlkMOVE", underlyingToken);

    }

}
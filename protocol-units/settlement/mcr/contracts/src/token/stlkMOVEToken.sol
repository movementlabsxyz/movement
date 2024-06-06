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

// Flow for staking
// StakingContract: signer call stake
// StakingContract: signer approves StakingContract to spend their stlkMOVE tokens. 
// StakingContract: calls transferFrom on stlkMOVE to move both stlkMOVE and MOVE tokens to the staking contract
// StakingContract: staking contract confirms it received the tokens and records balance for the signer with the custodian

// Flow for unstaking
// StakingContract: signer calls unstake with the custodian
// StakingContract: staking contract transfers stlkMOVE and MOVE tokens back to the custodian via calling transfer on the stlkMOVE contract
// StakingContract: staking contract confirms it transferred the tokens back to the custodian and updates the signer's balance to 0
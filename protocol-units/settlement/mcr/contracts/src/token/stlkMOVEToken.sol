// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./locked/LockedToken.sol";
import "./custodian/CustodianToken.sol";

contract stlkMOVE is LockedToken, CustodianToken {

    /**
     * @dev Initialize the contract
     * @param initialOwner The address to set as the owner
     */
    function initialize(
        IERC20Upgradeable underlyingToken
    ) initializer public {
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
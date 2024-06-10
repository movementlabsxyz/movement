// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {LockedToken} from "./locked/LockedToken.sol";
import {CustodianToken} from "./custodian/CustodianToken.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import {IMintableToken} from "./base/MintableToken.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract stlMoveToken is LockedToken, CustodianToken {
    using SafeERC20 for IERC20;

    /**
     * @dev Initialize the contract
     * @param _underlyingToken The underlying token to wrap
     */
    function initialize(IMintableToken _underlyingToken) public {
        initialize("Stakable Locked Move Token", "stlMOVE", _underlyingToken);
    }

    function initialize(string memory name, string memory symbol, IMintableToken _underlyingToken)
        public
        override(CustodianToken, LockedToken)
        initializer
    {
        __ERC20_init_unchained(name, symbol);
        __BaseToken_init_unchained();
        __MintableToken_init_unchained();
        __WrappedToken_init_unchained(_underlyingToken);
        __LockedToken_init_unchained();
        __CustodianToken_init_unchained();
    }

    function transfer(address to, uint256 amount)
        public
        override(CustodianToken, ERC20Upgradeable, IERC20)
        returns (bool)
    {
        return CustodianToken.transfer(to, amount);
    }

    function transferFrom(address from, address to, uint256 amount)
        public
        override(CustodianToken, ERC20Upgradeable, IERC20)
        returns (bool)
    {
        return CustodianToken.transferFrom(from, to, amount);
    }

    function approve(address spender, uint256 amount)
        public
        override(CustodianToken, ERC20Upgradeable, IERC20)
        returns (bool)
    {
        return CustodianToken.approve(spender, amount);
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

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { SafeERC20 } from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/interfaces/IERC20.sol";
import "./MintableToken.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract WrappedToken is MintableToken {

    using SafeERC20 for IERC20;

    IMintableToken public underlyingToken;

    /**
     * @dev Initialize the contract
     */
    function initialize(
        string memory name, 
        string memory symbol, 
        IMintableToken _underlyingToken
    ) public virtual {
        super.initialize(name, symbol);
        underlyingToken = _underlyingToken;
    }

    /** 
    * @dev Mint new tokens
    * @param account The address to mint tokens to
    * @param amount The amount of tokens to mint
    */
    function mint(address account, uint256 amount) public override virtual {
        super.mint(account, amount);
        underlyingToken.mint(address(this), amount);
    }

}
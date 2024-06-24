// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import "./MintableToken.sol";
import "./WrappedTokenStorage.sol";
import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract WrappedToken is WrappedTokenStorage, MintableToken {
    using SafeERC20 for IERC20;

    /**
     * @dev Initialize the contract
     * @param name The name of the token
     * @param symbol The symbol of the token
     * @param _underlyingToken The underlying token to wrap
     */
    function initialize(
        string memory name,
        string memory symbol,
        IMintableToken _underlyingToken
    ) public virtual initializer {
        __WrappedToken_init(name, symbol, _underlyingToken);
    }

    /**
     * @dev Initialize the contract
     * @param _underlyingToken The underlying token to wrap
     */
    function __WrappedToken_init(
        string memory name,
        string memory symbol,
        IMintableToken _underlyingToken
    ) internal onlyInitializing {
        __ERC20_init_unchained(name, symbol);
        __BaseToken_init_unchained();
        __MintableToken_init_unchained();
        __WrappedToken_init_unchained(_underlyingToken);
    }

    /**
     * @dev Initialize the contract unchained avoiding reinitialization
     * @param _underlyingToken The underlying token to wrap
     */
    function __WrappedToken_init_unchained(
        IMintableToken _underlyingToken
    ) internal onlyInitializing {
        underlyingToken = _underlyingToken;
    }

    /**
     * @dev Mint new tokens
     * @param account The address to mint tokens to
     * @param amount The amount of tokens to mint
     */
    function mint(address account, uint256 amount) public virtual override {
        super.mint(account, amount);
        underlyingToken.mint(address(this), amount);
    }
}

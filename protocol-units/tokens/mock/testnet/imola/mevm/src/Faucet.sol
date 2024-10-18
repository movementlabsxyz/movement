// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import {MockToken} from "./MockToken.sol";

contract Faucet {
    MockToken public usdt;
    MockToken public usdc;
    MockToken public wbtc;
    MockToken public weth;
    address public owner;

    constructor() public {
        owner = msg.sender;
    }

    function mint() public {
        uint256 usdtValue = usdt.faucetMint();
        uint256 usdcValue = usdc.faucetMint();
        uint256 wbtcValue = wbtc.faucetMint();
        uint256 wethValue = weth.faucetMint();
        usdt.transfer(msg.sender, usdtValue);
        usdc.transfer(msg.sender, usdcValue);
        wbtc.transfer(msg.sender, wbtcValue);
        weth.transfer(msg.sender, wethValue);
    }

    function setFaucetTokens(MockToken _usdt, MockToken _usdc, MockToken _wbtc, MockToken _weth) public {
        require(msg.sender == owner, "Only owner can set tokens");
        usdt = _usdt;
        usdc = _usdc;
        wbtc = _wbtc;
        weth = _weth;
    }
}

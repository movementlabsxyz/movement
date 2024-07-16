// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MockToken} from "../src/MockToken.sol";
import {WETH10} from "../src/WETH10.sol";

contract DeployScript is Script {
    function setUp() public {}

    function run() public {
        vm.broadcast();

        uint256 dexs = 10;

        MockToken usdc = new MockToken("Circle", "USDC", 6, 1000000 * dexs, 60000, 3600);
        MockToken usdt = new MockToken("Tether", "USDT", 6, 1000000 * dexs, 60000, 3600);
        MockToken wbtc = new MockToken("Bitcoin", "WBTC", 8, 17 * dexs, 1, 3600);
        MockToken weth = new MockToken("Ethereum", "WETH", 8, 340 * dexs, 20, 3600);
    }
}

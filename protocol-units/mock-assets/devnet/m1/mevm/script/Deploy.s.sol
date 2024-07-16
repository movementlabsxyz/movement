// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MockToken} from "../src/MockToken.sol";
import {WETH10} from "../src/WETH10.sol";

contract DeployScript is Script {
    function setUp() public {}

    function run() public {
        vm.broadcast();

        MockToken usdc = new MockToken("Circle", "USDC", 6, 60000000000000, 3600);
        MockToken usdt = new MockToken("Tether", "USDT", 6, 60000000000000, 3600);
        MockToken wbtc = new MockToken("Bitcoin", "WBTC", 8, 100000000, 3600);
        MockToken weth = new MockToken("Ethereum", "WETH", 8, 2000000000, 3600);
    }
}

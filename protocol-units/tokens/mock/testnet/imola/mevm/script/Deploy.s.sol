// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {MockToken} from "../src/MockToken.sol";
import {WETH10} from "../src/WETH10.sol";
import {Faucet} from "../src/Faucet.sol";
import "forge-std/console.sol";

contract DeployScript is Script {
    Faucet public faucet;
    MockToken public usdc;
    MockToken public usdt;
    MockToken public wbtc;
    MockToken public weth;
    WETH10 public wmove;

    function run() public {
        vm.startBroadcast(vm.envUint("PRIVATE_KEY"));

        uint256 dexs = 5;

        faucet = new Faucet();
        usdc = new MockToken("Circle", "USDC", 6, 1000000 * dexs, 60000, 3600, address(faucet));
        usdt = new MockToken("Tether", "USDT", 6, 1000000 * dexs, 60000, 3600, address(faucet));
        wbtc = new MockToken("Bitcoin", "WBTC", 8, 17 * dexs, 1, 3600, address(faucet));
        weth = new MockToken("Ethereum", "WETH", 8, 340 * dexs, 20, 3600, address(faucet));
        wmove = new WETH10();

        faucet.setFaucetTokens(usdt, usdc, wbtc, weth);

        console.log("Faucet:", address(faucet));
        console.log("USDC:", address(usdc));
        console.log("USDT:", address(usdt));
        console.log("WBTC", address(wbtc));
        console.log("WETH", address(weth));
        console.log("WMOVE", address(wmove));

        vm.stopBroadcast();
    }
}

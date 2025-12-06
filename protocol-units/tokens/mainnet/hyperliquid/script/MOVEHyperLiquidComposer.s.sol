// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "../src/MOVEHyperLiquidComposer.sol";
import "forge-std/Script.sol";

contract MOVEHyperLiquidComposerDeployer is Script {
    MOVEHyperLiquidComposer public hyperLiquidComposer;

    event Deployed(address indexed contractAddress);

    function run() public {
        vm.startBroadcast();
        hyperLiquidComposer = new MOVEHyperLiquidComposer(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073,395,0);
        vm.stopBroadcast();
    }
}
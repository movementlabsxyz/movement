// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/Settlement.sol";

contract DeploySettlement is Script {
    function run() external {
        vm.startBroadcast();

        new Settlement();

        vm.stopBroadcast();
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/Settlement.sol";
import "../src/ControlID.sol";

contract DeploySettlement is Script {
    function run() external {
        vm.startBroadcast();

        new Settlement(ControlID.CONTROL_ID_0, ControlID.CONTROL_ID_1);

        vm.stopBroadcast();
    }
}

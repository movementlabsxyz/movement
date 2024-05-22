pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/MCR.sol";

contract DeployMCR is Script {
    function run() external {
        vm.startBroadcast();

        new MCR(
            10, 
            128,
            100 ether, // should accumulate 100 ether
            0
        );

        vm.stopBroadcast();
    }
}
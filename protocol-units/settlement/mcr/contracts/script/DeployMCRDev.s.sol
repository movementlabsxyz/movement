pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/token/MOVEToken.sol";
import "../src/staking/MovementStaking.sol";
import "../src/settlement/MCR.sol";

contract DeployMCR is Script {
    function run() external {
        vm.startBroadcast();

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        vm.stopBroadcast();
    }
}

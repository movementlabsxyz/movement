pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/MCR.sol";

contract DeployMCR is Script {
    function run() external {
        vm.startBroadcast();

        MCR mcr = new MCR(
            5, 
            128,
            100 ether, // should accumulate 100 ether
            100 ether, // each genesis validator can stake up to 100 ether
            0
        );
        vm.stopBroadcast();

        // Comment because the Genesis ceremony works (Assert ok)
        // But in Rust Genesis is not done.
        // address payable signer1 = payable(vm.addr(1)); 
        // vm.deal(signer1, 100 ether);
        // address payable signer2 = payable(vm.addr(2));
        // vm.deal(signer2, 100 ether);
        // address payable signer3 = payable(vm.addr(3));
        // vm.deal(signer3, 100 ether);

        // // have them participate in the genesis ceremony
        // vm.prank(signer1);
        // mcr.stakeGenesis{value : 34 ether}();
        // vm.prank(signer2);
        // mcr.stakeGenesis{value : 33 ether}();
        // vm.prank(signer3);
        // mcr.stakeGenesis{value : 33 ether}();
        // assert(mcr.hasGenesisCeremonyEnded() == true);

    }
}
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

contract DeployMoveToken is Script {
    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    string public moveSignature = "initialize()";

    function run() external {
        vm.startBroadcast(vm.envUint("PRIVATE_KEY"));
        MOVEToken moveImplementation = new MOVEToken();
        admin = new ProxyAdmin(vm.addr(1));
        moveProxy = new TransparentUpgradeableProxy(
            address(moveImplementation), address(admin), abi.encodeWithSignature(moveSignature)
        );
        // since admin proxy owns both move and stlmove, we only need to transfer ownership of admin to timelock
        MOVEToken moveToken = MOVEToken(address(moveProxy));
        moveToken.transfer(0x47A9561eFaa534add7Ce95904690CD6bBd7cCb8f, 20 * 10**18);
        
        console.log("Move Token deployed at: ", address(moveProxy));
        console.log("implementation: ", address(moveImplementation));
        vm.stopBroadcast();
    }
}

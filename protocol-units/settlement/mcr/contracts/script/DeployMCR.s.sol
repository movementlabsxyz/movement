pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "../src/MCR.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/TransparentUpgradeableProxy.sol";
import {TimeLockController} from "@openzeppelin/contracts/access/TimeLockController.sol";

contract DeployMCR is Script {

    TransparentUpgradeableProxy public mcrProxy;
    ProxyAdmin public admin;
    TimeLock public timeLock;
    string public signature = "initialize(uint256,uint256,uint256,uint256,uint256)";


    function run() external {
        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = address(keccak256("Andy"));
        proposers[1] = address(keccak256("Bob"));
        proposers[2] = address(keccak256("Charlie"));
        proposers[3] = address(keccak256("David"));
        proposers[4] = address(keccak256("Eve"));

        executors[0] = address(keccak256("MultisigAddress"));

        address adminAddress = 0x0;

        vm.startBroadcast();
        
        MCR mcrImplementation = new MCR();

        admin = new ProxyAdmin();
        mcrProxy = new TransparentUpgradeableProxy(address(mcrImplementation), address(admin), abi.encodeWithSignature(signature,
            5, 
            128,
            100 ether, // should accumulate 100 ether
            100 ether, // each genesis validator can stake up to 100 ether
            0
        ));

        timeLock = new TimelockController(minDelay, proposers, executors, adminAddress);
        admin.transferOwnership(address(timeLock));

        MCR mcrImplementation2 = new MCR();
        vm.stopBroadcast();
        // deploy a new implementation of MCR and schedule an upgrade
        vm.startBroadcast(vm.envUint("ANDY_PRIVATE_KEY"););
        address to = address(mcrProxy);
        uint256 value = 0; // not sure
        bytes payload = abi.encodeWithSignature("upgradeTo(address)", address(mcrImplementation2));
        bytes32 predecessor = ""; // not sure
        bytes32 salt = ""; // not sure
        uint256 delay = 1 days + 1;

        timelock.schedule(to, value, payload, predecessor, salt, delay);
        vm.stopBroadcast();

        // multisig would be able to execute the upgrade after the delay
    }
}
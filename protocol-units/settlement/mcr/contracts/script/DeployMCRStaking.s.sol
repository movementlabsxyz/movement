pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MCR} from "../src/settlement/MCR.sol";
import {MovementStaking} from "../src/staking/MovementStaking.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";


function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

contract DeployMCRStaking is Script {
    TransparentUpgradeableProxy public mcrProxy;
    TransparentUpgradeableProxy public stakingProxy;
    ProxyAdmin public admin;
    TimelockController public timelock;
    string public signature = "initialize(address,uint256,uint256,uint256,address[])";
    string public stakingSignature = "initialize(address)";

    function run() external {
        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = string2Address("Andy");
        proposers[1] = string2Address("Bob");
        proposers[2] = string2Address("Charlie");
        proposers[3] = string2Address("David");
        proposers[4] = string2Address("Eve");

        executors[0] = string2Address("MultisigAddress");

        address adminAddress = address(0);

        vm.startBroadcast(vm.addr(1));

        MovementStaking stakingImplementation = new MovementStaking();
        MCR mcrImplementation = new MCR();

        admin = new ProxyAdmin(vm.addr(1));

        stakingProxy = new TransparentUpgradeableProxy(
            address(stakingImplementation),
            address(admin),
            abi.encodeWithSignature(stakingSignature, string2Address("MoveMockAddress"))
        );

        mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(admin),
            abi.encodeWithSignature(
                signature,
                address(stakingProxy),
                128,
                100 ether, // should accumulate 100 ether
                100 ether, // each genesis validator can stake up to 100 ether
                proposers // using this as a mock array of addresses
            )
        );

        timelock = new TimelockController(minDelay, proposers, executors, adminAddress);
        admin.transferOwnership(address(timelock));

        MCR mcrImplementation2 = new MCR();
        vm.stopBroadcast();
        // deploy a new implementation of MCR and schedule an upgrade
        // vm.startBroadcast(vm.envUint("ANDY_PRIVATE_KEY"));
        vm.startBroadcast(string2Address("Andy"));
        address to = address(mcrProxy);
        uint256 value = 0; // not sure
        bytes memory payload = abi.encodeWithSignature("upgradeTo(address)", address(mcrImplementation2));
        bytes32 predecessor = bytes32(0); // not sure
        bytes32 salt = bytes32(0); // not sure
        uint256 delay = 1 days + 1;

        timelock.schedule(to, value, payload, predecessor, salt, delay);
        vm.stopBroadcast();

        // multisig would be able to execute the upgrade after the delay
        // time.lock.execute(to, value, payload, predecessor, salt);
        // gnosis safe has a UI for this
    }
}

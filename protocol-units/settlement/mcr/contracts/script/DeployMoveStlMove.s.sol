pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MintableToken} from "../src/toke/base/MintableToken.sol";
import {stlMoveToken, IMintableToken} from "../src/token/stlMoveToken.sol";
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

contract DeployMintableToken is Script {

    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    TimelockController public timelock;
    string public moveSignature = "initialize(string,string)";
    string public stlSignature = "initialize(IMintableToken)";


    function run() external {
        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = address(string2Address("Andy"));
        proposers[1] = address(string2Address("Bob"));
        proposers[2] = address(string2Address("Charlie"));
        proposers[3] = address(string2Address("David"));
        proposers[4] = address(string2Address("Eve"));

        executors[0] = address(string2Address("MultisigAddress"));

        address adminAddress = address(0);

        vm.startBroadcast();
        
        MintableToken moveImplementation = new MintableToken();
        stlMoveToken stlMoveImplementation = new stlMoveToken();

        admin = new ProxyAdmin(address(this));
        moveProxy = new TransparentUpgradeableProxy(address(moveImplementation), address(admin), abi.encodeWithSignature(signature,
            "Move Token",
            "MOVE"
        ));
        stlMoveProxy = new TransparentUpgradeableProxy(address(stlMoveImplementation), address(admin), abi.encodeWithSignature(signature,
            IMintableToken(moveProxy)
        ));

        timelock = new TimelockController(minDelay, proposers, executors, adminAddress);
        // since admin proxy owns both move and stlmove, we only need to transfer ownership of admin to timelock
        admin.transferOwnership(address(timelock));

        MintableToken moveImplementation2 = new MintableToken();
        vm.stopBroadcast();
        // deploy a new implementation of MintableToken and schedule an upgrade
        vm.startBroadcast(vm.envUint("ANDY_PRIVATE_KEY"));
        address to = address(moveProxy);
        uint256 value = 0; // not sure
        bytes memory payload = abi.encodeWithSignature("upgradeTo(address)", address(moveImplementation2));
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

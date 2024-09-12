pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {Vm} from "forge-std/Vm.sol";

function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

interface create {
    function deploy(bytes32 _salt, bytes memory _bytecode) external returns (address);
}

contract DeployMoveToken is Script {
    TransparentUpgradeableProxy public moveProxy;
    string public moveSignature = "initialize(address)";
    string public safeSetupSignature = "setup(address[],uint256,address,bytes,address,address,uint256,address)";
    SafeProxyFactory public safeProxyFactory;
    Safe public safeSingleton;
    Safe public safe;
    TimelockController public timelock;
    address create3address = address(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);
    address moveAdmin;
    bytes32 public salt = 0xc000000000000000000000002774b8b4881d594b03ff8a93f4cad69407c90350;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // forge script DeployMoveToken --fork-url https://eth-sepolia.api.onfinality.io/public
        safe = Safe(payable(address(0x00db70A9e12537495C359581b7b3Bc3a69379A00)));

        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = string2Address("Andy");
        proposers[1] = string2Address("Bob");
        proposers[2] = string2Address("Charlie");
        proposers[3] = string2Address("David");
        proposers[4] = string2Address("Eve");

        executors[0] = address(safe);

        timelock = new TimelockController(minDelay, proposers, executors, address(0x0));

        
        _deployMove();
        

        console.log("Safe balance: ", MOVEToken(address(moveProxy)).balanceOf(address(safe)));
        console.log("Move Token decimals: ", MOVEToken(address(moveProxy)).decimals());
        console.log("Move Token supply: ", MOVEToken(address(moveProxy)).totalSupply());
        console.log("Timelock deployed at: ", address(timelock));
        console.log("Safe deployed at: ", address(safe));
        vm.stopBroadcast();
    }

    function _deployMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        // moveProxy = new TransparentUpgradeableProxy(
        //     address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(safe))
        // );
        bytes memory bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, address(safe)))
        );
        vm.recordLogs();
        moveProxy = TransparentUpgradeableProxy(payable(create(create3address).deploy(salt, bytecode)));
        Vm.Log[] memory logs = vm.getRecordedLogs();
        console.log("MOVE deployment records:");
        console.log("proxy", address(moveProxy));
        console.log("implementation", address(moveImplementation));
        // deployment.move = address(moveProxy);
        // deployment.moveAdmin = _storeAdminDeployment();
        
        moveAdmin = logs[logs.length - 2].emitter;
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        timelock.schedule(
            address(moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(moveProxy),
                address(newMoveImplementation),
                abi.encodeWithSignature("initialize(address)", address(safe))
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );
    }
}

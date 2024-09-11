pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stlMoveToken} from "../src/token/stlMoveToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";
import { Vm } from "forge-std/Vm.sol";

contract StlMoveDeployer is Helper {

    function run() external virtual {
        
         // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // timelock is required for all deployments
        if (deployment.timelock == ZERO) {
            timelock = new TimelockController(config.minDelay, config.proposers, config.executors, config.admin);
            deployment.timelock = address(timelock);
        }

        deployment.stlMoveAdmin == ZERO && deployment.stlMove == ZERO && deployment.move != ZERO ?
            _deployStlMove() : deployment.stlMoveAdmin != ZERO && deployment.stlMove != ZERO ?
                _upgradeStlMove() : revert("STL: both admin and proxy should be registered");

        vm.stopBroadcast();

        // Only write to file if chainid is not running a foundry local chain
        if (block.chainid != foundryChainId) {
            _writeDeployments();
        }
    }

    function _deployStlMove() internal {
        console.log("STL: deploying");
        stlMoveToken stlMoveImplementation = new stlMoveToken();
        vm.recordLogs();
        stlMoveProxy = new TransparentUpgradeableProxy(
            address(stlMoveImplementation),
            address(timelock),
            abi.encodeWithSignature(stlMoveSignature, "STL Move Token", "STL", address(moveProxy))
        );
        console.log("STL deployment records:");
        console.log("proxy", address(stlMoveProxy));
        deployment.stlMove = address(stlMoveProxy);
        deployment.stlMoveAdmin = _storeAdminDeployment();
    }

    function _upgradeStlMove() internal {
        console.log("STL: upgrading");
        stlMoveToken newStlMoveImplementation = new stlMoveToken();
        timelock.schedule(
            address(deployment.stlMoveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(stlMoveProxy),
                address(newStlMoveImplementation),
                ""
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );
        console.log("STL: upgrade scheduled");
    }

}

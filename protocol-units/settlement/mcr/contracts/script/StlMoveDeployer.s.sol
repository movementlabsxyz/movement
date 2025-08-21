pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stlMoveToken} from "../src/token/stlMoveToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";

contract StlMoveDeployer is Helper {

    function run() external virtual {
        
         // load config and deployments data
        _loadExternalData();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // Deploy CREATE3Factory, Safes and Timelock if not deployed
        _deployDependencies();

        deployment.stlMoveAdmin == ZERO && deployment.stlMove == ZERO && deployment.move != ZERO ?
            _deployStlMove() : deployment.stlMoveAdmin != ZERO && deployment.stlMove != ZERO ?
                _upgradeStlMove() : revert("STL: both admin and proxy should be registered");

        vm.stopBroadcast();

        // Only write to file if chainid is not running a foundry local chain
        if (vm.isContext(VmSafe.ForgeContext.ScriptBroadcast)) {
                _writeDeployments();
            }
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•
// Modifications to the following functions have to be throughly tested

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
        _checkBytecodeDifference(address(newStlMoveImplementation), deployment.stlMove);
        // Prepare the data for the upgrade
        bytes memory data = abi.encodeWithSignature(
            "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
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
            config.minDelay
        );
        
        _proposeUpgrade(data, "stlmove.json");
    }
}

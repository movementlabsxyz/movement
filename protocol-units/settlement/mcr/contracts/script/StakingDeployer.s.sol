pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MovementStaking} from "../src/staking/MovementStaking.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";
import { Vm } from "forge-std/Vm.sol";
import {stdJson} from "forge-std/StdJson.sol";

contract StakingDeployer is Helper {
        using stdJson for string;

    function run() external virtual {
        
         // load config and deployments data
        _loadExternalData();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // Deploy CREATE3Factory, Safes and Timelock if not deployed
        _deployDependencies();

        deployment.stakingAdmin == ZERO && deployment.staking == ZERO && deployment.move != ZERO ?
            _deployStaking() : deployment.stakingAdmin != ZERO && deployment.staking != ZERO ?
                _upgradeStaking() : revert("STAKING: both admin and proxy should be registered");

        vm.stopBroadcast();

        // Only write to file if chainid is not running a foundry local chain
        if (vm.isContext(VmSafe.ForgeContext.ScriptBroadcast)) {
                _writeDeployments();
            }
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•

    function _deployStaking() internal {
        console.log("STAKING: deploying");
        MovementStaking stakingImplementation = new MovementStaking();
        vm.recordLogs();
        stakingProxy = new TransparentUpgradeableProxy(
            address(stakingImplementation),
            address(timelock),
            abi.encodeWithSignature(stakingSignature, address(moveProxy))
        );
        console.log("STAKING deployment records:");
        console.log("proxy", address(stakingProxy));
        deployment.staking = address(stakingProxy);
        deployment.stakingAdmin = _storeAdminDeployment();
    }

    function _upgradeStaking() internal {
    console.log("STAKING: upgrading");
    MovementStaking newStakingImplementation = new MovementStaking();
    string memory json = "safeCall";

    // Prepare the data for the upgrade
    bytes memory data = abi.encodeWithSignature(
        "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
        address(deployment.stakingAdmin),
        0,
        abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(stakingProxy),
            address(newStakingImplementation),
            ""
        ),
        bytes32(0),
        bytes32(0),
        config.minDelay
    );
    
    // Serialize the relevant fields into JSON format
    json.serialize("to", address(timelock));
    string memory zero = "0";
    json.serialize("value", zero);
    json.serialize("data", data);
    string memory operation = "OperationType.Call";
    json.serialize("chainId", chainId);
    json.serialize("safeAddress", deployment.movementLabsSafe);
    string memory serializedData = json.serialize("operation", operation);

    // Log the serialized JSON for debugging
    console.log("STAKING upgrade json |start|", serializedData, "|end|");

    // Write the serialized data to a file
    vm.writeFile(string.concat(root, upgradePath, "staking.json"), serializedData);
}


}

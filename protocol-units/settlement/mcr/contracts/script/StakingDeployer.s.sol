pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MovementStaking} from "../src/staking/MovementStaking.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";
import { Vm } from "forge-std/Vm.sol";

contract StakingDeployer is Helper {

    function run() external virtual {
        
         // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // timelock is required for all deployments
        _deployTimelock();

        deployment.stakingAdmin == NULL && deployment.staking == NULL && deployment.move != ZERO ?
            _deployStaking() : deployment.stakingAdmin != ZERO && deployment.staking != ZERO ?
                _upgradeStaking() : revert("STAKING: both admin and proxy should be registered");

        vm.stopBroadcast();

        // Only write to file if chainid is not running a foundry local chain
        if (block.chainid != foundryChainId) {
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
        timelock.schedule(
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
            block.timestamp + 1 days
        );
        console.log("STAKING: upgrade scheduled");
    }

}

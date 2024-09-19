pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import { Helper } from "./helpers/Helper.sol";
import { MCRDeployer } from "./MCRDeployer.s.sol";
import { StakingDeployer } from "./StakingDeployer.s.sol";
import { StlMoveDeployer } from "./StlMoveDeployer.s.sol";
import { MOVETokenDeployer } from "./MOVETokenDeployer.s.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Vm, VmSafe } from "forge-std/Vm.sol";

contract CoreDeployer is MCRDeployer, StakingDeployer, StlMoveDeployer, MOVETokenDeployer {

    function run() external override(MCRDeployer, StakingDeployer, StlMoveDeployer, MOVETokenDeployer) {
        // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // Deploy CREATE3Factory if not deployed
        _deployCreate3();

        // Deploy Safes if not deployed
        _deploySafes();

        // timelock is required for all deployments
        _deployTimelock();
        
        // Deploy or upgrade contracts conditionally
        deployment.moveAdmin == NULL && deployment.move == NULL ?
            _deployMove() : deployment.moveAdmin != NULL && deployment.move != NULL ?
                // if move is already deployed, upgrade it
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");

        // requires move to be deployed
        deployment.stakingAdmin == NULL && deployment.staking == NULL && deployment.move != NULL ?
            _deployStaking() : deployment.stakingAdmin != NULL && deployment.staking != NULL ?
                // if staking is already deployed, upgrade it
                _upgradeStaking() : revert("STAKING: both admin and proxy should be registered");

        // requires move to be deployed
        deployment.stlMoveAdmin == NULL && deployment.stlMove == NULL && deployment.move != NULL ?
            _deployStlMove() : deployment.stlMoveAdmin != NULL && deployment.stlMove != NULL ?
                // if stlMove is already deployed, upgrade it
                _upgradeStlMove() : revert("STL: both admin and proxy should be registered");

        // requires staking and move to be deployed
        deployment.mcrAdmin == NULL && deployment.mcr == NULL && deployment.move != NULL && deployment.staking != NULL ?
            _deployMCR() : deployment.mcrAdmin != NULL && deployment.mcr != NULL ?
                // if mcr is already deployed, upgrade it
                _upgradeMCR() : revert("MCR: both admin and proxy should be registered");

        // Only write to file if chainid is not running a foundry local chain and if broadcasting
        if (block.chainid == foundryChainId) {
            _upgradeMove();
            _upgradeStaking();
            _upgradeStlMove();
            _upgradeMCR();
        } else {
            if (vm.isContext(VmSafe.ForgeContext.ScriptBroadcast)) {
                _writeDeployments();
            }
        }

        vm.stopBroadcast();
    }
}

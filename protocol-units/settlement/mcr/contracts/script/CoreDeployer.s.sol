pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import { Helper } from "./helpers/Helper.sol";
import { MCRDeployer } from "./MCRDeployer.s.sol";
import { StakingDeployer } from "./StakingDeployer.s.sol";
import { StlMoveDeployer } from "./StlMoveDeployer.s.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";

contract CoreDeployer is MCRDeployer, StakingDeployer, StlMoveDeployer {

    function run() external override(MCRDeployer, StakingDeployer, StlMoveDeployer) {
        // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // timelock is required for all deployments
        _deployTimelock();
        
        // Deploy or upgrade contracts conditionally
        deployment.moveAdmin == ZERO && deployment.move == ZERO ?
            _deployMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                // if move is already deployed, upgrade it
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");

        // requires move to be deployed
        deployment.stakingAdmin == ZERO && deployment.staking == ZERO && deployment.move != ZERO ?
            _deployStaking() : deployment.stakingAdmin != ZERO && deployment.staking != ZERO ?
                // if staking is already deployed, upgrade it
                _upgradeStaking() : revert("STAKING: both admin and proxy should be registered");

        // requires move to be deployed
        deployment.stlMoveAdmin == ZERO && deployment.stlMove == ZERO && deployment.move != ZERO ?
            _deployStlMove() : deployment.stlMoveAdmin != ZERO && deployment.stlMove != ZERO ?
                // if stlMove is already deployed, upgrade it
                _upgradeStlMove() : revert("STL: both admin and proxy should be registered");

        // requires staking and move to be deployed
        deployment.mcrAdmin == ZERO && deployment.mcr == ZERO && deployment.move != ZERO && deployment.staking != ZERO ?
            _deployMCR() : deployment.mcrAdmin != ZERO && deployment.mcr != ZERO ?
                // if mcr is already deployed, upgrade it
                _upgradeMCR() : revert("MCR: both admin and proxy should be registered");

        // Only write to file if chainid is not running a foundry local chain
        if (block.chainid == foundryChainId) {
            _upgradeMove();
            _upgradeStaking();
            _upgradeStlMove();
            _upgradeMCR();
        } else {
            _writeDeployments();
        }

        vm.stopBroadcast();
    }
    
    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•

    function _deployMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        vm.recordLogs();
        moveProxy = new TransparentUpgradeableProxy(
            address(moveImplementation),
            address(timelock),
            abi.encodeWithSignature(moveSignature)
        );
        console.log("MOVE deployment records:");
        console.log("proxy", address(moveProxy));
        deployment.move = address(moveProxy);
        deployment.moveAdmin = _storeAdminDeployment();
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        timelock.schedule(
            address(deployment.moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(moveProxy),
                address(newMoveImplementation),
                ""
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );
        console.log("MOVE: upgrade scheduled");
    }

}

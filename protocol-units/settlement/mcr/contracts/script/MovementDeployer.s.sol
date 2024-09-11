pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {stlMoveToken} from "../src/token/stlMoveToken.sol";
import {MovementStaking} from "../src/staking/MovementStaking.sol";
import {MCR} from "../src/settlement/MCR.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";
import { Vm } from "forge-std/Vm.sol";
import { stdJson } from "forge-std/StdJson.sol";

contract MovementDeployer is Script {

    using stdJson for string;

    TransparentUpgradeableProxy public mcrProxy;
    TransparentUpgradeableProxy public stakingProxy;
    TransparentUpgradeableProxy public stlMoveProxy;
    TransparentUpgradeableProxy public moveProxy;
    ProxyAdmin public admin;
    TimelockController public timelock;
    Deployment public deployment;
    Helper helper = new Helper();
    bool public record = true;

    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[])";
    string public stakingSignature = "initialize(address)";
    string public stlMoveSignature = "initialize(string,string,address)";
    string public moveSignature = "initialize()";
    address public ZERO = address(0x0);

    struct Deployment {
        address mcr;
        address mcrAdmin;
        address staking;
        address stakingAdmin;
        address move;
        address moveAdmin;
        address stlMove;
        address stlMoveAdmin;
        address timelock;
        address multisig;
    }

    function run() external {
        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);
        proposers[0] = helper.s2a("Andy");
        proposers[1] = helper.s2a("Bob");
        proposers[2] = helper.s2a("Charlie");
        proposers[3] = helper.s2a("David");
        proposers[4] = helper.s2a("Eve");
        executors[0] = helper.s2a("MultisigAddress");
        address adminAddress = ZERO;

        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        if (deployment.timelock == ZERO) {
            timelock = new TimelockController(minDelay, proposers, executors, adminAddress);
            deployment.timelock = address(timelock);
        }
        
        deployment.moveAdmin == ZERO && deployment.move == ZERO ?
            _deployMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");

        deployment.stakingAdmin == ZERO && deployment.staking == ZERO && deployment.move != ZERO ?
            _deployStaking() : deployment.stakingAdmin != ZERO && deployment.staking != ZERO ?
                _upgradeStaking() : revert("STAKING: both admin and proxy should be registered");

        deployment.stlMoveAdmin == ZERO && deployment.stlMove == ZERO && deployment.move != ZERO ?
            _deployStlMove() : deployment.stlMoveAdmin != ZERO && deployment.stlMove != ZERO ?
                _upgradeStlMove() : revert("STL: both admin and proxy should be registered");

        deployment.mcrAdmin == ZERO && deployment.mcr == ZERO && deployment.move != ZERO && deployment.staking != ZERO ?
            _deployMCR(proposers) : deployment.mcrAdmin != ZERO && deployment.mcr != ZERO ?
                _upgradeMCR() : revert("MCR: both admin and proxy should be registered");

        vm.stopBroadcast();

        if (record) {
            _writeDeployments();
        }
    }

    function _loadDeployments() internal{
        // load deployments
        string memory root = vm.projectRoot();
        string memory path = string.concat(root, "/script/helpers/deployments.json");
        string memory json = vm.readFile(path);
        bytes memory rawDeploymentData = json.parseRaw(string(abi.encodePacked(".", "mainnet")));
        deployment = abi.decode(rawDeploymentData, (Deployment));
    }

    function _writeDeployments() internal {
        string memory root = vm.projectRoot();
        string memory path = string.concat(root, "/script/helpers/test.json");
        string memory json = "json";
        string memory a = json.serialize("move", deployment.move);
        string memory b = json.serialize("moveAdmin", deployment.moveAdmin);
        string memory c = json.serialize("staking", deployment.staking);
        string memory d = json.serialize("stakingAdmin", deployment.stakingAdmin);
        string memory e = json.serialize("stlMove", deployment.stlMove);
        string memory f = json.serialize("stlMoveAdmin", deployment.stlMoveAdmin);
        string memory g = json.serialize("mcr", deployment.mcr);
        string memory h = json.serialize("mcrAdmin", deployment.mcrAdmin);
        string memory i = json.serialize("timelock", deployment.timelock);
        string memory j = json.serialize("multisig", deployment.multisig);
        vm.writeFile(path, j);
    }

    function _storeAdminDeployment() internal returns (address admin) {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        admin = logs[logs.length-2].emitter;
        console.log("admin", admin);
    }   

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

    function _deployMCR(address[] memory proposers) internal {
        console.log("MCR: deploying");
        MCR mcrImplementation = new MCR();
        vm.recordLogs();
        mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(timelock),
            abi.encodeWithSignature(
                mcrSignature,
                address(stakingProxy),
                128,
                100 ether,
                100 ether, 
                proposers
            )
        );
        console.log("MCR deployment records:");
        console.log("proxy", address(mcrProxy));
        deployment.mcr = address(mcrProxy);
        deployment.mcrAdmin = _storeAdminDeployment();
    }

    function _upgradeMCR() internal {
        console.log("MCR: upgrading");
        MCR newMCRImplementation = new MCR();
        timelock.schedule(
            address(deployment.mcrAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(mcrProxy),
                address(newMCRImplementation),
                ""
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );
        console.log("MCR: upgrade scheduled");
    }

}

pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MCR} from "../src/settlement/MCR.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Helper } from "./helpers/Helper.sol";
import { Vm } from "forge-std/Vm.sol";
import { stdJson } from "forge-std/StdJson.sol";

contract MCRDeployer is Helper {

    using stdJson for string;

    function run() external virtual {
        
        // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // timelock is required for all deployments
        _deployTimelock();

        deployment.mcrAdmin == ZERO && deployment.mcr == ZERO && deployment.move != ZERO && deployment.staking != ZERO ?
            _deployMCR() : deployment.mcrAdmin != ZERO && deployment.mcr != ZERO ?
                _upgradeMCR() : revert("MCR: both admin and proxy should be registered");

        vm.stopBroadcast();

        // Only write to file if chainid is not running a foundry local chain
        if (block.chainid != foundryChainId) {
            _writeDeployments();
        }
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•

    function _deployMCR() internal {
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
                config.proposers
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
        string memory json = "safeCall";
        bytes memory data = abi.encodeWithSignature(
            "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
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
            config.minDelay
        );
        json.serialize("to", address(timelock));
        string memory zero = "0";
        json.serialize("value", zero);
        json.serialize("data", data);
        string memory operation = "OperationType.Call";
        json.serialize("chainId", chainId);
        json.serialize("safeAddress", config.proposers[0]);
        string memory serializedData = json.serialize("operation", operation);

        console.log("MCR upgrade json |start|", serializedData, "|end|");
        vm.writeFile(string.concat(root, upgradePath, "mcr.json"), serializedData);
    }

}

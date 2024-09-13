// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import { Vm } from "forge-std/Vm.sol";

contract Helper is Script {
    using stdJson for string;

    TransparentUpgradeableProxy public moveProxy;
    TransparentUpgradeableProxy public stlMoveProxy;
    TransparentUpgradeableProxy public stakingProxy;
    TransparentUpgradeableProxy public mcrProxy;
    TimelockController public timelock;
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[])";
    string public stakingSignature = "initialize(address)";
    string public stlMoveSignature = "initialize(string,string,address)";
    string public moveSignature = "initialize()";
    string public root = vm.projectRoot();
    string public deploymentsPath = "/script/helpers/deployments.json";
    string public upgradePath = "/script/helpers/upgrade/";
    address public ZERO = address(0x0);
    string public chainId = uint2str(block.chainid);
    uint256 public foundryChainId = 31337;
    string public storageJson;

    Deployment public deployment;
    ConfigData public config;

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

    struct ConfigData {
        uint256 minDelay;
        address[] signers;
        address admin;
        uint256 threshold;
        address[] proposers;
        address[] executors;
    }

    function _loadConfig() internal returns (uint256 minDelay, address[] memory signers, address[] memory proposers, address[] memory executors, address adminAddress) {
        string memory path = string.concat(root, "/script/helpers/config.json");
        string memory json = vm.readFile(path);
        bytes memory rawConfigData = json.parseRaw(string(abi.encodePacked(".")));
        config = abi.decode(rawConfigData, (ConfigData));
        if (config.proposers[0] == ZERO) {
            config.proposers[0] = vm.addr(vm.envUint("PRIVATE_KEY"));
        }
    }

    function _loadDeployments() internal {
        // load deployments
        // Inspo https://github.com/traderjoe-xyz/joe-v2/blob/main/script/deploy-core.s.sol
        string memory path = string.concat(root, deploymentsPath);
        string memory json = vm.readFile(path);
        bytes memory rawDeploymentData = json.parseRaw(string(abi.encodePacked(".", chainId)));
        deployment = abi.decode(rawDeploymentData, (Deployment));
        storageJson = json;
    }

    function _deployTimelock() internal {
        if (deployment.timelock == ZERO) {
            timelock = new TimelockController(config.minDelay, config.proposers, config.executors, config.admin);
            deployment.timelock = address(timelock);
        }
    }

    function _storeAdminDeployment() internal returns (address admin) {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        admin = logs[logs.length-2].emitter;
        console.log("admin", admin);
    }  

    function _writeDeployments() internal {
        string memory path = string.concat(root, deploymentsPath);
        string memory json = storageJson;
        string memory base = "new";
        string memory newChainData = _serializer(json, deployment);
        // take values from storageJson that were not updated (e.g. 3771) and serialize them
        uint256[] memory validChains = new uint256[](4);
        validChains[0] = 1; // ethereum
        validChains[1] = 11155111; // sepolia
        validChains[2] = 17000; // holesky
        validChains[3] = 31337; // foundry
        for (uint256 i = 0; i < validChains.length; i++) {
            if (validChains[i] != block.chainid) {
                _serializeChainData(base, storageJson, validChains[i]);
            }
        }
        // new chain data
        string memory data = base.serialize(chainId, newChainData);
        vm.writeFile(path, data);
    }

    function _serializeChainData(string memory base, string storage sJson, uint256 chain) internal {
        bytes memory rawDeploymentData = sJson.parseRaw(string(abi.encodePacked(".", uint2str(chain))));
        Deployment memory deploymentData = abi.decode(rawDeploymentData, (Deployment));
        string memory json = uint2str(chain);
        string memory chainData = _serializer(json, deploymentData);
        base.serialize(uint2str(chain), chainData);
    }

    function _serializer(string memory json, Deployment memory deployment) internal returns (string memory) {
        json.serialize("move", deployment.move);
        json.serialize("moveAdmin", deployment.moveAdmin);
        json.serialize("staking", deployment.staking);
        json.serialize("stakingAdmin", deployment.stakingAdmin);
        json.serialize("stlMove", deployment.stlMove);
        json.serialize("stlMoveAdmin", deployment.stlMoveAdmin);
        json.serialize("mcr", deployment.mcr);
        json.serialize("mcrAdmin", deployment.mcrAdmin);
        json.serialize("timelock", deployment.timelock);
        return json.serialize("multisig", deployment.multisig);
    }

    // string to address
    function s2a(bytes memory str) public returns (address addr) {
        bytes32 data = keccak256(str);  
        assembly {  
            addr := data  
        }
    }

    function uint2str(uint256 _i) internal pure returns (string memory _uintAsString) {
        if (_i == 0) {
            return "0";
        }
        uint256 j = _i;
        uint256 len;
        while (j != 0) {
            len++;
            j /= 10;
        }
        bytes memory bstr = new bytes(len);
        uint256 k = len;
        while (_i != 0) {
            k = k - 1;
            uint8 temp = (48 + uint8(_i - _i / 10 * 10));
            bytes1 b1 = bytes1(temp);
            bstr[k] = b1;
            _i /= 10;
        }
        return string(bstr);
    }
}

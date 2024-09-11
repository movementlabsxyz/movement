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
        string memory path = string.concat(vm.projectRoot(), "/script/helpers/config.json");
        string memory json = vm.readFile(path);
        bytes memory rawConfigData = json.parseRaw(string(abi.encodePacked(".")));
        config = abi.decode(rawConfigData, (ConfigData));
    }

    function _loadDeployments() internal {
        // load deployments
        string memory path = string.concat(vm.projectRoot(), "/script/helpers/deployments.json");
        string memory json = vm.readFile(path);
        bytes memory rawDeploymentData = json.parseRaw(string(abi.encodePacked(".", chainId)));
        deployment = abi.decode(rawDeploymentData, (Deployment));
        storageJson = json;
    }

    function _storeAdminDeployment() internal returns (address admin) {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        admin = logs[logs.length-2].emitter;
        console.log("admin", admin);
    }  

    function _writeDeployments() internal {
        string memory root = vm.projectRoot();
        string memory path = string.concat(root, "/script/helpers/test.json");
        string memory json = storageJson;
        string memory base = "new";
        json.serialize("move", deployment.move);
        json.serialize("moveAdmin", deployment.moveAdmin);
        json.serialize("staking", deployment.staking);
        json.serialize("stakingAdmin", deployment.stakingAdmin);
        json.serialize("stlMove", deployment.stlMove);
        json.serialize("stlMoveAdmin", deployment.stlMoveAdmin);
        json.serialize("mcr", deployment.mcr);
        json.serialize("mcrAdmin", deployment.mcrAdmin);
        json.serialize("timelock", deployment.timelock);
        string memory newChainData = json.serialize("multisig", deployment.multisig);

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
        json.serialize("move", deploymentData.move);
        json.serialize("moveAdmin", deploymentData.moveAdmin);
        json.serialize("staking", deploymentData.staking);
        json.serialize("stakingAdmin", deploymentData.stakingAdmin);
        json.serialize("stlMove", deploymentData.stlMove);
        json.serialize("stlMoveAdmin", deploymentData.stlMoveAdmin);
        json.serialize("mcr", deploymentData.mcr);
        json.serialize("mcrAdmin", deploymentData.mcrAdmin);
        string memory chainData = json.serialize("timelock", deploymentData.timelock);
        base.serialize(uint2str(chain), chainData);
    }

    // string to address
    function s2a(bytes memory str) public returns (address addr) {
        bytes32 data = keccak256(str);
        assembly {
            mstore(0, data)
            addr := mload(0)
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

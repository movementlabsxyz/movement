// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {SafeProxyFactory} from "@safe-smart-account/contracts/proxies/SafeProxyFactory.sol";
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";
import {SafeProxy} from "@safe-smart-account/contracts/proxies/SafeProxy.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";
import {Vm} from "forge-std/Vm.sol";
import {CREATE3Factory} from "./Create3/CREATE3Factory.sol";

contract Helper is Script {
    using stdJson for string;

    TransparentUpgradeableProxy public moveProxy;
    TransparentUpgradeableProxy public stlMoveProxy;
    TransparentUpgradeableProxy public stakingProxy;
    TransparentUpgradeableProxy public mcrProxy;
    TimelockController public timelock;
    // CREATE3 exists across all major chains, we only enforce it on the same address if not deployed yet
    CREATE3Factory public create3 = CREATE3Factory(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[])";
    string public stakingSignature = "initialize(address)";
    string public stlMoveSignature = "initialize(string,string,address)";
    string public moveSignature = "initialize(address)";
    string public safeSetupSignature = "setup(address[],uint256,address,bytes,address,address,uint256,address)";
    string public root = vm.projectRoot();
    string public deploymentsPath = "/script/helpers/deployments.json";
    string public upgradePath = "/script/helpers/upgrade/";
    string public labsConfigPath = "/script/helpers/labsConfig.json";
    string public foundationConfigPath = "/script/helpers/foundationConfig.json";
    address public ZERO = 0x0000000000000000000000000000000000000000;
    address public NULL = 0x0000000000000000000000000000000000000001;
    string public chainId = uint2str(block.chainid);
    uint256 public foundryChainId = 31337;
    string public storageJson;

    uint256 public minDelay = 2 days;
    ConfigData public labsConfig;
    ConfigData public foundationConfig;

    struct ConfigData {
        uint256 threshold;
        address[] signers;
    }

    Deployment public deployment;

    struct Deployment {
        address move;
        address moveAdmin;
        address mcr;
        address mcrAdmin;
        address staking;
        address stakingAdmin;
        address stlMove;
        address stlMoveAdmin;
        address timelock;
        address movementLabsSafe;
        address movementFoundationSafe;
    }

    function _loadConfig() internal {
        // string memory path = string.concat(root, labsConfigPath);
        // string memory json = vm.readFile(path);
        // bytes memory rawConfigDataLabs = json.parseRaw(string(abi.encodePacked(".")));
        // console.logBytes(rawConfigDataLabs);
        // labsConfig = abi.decode(rawConfigDataLabs, (ConfigData));

        // string memory path2 = string.concat(root, foundationConfigPath);
        // string memory json2 = vm.readFile(path2);
        // bytes memory rawConfigDataFoundation = json2.parseRaw(string(abi.encodePacked(".")));
        // foundationConfig = abi.decode(rawConfigDataFoundation, (ConfigData));

        address[] memory labsSigners = new address[](5);
        labsSigners[0] = 0x49F86Aee2C2187870ece0e64570D0048EaF4C751;
        labsSigners[1] = 0xaFf3deeb13bD2B480751189808C16e9809EeBcce;
        labsSigners[2] = 0x12Cbb2C9F072E955b6B95ad46213aAa984A4434D;
        labsSigners[3] = 0xB2105464215716e1445367BEA5668F581eF7d063;
        labsSigners[4] = 0x0eEd12Ca165A962cd12420DfB38407637bcA4267;

        address[] memory foundationSigners = new address[](1);
        foundationSigners[0] = 0xB2105464215716e1445367BEA5668F581eF7d063;
        // foundationSigners[1] = ZERO;
        // foundationSigners[2] = ZERO;
        // foundationSigners[3] = ZERO;
        // foundationSigners[4] = ZERO;

        labsConfig = ConfigData(4, labsSigners);

        foundationConfig = ConfigData(1, foundationSigners);

        if (labsConfig.signers[0] == NULL) {
            console.log("labsSigner", labsConfig.signers[0]);
            labsConfig.signers[0] = vm.addr(vm.envUint("PRIVATE_KEY"));
        }
        if (foundationConfig.signers[0] == NULL) {
            console.log("foundationSigner", foundationConfig.signers[0]);
            foundationConfig.signers[0] = vm.addr(vm.envUint("PRIVATE_KEY"));
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

    function _deploySafes() internal {
        console.log("Deploying Safes");
        if (deployment.movementLabsSafe == NULL && block.chainid != foundryChainId) {
            // use canonical v1.4.1 safe factory address 0x4e1DCf7AD4e460CfD30791CCC4F9c8a4f820ec67 if:
            // - chainid is not foundry
            // - safe is not deployed
            SafeProxyFactory safeFactory = SafeProxyFactory(0x4e1DCf7AD4e460CfD30791CCC4F9c8a4f820ec67);
            deployment.movementLabsSafe = _deploySafe(
                safeFactory,
                0x41675C099F32341bf84BFc5382aF534df5C7461a,
                0xfd0732Dc9E303f09fCEf3a7388Ad10A83459Ec99,
                labsConfig.signers,
                labsConfig.threshold
            );
            deployment.movementFoundationSafe = _deploySafe(
                safeFactory,
                0x41675C099F32341bf84BFc5382aF534df5C7461a,
                0xfd0732Dc9E303f09fCEf3a7388Ad10A83459Ec99,
                foundationConfig.signers,
                foundationConfig.threshold
            );
        } else {
            if (block.chainid == foundryChainId) {
                SafeProxyFactory safeFactory = new SafeProxyFactory();
                Safe safeSingleton = new Safe();
                CompatibilityFallbackHandler fallbackHandler = new CompatibilityFallbackHandler();
                deployment.movementLabsSafe = _deploySafe(
                    safeFactory,
                    address(safeSingleton),
                    address(fallbackHandler),
                    labsConfig.signers,
                    labsConfig.threshold
                );
                deployment.movementFoundationSafe = _deploySafe(
                    safeFactory,
                    address(safeSingleton),
                    address(fallbackHandler),
                    foundationConfig.signers,
                    foundationConfig.threshold
                );
            }
        }
        console.log("Safe addresses:");
        console.log("Labs:", address(deployment.movementLabsSafe));
        console.log("Foundation:", address(deployment.movementFoundationSafe));
    }

    function _deploySafe(
        SafeProxyFactory safeFactory,
        address safeSingleton,
        address fallbackHandler,
        address[] memory signers,
        uint256 threshold
    ) internal returns (address safe) {
        safe = payable(
            address(
                safeFactory.createProxyWithNonce(
                    safeSingleton,
                    abi.encodeWithSignature(
                        safeSetupSignature, signers, threshold, ZERO, "0x", fallbackHandler, ZERO, 0, payable(ZERO)
                    ),
                    0
                )
            )
        );
    }

    function _deployTimelock() internal {
        if (deployment.timelock == NULL) {
            timelock = new TimelockController(minDelay, labsConfig.signers, foundationConfig.signers, ZERO);
            deployment.timelock = address(timelock);
        }
    }

    function _deployCreate3() internal {
        if (address(create3).code.length == 0) {
            console.log("CREATE3: deploying");
            create3 = new CREATE3Factory();
        }
    }

    function _storeAdminDeployment() internal returns (address admin) {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        admin = logs[logs.length - 2].emitter;
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

    function _serializer(string memory json, Deployment memory memoryDeployment) internal returns (string memory) {
        json.serialize("move", memoryDeployment.move);
        json.serialize("moveAdmin", memoryDeployment.moveAdmin);
        json.serialize("staking", memoryDeployment.staking);
        json.serialize("stakingAdmin", memoryDeployment.stakingAdmin);
        json.serialize("stlMove", memoryDeployment.stlMove);
        json.serialize("stlMoveAdmin", memoryDeployment.stlMoveAdmin);
        json.serialize("mcr", memoryDeployment.mcr);
        json.serialize("mcrAdmin", memoryDeployment.mcrAdmin);
        json.serialize("timelock", memoryDeployment.timelock);
        json.serialize("movementLabsSafe", memoryDeployment.movementLabsSafe);
        return json.serialize("movementFoundationSafe", memoryDeployment.movementFoundationSafe);
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

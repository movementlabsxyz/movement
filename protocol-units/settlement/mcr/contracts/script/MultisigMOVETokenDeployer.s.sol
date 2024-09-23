pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import { Helper, Safe } from "./helpers/Helper.sol";
import {Vm} from "forge-std/Vm.sol";
import {ICREATE3Factory} from "./helpers/Create3/ICREATE3Factory.sol";
import {Enum} from "@safe-smart-account/contracts/common/Enum.sol";


// Script intended to be used for deploying the MOVE token from an EOA
// Utilizies existing safes and sets them as proposers and executors.
// The MOVEToken contract takes in the Movement Foundation address and sets it as its own admin for future upgrades.
// The whole supply is minted to the Movement Foundation Safe.
// The script also verifies that the token has the correct balances, decimals and permissions.
contract MultisigMOVETokenDeployer is Helper {
    // COMMANDS
    // mainnet
    // forge script MOVETokenDeployer --fork-url https://eth.llamarpc.com --verify --etherscan-api-key ETHERSCAN_API_KEY
    // testnet
    // forge script MOVETokenDeployer --fork-url https://eth-sepolia.api.onfinality.io/public
    // Safes should be already deployed
    bytes32 public salt = 0x00000000000000000000000012bb669b1a73513f43bb92816a3461b7717f3638;
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function run() external virtual {
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
        deployment.moveAdmin == ZERO && deployment.move == ZERO ?
            _deployMultisigMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                // if move is already deployed, upgrade it
                _upgradeMultisigMove() : revert("MOVE: both admin and proxy should be registered");
        
        require(MOVEToken(deployment.move).balanceOf(address(deployment.movementFoundationSafe)) == 1000000000000000000, "Movement Foundation Safe balance is wrong");
        require(MOVEToken(deployment.move).decimals() == 8, "Decimals are expected to be 8"); 
        require(MOVEToken(deployment.move).totalSupply() == 1000000000000000000,"Total supply is wrong");
        require(MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementFoundationSafe)),"Movement Foundation expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementLabsSafe)),"Movement Labs not expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(timelock)),"Timelock not expected to have token admin role");
        vm.stopBroadcast();
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•

    function _deployMultisigMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        // genetares bytecode for CREATE3 deployment
        bytes memory create3Bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, deployment.movementFoundationSafe))
        );
        vm.recordLogs();
        // craete bytecode the MOVE token proxy using CREATE3
        bytes memory bytecode = abi.encodeWithSignature("deploy(bytes32,bytes)", salt, create3Bytecode);
        bytes32 digest = Safe(payable(deployment.movementFoundationSafe)).getTransactionHash(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), 0
        );

        // three signers for the deployment (this is mocked and only works in foundry chain)
        uint256[] memory signers = new uint256[](3);
        signers[0] = vm.envUint("PRIVATE_KEY");
        signers[1] = 1;
        signers[2] = 2;

        bytes memory signatures = _generateSignatures(signers, digest);

        Safe(payable(deployment.movementFoundationSafe)).execTransaction(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), signatures
        );
        // moveProxy = 
        console.log("MOVEToken deployment records:");
        Vm.Log[] memory logs = vm.getRecordedLogs();
        deployment.move = logs[0].emitter;
        deployment.moveAdmin = logs[logs.length-3].emitter;
        console.log("proxy", deployment.move);
        console.log("admin", deployment.moveAdmin);
    }
    

    // MULTISIG WILL NEVER BE USED WITHIN THE CONTRACT PIPELINE
    function _upgradeMultisigMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        timelock.schedule(
            deployment.moveAdmin,
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                deployment.move,
                address(newMoveImplementation),
                abi.encodeWithSignature("initialize(address)", deployment.movementFoundationSafe)
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + config.minDelay
        );
    }
}

pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import { Helper } from "./helpers/Helper.sol";
import {Vm} from "forge-std/Vm.sol";
import {ICREATE3Factory} from "./helpers/Create3/ICREATE3Factory.sol";


// Script intended to be used for deploying the MOVE token from an EOA
// Utilizies existing safes and sets them as proposers and executors.
// The MOVEToken contract takes in the Movement Foundation address and sets it as its own admin for future upgrades.
// The whole supply is minted to the Movement Foundation Safe.
// The script also verifies that the token has the correct balances, decimals and permissions.
contract MOVETokenDeployer is Helper {
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
            _deployMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                // if move is already deployed, upgrade it
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");
        
        require(MOVEToken(deployment.move).balanceOf(address(deployment.movementFoundationSafe)) == 1000000000000000000, "Movement Foundation Safe balance is wrong");
        require(MOVEToken(deployment.move).decimals() == 8, "Decimals are expected to be 8"); 
        require(MOVEToken(deployment.move).totalSupply() == 1000000000000000000,"Total supply is wrong");
        require(MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementFoundationSafe)),"Movement Foundation expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementLabsSafe)),"Movement Labs not expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(timelock)),"Timelock not expected to have token admin role");
        vm.stopBroadcast();
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•

    function _deployMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        // genetares bytecode for CREATE3 deployment
        bytes memory bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, deployment.movementFoundationSafe))
        );
        vm.recordLogs();
        // deploys the MOVE token proxy using CREATE3
        moveProxy = TransparentUpgradeableProxy(payable(ICREATE3Factory(create3).deploy(salt, bytecode)));
        console.log("MOVEToken deployment records:");
        console.log("proxy", address(moveProxy));
        deployment.move = address(moveProxy);
        deployment.moveAdmin = _storeAdminDeployment();
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        string memory json = "safeCall";
        
        // Prepare the data for the upgrade
        bytes memory data = abi.encodeWithSignature(
            "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
            address(deployment.moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(deployment.move),
                address(newMoveImplementation),
                abi.encodeWithSignature("initialize(address)", deployment.movementFoundationSafe)
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
        console.log("MOVE upgrade json |start|", serializedData, "|end|");

        // Write the serialized data to a file
        vm.writeFile(string.concat(root, upgradePath, "move.json"), serializedData);
    }
}

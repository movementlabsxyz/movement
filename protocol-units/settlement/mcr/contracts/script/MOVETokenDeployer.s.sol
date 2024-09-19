pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import { Helper } from "./helpers/Helper.sol";
import {Vm} from "forge-std/Vm.sol";

interface create {
    function deploy(bytes32 _salt, bytes memory _bytecode) external returns (address);
}


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
    address create3address = address(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);
    address moveAdmin;
    bytes32 public salt = 0xc000000000000000000000002774b8b4881d594b03ff8a93f4cad69407c90350;
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function run() external virtual {
        // load config data
        _loadConfig();

        // Load deployment data
        _loadDeployments();

        // Deploy Safes
        _deploySafes();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        // timelock is required for all deployments
        _deployTimelock();
        
        deployment.moveAdmin == ZERO && deployment.move == ZERO ?
            _deployMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                // if move is already deployed, upgrade it
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");
        
        require(MOVEToken(address(moveProxy)).balanceOf(address(deployment.movementFoundationSafe)) == 1000000000000000000, "Movement Foundation Safe balance is wrong");
        require(MOVEToken(address(moveProxy)).decimals() == 8, "Decimals are expected to be 8"); 
        require(MOVEToken(address(moveProxy)).totalSupply() == 1000000000000000000,"Total supply is wrong");
        require(MOVEToken(address(moveProxy)).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementFoundationSafe)),"Movement Foundation expected to have token admin role");
        require(!MOVEToken(address(moveProxy)).hasRole(DEFAULT_ADMIN_ROLE, address(timelock)),"Timelock not expected to have token admin role");
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
        moveProxy = TransparentUpgradeableProxy(payable(create(create3address).deploy(salt, bytecode)));
        console.log("MOVEToken deployment records:");
        console.log("proxy", address(moveProxy));
        deployment.move = address(moveProxy);
        deployment.moveAdmin = _storeAdminDeployment();
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        timelock.schedule(
            address(moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(moveProxy),
                address(newMoveImplementation),
                abi.encodeWithSignature("initialize(address)", deployment.movementFoundationSafe)
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + config.minDelay
        );
    }
}

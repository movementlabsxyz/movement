// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import "forge-std/Test.sol";
import {MOVETokenV3} from "../src/token/MOVETokenV3.sol";
import {CREATE3Factory, ICREATE3Factory} from "../script/helpers/Create3/CREATE3Factory.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {Enum} from "@safe-smart-account/contracts/common/Enum.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";


interface IProxyFactory {
    function createProxyWithNonce(address masterCopy, bytes memory data, uint256 nonce)
        external
        returns (address proxy);
}

interface IDelegates {
        function delegates(address) external view returns (address);
    }

contract DeployHypernativeTest is Test {
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    address public lzEndpoint = 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9;
    IProxyFactory public ProxyFactory = IProxyFactory(0xa6B71E26C5e0845f74c812102Ca7114b6a896AB2);
    address constant MASTER_COPY_ADDRESS = 0xd9Db270c1B5E3Bd161E8c8503c55cEABeE709552;
    CREATE3Factory public create3 = CREATE3Factory(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);
    MOVETokenV3 public moveTokenV3Implementation;
    MOVETokenV3 public move = MOVETokenV3(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);
    address constant ZERO = address(0x0);

    bytes32 public salt = 0x6c0000000000000000000000018eddf77afc0a5c6d05a564a44fe37b068922c3;

    function setUp() public {}

    function testDeploy() public {
        // Simulate a transaction from TEST_ADDRESS
        vm.startPrank(DEPLOYER_ADDRESS);

        address expectedAddress = 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2;
        address expectedAddress2 = 0x7aE744e3b2816F660054EAbd1a1C4935DA34Ae28;
        bytes memory setPeerSignature = abi.encodeWithSignature("setPeer(uint32,bytes32)", 30325, 0x0);
        bytes memory firstInitData =
            hex"b63e800d00000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000000000000b2105464215716e1445367bea5668f581ef7d06300000000000000000000000049f86aee2c2187870ece0e64570d0048eaf4c75100000000000000000000000012cbb2c9f072e955b6b95ad46213aaa984a4434d000000000000000000000000aff3deeb13bd2b480751189808c16e9809eebcce0000000000000000000000000eed12ca165a962cd12420dfb38407637bca42670000000000000000000000000000000000000000000000000000000000000000";
        bytes memory secondInitData =
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000b2105464215716e1445367bea5668f581ef7d0630000000000000000000000003eb69ef2dbedd5d58aa5e074131cd22d5e87ff530000000000000000000000000000000000000000000000000000000000000000";

        address multisigLabsOps = ProxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, firstInitData, 0);
        address multisigDeployer = ProxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, secondInitData, 0);
        assertEq(multisigLabsOps, expectedAddress);
        assertEq(multisigDeployer, expectedAddress2);

        moveTokenV3Implementation = new MOVETokenV3(lzEndpoint);

        bytes memory create3Bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(
                address(moveTokenV3Implementation),
                address(multisigLabsOps),
                abi.encodeWithSignature("initialize(address)", DEPLOYER_ADDRESS)
            )
        );

        // craete bytecode the MOVE token proxy using CREATE3
        bytes memory bytecode = abi.encodeWithSignature("deploy(bytes32,bytes)", salt, create3Bytecode);
        bytes32 digest = Safe(payable(multisigDeployer)).getTransactionHash(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), 0
        );

        assertEq(vm.addr(vm.envUint("PRIVATE_KEY")), DEPLOYER_ADDRESS);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(vm.envUint("PRIVATE_KEY"), digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        console.logBytes(signature);

        require(signature.length >= 65);

        Safe(payable(multisigDeployer)).execTransaction(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), signature
        );

        vm.stopPrank();

    }

    

    function testVerifyMoveTokenDeployment() public {
        testDeploy();
        assertEq(move.decimals(), 8);
        assertEq(move.name(), "Movement");
        assertEq(move.symbol(), "MOVE");
        assertEq(move.owner(), DEPLOYER_ADDRESS);
        assertEq(address(move.endpoint()), lzEndpoint);
        assertEq(move.totalSupply(), 0);
        assertTrue(move.DOMAIN_SEPARATOR() != bytes32(0));
        assertEq(IDelegates(address(move.endpoint())).delegates(address(move)), DEPLOYER_ADDRESS);

        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            bytes32 salt,
            uint256[] memory extensions
        ) = move.eip712Domain();

        assertEq(fields, hex"0f", "fields should be 0x0f (01111)");
        assertEq(name, "Movement", "EIP-712 name should be Movement");
        assertEq(version, "1", "EIP-712 version should be 1");
        assertEq(chainId, block.chainid, "chainId should match block.chainid");
        assertEq(verifyingContract, address(move), "verifyingContract should be the token address");
        assertEq(salt, bytes32(0), "salt should be zero");
        assertEq(extensions.length, 0, "extensions array should be empty");
    }
}

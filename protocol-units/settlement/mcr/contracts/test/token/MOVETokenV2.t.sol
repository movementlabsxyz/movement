// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// =============================================================================
// IMPORTS
// =============================================================================

// Forge testing framework
import "forge-std/Test.sol";

// Local contracts
import {MOVEToken} from "../../src/token/MOVEToken.sol";
import {MOVETokenV2} from "../../src/token/MOVETokenV2.sol";

// OpenZeppelin contracts
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

// LayerZero contracts
import {EndpointV2Mock} from "@layerzerolabs/test-devtools-evm-foundry/contracts/mocks/EndpointV2Mock.sol";
import {ExecutorConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/SendLibBase.sol";
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {SetConfigParam} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";
import {UlnConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/uln/UlnBase.sol";
import {EnforcedOptionParam} from "@layerzerolabs/oapp-evm/contracts/oapp/interfaces/IOAppOptionsType3.sol";
import {
    IOFT,
    SendParam,
    OFTLimit,
    OFTReceipt,
    OFTFeeDetail,
    MessagingReceipt,
    MessagingFee
} from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";
import {IOAppCore} from "@layerzerolabs/oapp-evm/contracts/oapp/interfaces/IOAppCore.sol";
import {MessagingFee} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {OFTAdapter} from "lib/LayerZero-v2/packages/layerzero-v2/evm/oapp/contracts/oft/OFTAdapter.sol";
import {OptionsBuilder} from "lib/LayerZero-v2/packages/layerzero-v2/evm/oapp/contracts/oapp/libs/OptionsBuilder.sol";

// Safe contracts
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";

interface IRetrieveDelegates {
    function delegates(address account) external view returns (address);
}

contract MOVETokenV2Test is Test {
    // =============================================================================
    // STATE VARIABLES - CONTRACT INSTANCES
    // =============================================================================

    MOVEToken public move;
    MOVETokenV2 public move2;
    MOVETokenV2 public moveTokenImplementation2;

    /// @dev Bytes32 representation of the Movement OFT adapter address
    bytes32 public moveOftAdapterBytes32 = 0x7e4fd97ef92302eea9b10f74be1d96fb1f1511cf7ed28867b0144ca89c6ebc3c;

    /// @dev Proxy admin contract for managing upgrades
    ProxyAdmin public admin = ProxyAdmin(payable(0x8365AA031806A1ac2b31a5d3b8323020FC85DfEc));
    /// @dev Transparent upgradeable proxy for MOVE token
    TransparentUpgradeableProxy public moveProxy =
        TransparentUpgradeableProxy(payable(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073));
    /// @dev Timelock controller for governance operations
    TimelockController public timelock = TimelockController(payable(0x25a5A3FA61cba5Fd5fb1D75D0AcfEB81370778Eb));
    /// @dev LayerZero endpoint V2 for cross-chain messaging
    ILayerZeroEndpointV2 public endpoint = ILayerZeroEndpointV2(payable(0x1a44076050125825900e736c501f859c50fE728c));

    // =============================================================================
    // STATE VARIABLES - ADDRESSES
    // =============================================================================

    /// @dev Anchorage custody address - holds user funds
    address public anchorage = 0xe3e86E126fcCd071Af39a0899734Ca5C8E5F4F25;
    /// @dev Movement Labs multisig address - operational control
    address public labs = 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2;
    /// @dev Movement Foundation address - governance entity
    address public foundation = 0xB304C899EcB46DD91F31Ef0d177fF9dAf8C17edf;
    /// @dev Deprecated bridge address - to be burned during upgrade
    address public bridge = 0xf1dF43A3053cd18E477233B59a25fC483C2cBe0f;
    /// @dev Old foundation address - previous admin, replaced during upgrade
    address public oldFoundation = 0x074C155f09cE5fC3B65b4a9Bbb01739459C7AD63;

    // LayerZero infrastructure addresses
    /// @dev LayerZero send Ultra Light Node v3.0.2 address
    address public sendUln302 = 0xbB2Ea70C9E858123480642Cf96acbcCE1372dCe1;
    /// @dev LayerZero receive Ultra Light Node v3.0.2 address
    address public receiveUln302 = 0xc02Ab410f0734EFa3F14628780e6e695156024C2;
    /// @dev LayerZero executor address for message execution
    address public lzExecutor = 0x173272739Bd7Aa6e4e214714048a9fE699453059;

    // DVN (Data Verification Network) addresses
    /// @dev P2P DVN address for message verification
    address public p2pDVN = 0x06559EE34D85a88317Bf0bfE307444116c631b67;
    /// @dev Horizen DVN address for message verification
    address public horizenDVN = 0x380275805876Ff19055EA900CDb2B46a94ecF20D;
    /// @dev LayerZero Labs DVN address for message verification
    address public lzDVN = 0x589dEDbD617e0CBcB916A9223F4d1300c294236b;
    /// @dev Nethermind DVN address for message verification
    address public nethermindDVN = 0xa59BA433ac34D2927232918Ef5B2eaAfcF130BA5;

    // =============================================================================
    // STATE VARIABLES - CONFIGURATION
    // =============================================================================

    /// @dev Signature for MOVE token initialization function
    string public moveSignature = "initialize(address,address)";
    /// @dev Number of block confirmations required for LayerZero DVN
    uint64 public confirmations = 0;
    /// @dev Movement blockchain endpoint ID for LayerZero
    uint32 public movementEid = 30325;
    /// @dev Minimum delay for timelock operations (2 days)
    uint256 public minDelay = 2 days;

    // =============================================================================
    // STATE VARIABLES - CONSTANTS
    // =============================================================================

    /// @dev LayerZero config type for executor configuration
    uint32 public constant EXECUTOR_CONFIG_TYPE = 1;
    /// @dev LayerZero config type for ULN (Ultra Light Node) configuration
    uint32 public constant ULN_CONFIG_TYPE = 2;
    /// @dev LayerZero config type for receive library configuration
    uint32 public constant RECEIVE_CONFIG_TYPE = 2;
    
    /// @dev Total supply of MOVE tokens (10 billion with 8 decimals)
    uint256 public constant TOTAL_SUPPLY = 10000000000 * 10 ** 8;
    /// @dev MOVE token decimals
    uint8 public constant MOVE_DECIMALS = 8;
    /// @dev Default admin role identifier
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    // =============================================================================
    // SETUP
    // =============================================================================

    /**
     * @dev Sets up the test environment by deploying a new MOVETokenV2 implementation
     *      and initializing proxy instances for both MOVEToken and MOVETokenV2
     */
    function setUp() public {
        moveTokenImplementation2 = new MOVETokenV2(address(endpoint));

        move = MOVEToken(address(moveProxy));
        move2 = MOVETokenV2(address(moveProxy));
    }

    // =============================================================================
    // BASIC TOKEN TESTS
    // =============================================================================

    /**
     * @dev Tests that the MOVE token cannot be initialized twice
     *      Should revert with InvalidInitialization error (0xf92ee8a9)
     */
    function testCannotInitializeTwice() public {
        vm.expectRevert(0xf92ee8a9);
        move.initialize(labs, anchorage);
    }

    /**
     * @dev Fuzz test for admin role management in MOVE token V1
     *      Tests that only the oldFoundation can grant admin roles (0x00)
     *      Verifies that unauthorized accounts cannot grant admin privileges
     * @param other Random address to test role assignment
     */
    function testAdminRoleFuzz(address other) public {
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, other), false);

        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), DEFAULT_ADMIN_ROLE)
        );
        move.grantRole(DEFAULT_ADMIN_ROLE, other);

        vm.prank(labs);
        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, labs, DEFAULT_ADMIN_ROLE));
        move.grantRole(DEFAULT_ADMIN_ROLE, other);

        vm.prank(oldFoundation);
        move.grantRole(DEFAULT_ADMIN_ROLE, other);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, other), true);
    }

    // =============================================================================
    // UPGRADE TESTS
    // =============================================================================

    /**
     * @dev Tests the schedule to upgrade process from MOVEToken to MOVETokenV2 via timelock
     *      This test simulates the real upgrade scenario including:
     *      1. Scheduling the upgrade through timelock with proper delay
     *      2. Pausing the existing bridge during the upgrade window
     */
    function testScheduleAndSetPeer() public {
        assertEq(admin.owner(), address(timelock));
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, oldFoundation), true);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, anchorage), false);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, labs), false);

        // Define deprecated addresses whose balances will be burned during upgrade
        address[] memory deprecated = new address[](1);
        deprecated[0] = bridge;

        bytes memory initializeData =
            abi.encodeWithSignature("initialize(address,address,address[])", labs, oldFoundation, deprecated);

        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementation2),
            initializeData
        );

        vm.prank(labs);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), minDelay);

        // While upgrade is scheduled, labs pauses the existing bridge to prevent new transactions
        vm.startPrank(labs);
        OFTAdapter(payable(bridge)).setPeer(30325, 0x0);
    }

    /**
     * @dev Tests the complete upgrade process from MOVEToken to MOVETokenV2 via timelock
     *      This test simulates the real upgrade scenario including:
     *      1. Scheduling the upgrade through timelock with proper delay
     *      2. Pausing the existing bridge during the upgrade window
     *      3. Executing the upgrade after timelock delay
     *      4. Burning deprecated bridge balances during initialization
     *      5. Transferring admin roles from oldFoundation to labs
     *      6. Verifying all state transitions and access controls
     */
    function testUpgradeFromTimelock() public {
        // Define deprecated addresses whose balances will be burned during upgrade
        address[] memory deprecated = new address[](1);
        deprecated[0] = bridge;

        bytes memory initializeData =
            abi.encodeWithSignature("initialize(address,address,address[])", labs, oldFoundation, deprecated);

        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementation2),
            initializeData
        );
        
        // Once transaction is scheduled comment out testScheduleAndSetPeer and RERUN TEST to verify that all arguments are correct
        testScheduleAndSetPeer();

        // Verify that upgrade cannot be executed before timelock delay
        vm.expectRevert();
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.warp(block.timestamp + minDelay + 1);

        uint256 bridgeBalance = move.balanceOf(bridge);

        vm.prank(foundation);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify V1 token state after upgrade with old interface
        assertEq(move.decimals(), MOVE_DECIMALS);
        assertEq(move.totalSupply(), TOTAL_SUPPLY - bridgeBalance);
        assertEq(move.balanceOf(bridge), 0);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, oldFoundation), false);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, foundation), false);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, labs), true);
        assertEq(move.hasRole(DEFAULT_ADMIN_ROLE, anchorage), false);

        // Verify V2 token state after upgrade
        assertEq(move2.decimals(), 8);
        assertEq(move2.totalSupply(), 10000000000 * 10 ** 8 - bridgeBalance);
        assertEq(move2.balanceOf(bridge), 0);
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, oldFoundation), false);
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, foundation), false);
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, labs), true);
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, anchorage), false);
        assertEq(move2.endpoint() == endpoint, true);
        assertEq(move2.owner(), labs);

        // couldn't find exactly where delegates function is defined, so using IRetrieveDelegates interface
        // asserts labs is the delegate for move2 being able to perform operational tasks
        assertEq(IRetrieveDelegates(address(move2.endpoint())).delegates(address(move2)), labs);

        // Verify proxy implementation was updated correctly
        bytes32 implementation = vm.load(address(moveProxy), ERC1967Utils.IMPLEMENTATION_SLOT);
        address implAddr;
        assembly {
            implAddr := and(implementation, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }

        assertEq(implAddr, address(moveTokenImplementation2));
    }

    /**
     * @dev Tests that MOVETokenV2 cannot be reinitialized after upgrade
     *      Verifies protection against multiple initialization attempts from various actors
     *      including labs, oldFoundation, foundation, and proxy admin
     */
    function testCannotReinitialize2() public {
        address[] memory deprecated = new address[](1);
        deprecated[0] = bridge;

        bytes memory initializeData =
            abi.encodeWithSignature("initialize(address,address,address[])", labs, oldFoundation, deprecated);

        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementation2),
            initializeData
        );

        testUpgradeFromTimelock();
        // Using implementation slot as salt for this test
        bytes32 salt = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;
        vm.prank(labs);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), salt, minDelay);

        vm.warp(block.timestamp + minDelay + 1);
        vm.prank(foundation);
        vm.expectRevert(0xf92ee8a9); // InvalidInitialization
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), salt);

        // Test direct initialization attempts from various actors
        vm.prank(labs);
        vm.expectRevert(0xf92ee8a9); // InvalidInitialization
        move2.initialize(labs, oldFoundation, deprecated);

        vm.prank(oldFoundation);
        vm.expectRevert(0xf92ee8a9); // InvalidInitialization
        move2.initialize(labs, oldFoundation, deprecated);

        vm.prank(foundation);
        vm.expectRevert(0xf92ee8a9); // InvalidInitialization
        move2.initialize(labs, oldFoundation, deprecated);

        vm.prank(address(admin));
        vm.expectRevert(TransparentUpgradeableProxy.ProxyDeniedAdminAccess.selector);
        move2.initialize(labs, oldFoundation, deprecated);
    }

    /**
     * @dev Fuzz test for admin role management in MOVETokenV2 after upgrade
     *      Tests that only labs can grant admin roles after the upgrade
     *      Verifies that oldFoundation and foundation lose admin privileges
     * @param other Random address to test role assignment
     */
    function testAdminRole2Fuzz(address other) public {
        testUpgradeFromTimelock();
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, other), false);

        vm.prank(other);
        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, other, DEFAULT_ADMIN_ROLE));
        move2.grantRole(DEFAULT_ADMIN_ROLE, other);

        vm.prank(foundation);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, foundation, DEFAULT_ADMIN_ROLE)
        );
        move2.grantRole(DEFAULT_ADMIN_ROLE, other);

        vm.prank(oldFoundation);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, oldFoundation, DEFAULT_ADMIN_ROLE)
        );
        move2.grantRole(DEFAULT_ADMIN_ROLE, other);

        vm.prank(labs);
        move2.grantRole(DEFAULT_ADMIN_ROLE, other);
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, other), true);
    }

    /**
     * @dev Fuzz test ensuring no unintended admin roles are granted during upgrade
     *      Verifies that only labs has admin role after upgrade, no other addresses
     * @param other Random address to verify has no admin role
     */
    function testNoExtraAdminFuzz(address other) public {
        vm.assume(other != labs);
        testUpgradeFromTimelock();
        assertEq(move2.hasRole(DEFAULT_ADMIN_ROLE, other), false);
    }

    /**
     * @dev Tests that the deprecated bridge cannot be used after upgrade
     *      Verifies that the old bridge balance was burned during upgrade
     *      Tests that all bridge operations (quoteSend, send) fail for the deprecated bridge
     *      while the new MOVETokenV2 can still provide quotes for the same parameters
     */
    function testDeprecatedBridge() public {
        testSend();
        assertEq(move2.balanceOf(bridge), 0);

        uint256 amount = 1 * 10 ** MOVE_DECIMALS;
        vm.prank(anchorage);

        // Movement Multisig address on destination chain
        bytes32 moveAddress = 0x98ebb7985c84a89972022edf391bdaa7d95f061d9742efb3703de368413431e1;
        SendParam memory sendParam = SendParam({
            dstEid: movementEid,
            to: moveAddress,
            amountLD: amount,
            minAmountLD: amount,
            extraOptions: bytes(""),
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });
        
        IOFT deprecatedBridge = IOFT(bridge);
        vm.expectRevert(); // Should fail - bridge is deprecated
        deprecatedBridge.quoteSend(sendParam, false);

        // Verify new contract can still provide quotes for same parameters
        MessagingFee memory fee = move2.quoteSend(sendParam, false);

        // Test that deprecated bridge cannot send tokens
        vm.prank(anchorage);
        vm.expectRevert(); // Should fail - bridge is deprecated
        deprecatedBridge.send(sendParam, fee, anchorage);

        vm.prank(anchorage);
        vm.expectRevert(); // Should fail - bridge is deprecated
        deprecatedBridge.send{value: fee.nativeFee}(sendParam, fee, anchorage);
    }

    // =============================================================================
    // OFT (OMNICHAIN FUNGIBLE TOKEN) TESTS
    // =============================================================================

    /**
     * @dev Tests basic OFT (Omnichain Fungible Token) features after upgrade
     *      Verifies endpoint integration and decimal consistency
     */
    function testOFTFeatures() public {
        testUpgradeFromTimelock();

        assertEq(address(move2.endpoint()), address(endpoint));
        assertEq(move2.decimals(), MOVE_DECIMALS);
    }

    /**
     * @dev Tests OFT configuration including DVN setup and enforced options
     *      Configures LayerZero infrastructure (DVNs, executor, ULN libraries)
     *      Sets and verifies enforced options for cross-chain messaging
     */
    function testConfigOFT() public {
        testUpgradeFromTimelock();

        vm.startPrank(labs);
        configDVNExecutor(address(move2));

        // Verify send library configuration
        address receivedSendLib = move2.endpoint().getSendLibrary(address(move2), movementEid);
        assertEq(receivedSendLib, sendUln302);

        // Verify receive library configuration
        (address receivedReceiveLib,) = move2.endpoint().getReceiveLibrary(address(move2), movementEid);
        assertEq(receivedReceiveLib, receiveUln302);

        // Set enforced options for gas and execution parameters
        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000001388));
        setEnforcedParams(options);

        // Verify enforced options are set for both message types
        assertEq(move2.enforcedOptions(movementEid, uint16(1)), options);
        assertEq(move2.enforcedOptions(movementEid, uint16(2)), options);
        vm.stopPrank();
    }

    /**
     * @dev Tests cross-chain token sending functionality via LayerZero OFT
     *      Tests the complete flow from quote generation to token burning
     *      Verifies peer setup requirements, balance changes, and restrictions
     *      Tests both basic sends and sends with destination gas drops
     */
    function testSend() public {
        testConfigOFT();

        console.log("anchorage balance");
        console.log(move2.balanceOf(anchorage));

        uint256 amount = 1 * 10 ** MOVE_DECIMALS;
        vm.prank(anchorage);

        // Movement Multisig address on destination chain
        bytes32 moveAddress = 0x98ebb7985c84a89972022edf391bdaa7d95f061d9742efb3703de368413431e1;
        SendParam memory sendParam = SendParam({
            dstEid: movementEid,
            to: moveAddress,
            amountLD: amount,
            minAmountLD: amount,
            extraOptions: bytes(""),
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });

        // Should fail before peer is set
        vm.expectRevert();
        move2.quoteSend(sendParam, false);

        vm.prank(anchorage);
        vm.expectRevert();
        move2.send(sendParam, MessagingFee({nativeFee: 100000000000000, lzTokenFee: 0}), bridge);

        // Set peer connection to Movement blockchain
        vm.prank(labs);
        move2.setPeer(movementEid, moveOftAdapterBytes32);

        // Now quote and send should work
        MessagingFee memory fee = move2.quoteSend(sendParam, false);
        uint256 totalSupplyBefore = move2.totalSupply();
        uint256 balanceBefore = move2.balanceOf(anchorage);
        vm.prank(anchorage);
        move2.send{value: fee.nativeFee}(sendParam, fee, bridge);

        // Send burns tokens on source chain
        assertEq(move2.balanceOf(anchorage), balanceBefore - amount);
        assertEq(move2.totalSupply(), totalSupplyBefore - amount);

        vm.deal(anchorage, 10000000000000000000000); // Fund for gas

        // Test sending to invalid endpoint ID
        SendParam memory badSendParam = SendParam({
            dstEid: 30324, // Different EID - should fail
            to: moveAddress,
            amountLD: amount,
            minAmountLD: amount,
            extraOptions: bytes(""),
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });

        vm.prank(anchorage);
        vm.expectRevert(); // Should fail - no peer set for this EID
        move2.send{value: fee.nativeFee}(badSendParam, fee, anchorage);

        // Test send with native gas drop on destination
        bytes memory option =
            OptionsBuilder.addExecutorNativeDropOption(OptionsBuilder.newOptions(), 10000000, moveAddress);

        SendParam memory sendWithGasParam = SendParam({
            dstEid: movementEid,
            to: moveAddress,
            amountLD: amount,
            minAmountLD: amount,
            extraOptions: option,
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });

        MessagingFee memory sendWithGasFee = move2.quoteSend(sendWithGasParam, false);

        vm.prank(anchorage);
        move2.send{value: sendWithGasFee.nativeFee}(sendParam, sendWithGasFee, anchorage);

        // Verify total of 2 sends completed
        assertEq(move2.balanceOf(anchorage), balanceBefore - (amount * 2));
        assertEq(move2.totalSupply(), totalSupplyBefore - (amount * 2));
    }

    // =============================================================================
    // HELPER FUNCTIONS
    // =============================================================================

    /**
     * @dev Utility function to convert bytes to address using keccak256 hash
     *      Used for generating deterministic addresses from string inputs
     * @param str Input bytes to convert
     * @return addr Resulting address from hash
     */
    function string2Address(bytes memory str) public pure returns (address addr) {
        bytes32 data = keccak256(str);
        assembly {
            mstore(0, data)
            addr := mload(0)
        }
    }

    /**
     * @dev Configures LayerZero DVN (Data Verification Network) and executor settings
     *      Sets up the required infrastructure for cross-chain message verification
     *      Configures 3 DVNs for security and the LayerZero executor
     * @param adapter Address of the OFT adapter to configure
     */
    function configDVNExecutor(address adapter) public {
        setLibraries(adapter, movementEid, sendUln302, receiveUln302);

        // Configure required DVNs for message verification
        address[] memory dvnArray = new address[](3);
        dvnArray[0] = p2pDVN;
        dvnArray[1] = horizenDVN;
        dvnArray[2] = lzDVN;

        address[] memory emptyArray = new address[](0);
        UlnConfig memory ulnConfig =
            UlnConfig(uint64(confirmations), uint8(3), uint8(0), uint8(0), dvnArray, emptyArray);
        ExecutorConfig memory executorConfig = ExecutorConfig(0, lzExecutor);
        setConfigs(adapter, movementEid, sendUln302, receiveUln302, ulnConfig, executorConfig);
    }

    /**
     * @dev Sets LayerZero configuration parameters for send and receive libraries
     *      Configures executor and ULN settings for both sending and receiving messages
     * @param contractAddress Address of the contract to configure
     * @param remoteEid Remote endpoint ID to configure for
     * @param sendLibraryAddress Address of the send library
     * @param receiveLibraryAddress Address of the receive library
     * @param ulnConfig ULN configuration with DVN settings
     * @param executorConfig Executor configuration settings
     */
    function setConfigs(
        address contractAddress,
        uint32 remoteEid,
        address sendLibraryAddress,
        address receiveLibraryAddress,
        UlnConfig memory ulnConfig,
        ExecutorConfig memory executorConfig
    ) public {
        // Configure send library with executor and ULN configs
        SetConfigParam[] memory sendConfigParams = new SetConfigParam[](2);
        sendConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: EXECUTOR_CONFIG_TYPE, config: abi.encode(executorConfig)});
        sendConfigParams[1] =
            SetConfigParam({eid: remoteEid, configType: ULN_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        // Configure receive library with ULN config
        SetConfigParam[] memory receiveConfigParams = new SetConfigParam[](1);
        receiveConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: RECEIVE_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        endpoint.setConfig(contractAddress, sendLibraryAddress, sendConfigParams);
        endpoint.setConfig(contractAddress, receiveLibraryAddress, receiveConfigParams);
    }

    /**
     * @dev Sets the send and receive libraries for LayerZero messaging
     * @param _oapp Address of the OmniApp to configure
     * @param _eid Endpoint ID to configure for
     * @param _sendLib Address of the send library
     * @param _receiveLib Address of the receive library
     */
    function setLibraries(address _oapp, uint32 _eid, address _sendLib, address _receiveLib) public {
        endpoint.setSendLibrary(_oapp, _eid, _sendLib);
        endpoint.setReceiveLibrary(_oapp, _eid, _receiveLib, 0);
    }

    /**
     * @dev Sets enforced options for LayerZero messaging
     *      Applies the same options to both message types (1 and 2)
     * @param options Encoded options for gas and execution parameters
     */
    function setEnforcedParams(bytes memory options) public {
        EnforcedOptionParam[] memory enforcedParams = new EnforcedOptionParam[](2);
        enforcedParams[0] = EnforcedOptionParam({eid: movementEid, msgType: uint16(1), options: options});
        enforcedParams[1] = EnforcedOptionParam({eid: movementEid, msgType: uint16(2), options: options});
        move2.setEnforcedOptions(enforcedParams);
    }
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MovementRestakedV1 is ERC20 {

    address public admin;
    mapping (address => mapping (string => bool)) public permissions;
    string[] public permissionsList = ["admin", "airdrop", "openMarket", "migrate"];

    bool public isOpen = false;

    uint256 public startEpoch;
    uint256 public epochDuration = 1 days;

    mapping(address => mapping(uint256 => uint256)) public lockedBalancesByEpoch;

    event Bought(address indexed buyer, uint256 amount);
    event Sold(address indexed seller, uint256 amount);
    event Funded(address indexed funder, uint256 amount);

    constructor(
        string memory _name,
        string memory _symbol,
        uint256 initialSupply
    ) ERC20(_name, _symbol) {
        _mint(address(this), initialSupply);
        admin = msg.sender;
        for (uint256 i = 0; i < permissionsList.length; i++) {
            permissions[msg.sender][permissionsList[i]] = true;
        }
        startEpoch = block.timestamp / epochDuration;
    }

    function requirePermissions(string[] memory _permissions) view internal {
        for (uint256 i = 0; i < _permissions.length; i++) {
            require(permissions[msg.sender][_permissions[i]], "Permission required");
        }
    }   

    function addPermissions(address[] calldata delegates, string[] calldata _permissions) external {
        string[] memory perms = new string[](1);
        perms[0] = "admin";
        requirePermissions(perms);
        for (uint256 i = 0; i < delegates.length; i++) {
            for (uint256 j = 0; j < _permissions.length; j++) {
                permissions[delegates[i]][_permissions[j]] = true;
            }
        }
    }

    function buy() external payable {

        // Custom logic: Ensure the contract is open
        require(isOpen, "MOVE market is not open");

        // Transfer the amount of MOVE from the contract to the sender
        // This already handles erroring on insufficient balance
        transferFrom(address(this), msg.sender, msg.value);

        // Custom logic: Emit an event
        emit Bought(msg.sender, msg.value);
    }

    function sell(uint256 amount) external {

        // Custom logic: Ensure the contract is open
        require(isOpen, "MOVE market is not open");

        // Check if the user has enought MOVE to sell
        require(balanceOf(msg.sender) >= amount, "Insufficient MOVE balance");

        // Now just pay the user
        // ! This should always work because the contract never spends MOVE outside of this function
        payable(msg.sender).transfer(amount);

        // Custom logic: Emit an event
        emit Sold(msg.sender, amount);
    }

    // Simply allows the addition of ETH to the contract
    function fund() external payable {

        // Custom logic: Emit an event
        emit Funded(msg.sender, msg.value);

    }

    function airdrop(address[] calldata recipients, uint256[] calldata amounts) external {

        string[] memory perms = new string[](1);
        perms[0] = "airdrop";
        requirePermissions(perms);

        // ensure the market it not open
        require(!isOpen, "MOVE market is already open");

        // simply transfer from this contract to the recipients
        for (uint256 i = 0; i < recipients.length; i++) {
            transferFrom(address(this), recipients[i], amounts[i]);
        }
       
    }

    
    function aidropLocked(address[] calldata recipients, uint256[] calldata epochs, uint256[] calldata amounts) external {

        // ensure admin is calling this function
        string[] memory perms = new string[](1);
        perms[0] = "airdrop";
        requirePermissions(perms);

        // ensure the market it not open
        require(!isOpen, "MOVE market is already open");

        // simply transfer from this contract to the recipients
        for (uint256 i = 0; i < recipients.length; i++) {
            lockedBalancesByEpoch[recipients[i]][epochs[i]] = amounts[i];
        }
       
    }

    function claimLockedAirdrops(uint256 fromEpoch, uint256 toEpoch) public {

        // for every epoch from the startEpoch to the current epoch
        for (uint256 i = fromEpoch; i <= toEpoch; i++) {

            // get the locked balance for the epoch
            uint256 lockedBalance = lockedBalancesByEpoch[msg.sender][i];

            // if the locked balance is greater than 0
            if (lockedBalance > 0) {

                // transfer the locked balance to the user
                transferFrom(address(this), msg.sender, lockedBalance);

                // set the locked balance to 0
                lockedBalancesByEpoch[msg.sender][i] = 0;

            }

        }

    }

    function claimAllLockedAirdrops() public {

        // call claimLockedAirdrops with the startEpoch and the current epoch
        claimLockedAirdrops(startEpoch, block.timestamp / epochDuration);

    }

    function openMarket() public {

        // ensure admin is calling this function
        string[] memory perms = new string[](1);
        perms[0] = "openMarket";
        requirePermissions(perms);

        // ensure the market it not open
        require(!isOpen, "MOVE market is already open");

        // set the market to open
        isOpen = true;

    }

    function migrate(address _newContract) public {

        // ensure admin is calling this function
        string[] memory perms = new string[](1);
        perms[0] = "migrate";
        requirePermissions(perms);

        // * TRANSFER STATE USING RECEIVER FUNCTIONS

        // call receiveAdmin on new contact

        // call receivePermissions on new contact

        // call receiveBalances on new contact

        // call receiveLockedBalances on new contact

        // call receiveStartEpoch on new contact

        // call receiveIsOpen on new contact

    }


}

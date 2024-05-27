// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract MovementRestakedV2 is IERC20 {

    string public name;
    string public symbol;
    uint8 public decimals = 18;

    address public admin;
    mapping (address => mapping (string => bool)) public permissions;
    string[] public permissionsList = ["admin", "mint", "airdrop", "openMarket", "migrate"];

    uint256 public totalSupply;

    bool public isOpen = false;

    uint256 public startEpoch;
    uint256 public epochDuration = 1 days;

    // Ownership by address managed by ownership mode
    // Addresses managed will for the most part be the same as the top-level address. This is mainly provided as a means for this contract to stake on behalf of users when they have locked balances.
    // RESERVED MODES:
    // 0: is always standard ownership; no locks can be applied to this mode
    // 1: is a strict locked ownership mode. No funds can be moved from 1 to any other mode until the lock period is over.
    // 2: is zero-interest loan mode. No funds can be moved from 2 to any other mode unless paid for from this contract.
    mapping(address => 
        mapping(address =>
        mapping(uint256 => uint256))) public ownership;
    uint256[] public canonicalOwnershipModes = [0, 1, 2];

    // Locked balances by epoch by ownership mode
    // Notice, there is no managed address here. This is because even when managed by a different address, the locked balances are still owned by the user and should adhere to the original lock schedule.
    mapping(address => 
        mapping(uint256 => 
        mapping(uint256 => uint256))) public unlockAllowanceByEpoch;

    // Describes the ability to transfer between ownership modes
    mapping(uint256 => mapping(uint256 => bool)) public potency;

    event Bought(address indexed buyer, uint256 amount);
    event Sold(address indexed seller, uint256 amount);
    event Funded(address indexed funder, uint256 amount);

    constructor(
        string memory _name,
        string memory _symbol,
        uint256 initialSupply
    )  {

        // name and symbol
        name = _name;
        symbol = _symbol;

        // mint the initial supply to the contract
        mint(address(this), initialSupply);

        // assign the admin and give it all permissions
        admin = msg.sender;
        for (uint256 i = 0; i < permissionsList.length; i++) {
            permissions[msg.sender][permissionsList[i]] = true;
        }

        // set the start epoch
        startEpoch = block.timestamp / epochDuration;

        // potencies...
        // 0 is potent with all modes
        for (uint256 i = 0; i < canonicalOwnershipModes.length; i++) {
           potency[0][canonicalOwnershipModes[i]] = true;
        }

        // 1 can only transfer directly with 1
        potency[1][1] = true;

        // 2 can only transfer directly with 2
        potency[2][2] = true;

    }

    function mint(address to, uint256 amount) internal {
        // ownership mode 0 is always standard ownership
        setBalance(address(this), to, 0, amount);
        totalSupply += amount;
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

    function setBalance(
        address managedBy, 
        address account, 
        uint256 mode, 
        uint256 amount
    ) internal {
        ownership[managedBy][account][mode] = amount;
    }

    function requirePotencyMatch(uint256 fromMode, uint256 toMode) view internal {
        require(potency[fromMode][toMode], "Ownership modes are not potent");
    }

    function transferWithMode(
        address sender, 
        uint256 senderMode,
        address recipient, 
        uint256 recipientMode,
        uint256 amount
    ) internal {

        // check that potency matches
        requirePotencyMatch(senderMode, recipientMode);
      
        // check if the sender has enough balance
        _transferWithMode(sender, senderMode, recipient, recipientMode, amount);
    
    }

    function _transferWithMode(
        address sender, 
        uint256 senderMode,
        address recipient, 
        uint256 recipientMode,
        uint256 amount
    ) internal {
      
        _transferManagedWithMode(
            sender, 
            sender, 
            senderMode, 
            recipient, 
            recipient, 
            recipientMode, 
            amount
        );

    }

    function transferManagedWithMode(
        address senderManagedBy,
        address sender, 
        uint256 senderMode,
        address recipientManagedBy,
        address recipient,
        uint256 recipientMode,
        uint256 amount
    ) internal {

        // check that potency matches
        requirePotencyMatch(senderMode, recipientMode);

        // check if the sender has enough balance
        _transferManagedWithMode(
            senderManagedBy, 
            sender, 
            senderMode, 
            recipientManagedBy, 
            recipient, 
            recipientMode, 
            amount
        );

    }

    function _transferManagedWithMode(
        address senderManagedBy,
        address sender, 
        uint256 senderMode,
        address recipientManagedBy,
        address recipient,
        uint256 recipientMode,
        uint256 amount
    ) internal {

        // check if the sender has enough balance
        require(ownership[senderManagedBy][sender][senderMode] >= amount, "Insufficient balance");

        // perform the transfer
        ownership[senderManagedBy][sender][senderMode] -= amount;
        ownership[recipientManagedBy][recipient][recipientMode] += amount;

        // todo: do the locks need to travel?
        // ! if they don't, then if this a locked mode only the original sender can unlock this balance, so to speak. 
        // ! This could be desirable, and instead setsup locking as more of a general account restriction, rather than a restriction on certain tokens.
        // ! There is on concern about a locked user using a second account to unlock the balance, because they would need a token allocation matching the balance on the second account anyways (which should be controlled at a higher level).

    }

    function manageWithMode(
        address manageBy,
        uint256 manageMode,
        address account,
        uint256 accountMode,
        uint256 amount
    ) internal {

        // check that potency matches
        requirePotencyMatch(manageMode, accountMode);

        // check that the account has enough balance
        require(ownership[manageBy][account][accountMode] >= amount, "Insufficient balance");

        // perform the transfer
        ownership[manageBy][account][accountMode] -= amount;
        ownership[account][account][accountMode] += amount;
        
        // todo: do the locks need to travel?

    }

    function addBalance(
        address managedBy, 
        address account, 
        uint256 mode, 
        uint256 amount
    ) internal {
        ownership[managedBy][account][mode] += amount;
    }

    function unlockBalance(
        address account, 
        uint256 mode, 
        uint256 epoch
    ) internal {

        // todo: I think we could remodel some of this in terms of a standard approval flow. 

        // ! if this weren't here, the contract would be able to keep unlocking the same balance and adding to it.
        require(mode != 0, "Cannot unlock standard ownership");

        // get the balance that unlocks
        uint256 unlockAllowance = unlockAllowanceByEpoch[account][mode][epoch];

        // min of unlockedBalance and the actual balance of the user for that ownership mode (managed by themselves) is the amount that can be unlocked
        uint256 unlockedBalance = min(
            unlockAllowance, 
            ownership[account][account][mode]
        );

        // add the unlockedBalance to the standard ownership balance
        ownership[account][account][0] += unlockedBalance;

        // 0 out the unlockAllowance
        unlockAllowanceByEpoch[account][mode][epoch] = 0;

    }

    function realizeOwnership(
        address account, 
        uint256 mode,
        uint256 fromEpoch,
        uint256 toEpoch
    ) internal {
        // if the ownership mode is not 0, unlock the balance
        if (mode != 0) {
            for (uint256 i = fromEpoch; i <= toEpoch; i++) {
                unlockBalance(account, mode, i);
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

    function airdrop(
        address[] calldata recipients, 
        uint256[] calldata amounts,
        uint256[] calldata unlockAllowances,
        uint256[] calldata epochs,
        uint256[] calldata modes
    ) external {

        string[] memory perms = new string[](1);
        perms[0] = "airdrop";
        requirePermissions(perms);

        for (uint256 i = 0; i < recipients.length; i++) {
            // transfer the amount
            transferWithMode(address(this), 0, recipients[i], modes[i], amounts[i]);
            // set the unlock allowance
            unlockAllowanceByEpoch[recipients[i]][modes[i]][epochs[i]] = unlockAllowances[i];
        }
       
    }

    function claimDomainLoan(
        address domain,
        uint256 amount //! place some kind of limit on this
    ) external {

        // transfer the amount
        transferWithMode(address(this), 0, domain, 2, amount);

    }

    // Domain pays back the loan to this contract and properly rewards its users
    function payLoanAndReward(
        address domain,
        address[] calldata accounts,
        uint256[] calldata modes
        // todo: you could also have these here, though you might want to do accounts, amounts, modes, then these
        // address[] calldata unlockAllowances,
        // address[] calldata epochs
    ) external {

        // for each account
        for (uint256 i = 0; i < accounts.length; i++) {

            // get the amount of the loaned reward
            uint256 amount = ownership[domain][accounts[i]][2];

            // transfer that amount from the domain back to this account on the loan mode
            transferWithMode(domain, 2, accounts[i], 2, amount);

            // accept the loan repayment
            acceptLoanRepayment();

            // transfer the amount to the account on the specified mode
            transferWithMode(address(this), 0, accounts[i], modes[i], amount);

        }

    }

    function acceptLoanRepayment() internal {

        // get all of the amount in the loan mode
        uint256 amount = ownership[address(this)][address(this)][2];

        //! bypass the potency check and just transfer the amount
        _transferWithMode(address(this), 2, address(this), 0, amount);

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

    }

    // IERC20 functions
    // name, symbol, and totalSupply are already defined
    function balanceOf(address _owner) public view returns (uint256 balance) {
        return ownership[_owner][_owner][0];
    }

    function transfer(address _to, uint256 _value) public returns (bool success) {
        transferWithMode(msg.sender, 0, _to, 0, _value);
        return true;
    }

    function transferFrom(address _from, address _to, uint256 _value) public returns (bool success) {

        transferManagedWithMode(
            msg.sender,
            _from,
            0,
            _to,
            _to,
            0,
            _value
        );

        emit Transfer(_from, _to, _value);

        return true;
    }


    function approve(address _spender, uint256 _value) public returns (bool success) {

        transferManagedWithMode(
            msg.sender,
            msg.sender,
            0,
            // transfer to the account managed by the spender
            _spender,
            msg.sender,
            0,
            _value
        );

        emit Approval(msg.sender, _spender, _value);

        return true;

    }  


    function allowance(address _owner, address _spender) public view returns (uint256 remaining) {

        return ownership[_owner][_spender][0];

    }

    // Helper functions
    function min(uint256 a, uint256 b) internal pure returns (uint256) {
        return a < b ? a : b;
    }

}

// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {EnumerableSet} from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "forge-std/console.sol";
import {MovementStaking, IMovementStaking} from "../staking/MovementStaking.sol";
import {MCRStorage} from "./MCRStorage.sol";
import {BaseSettlement} from "./settlement/BaseSettlement.sol";
import {IMCR} from "./interfaces/IMCR.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract MCR is Initializable, BaseSettlement, MCRStorage, IMCR {

    // A role for setting commitments
    bytes32 public constant COMMITMENT_ADMIN = keccak256("COMMITMENT_ADMIN");

    // Trusted attesters admin
    bytes32 public constant TRUSTED_ATTESTER = keccak256("TRUSTED_ATTESTER");

    function initialize(
        IMovementStaking _stakingContract,
        uint256 _lastAcceptedBlockHeight,
        uint256 _leadingBlockTolerance,
        uint256 _epochDuration,
        address[] memory _custodians
    ) public initializer {
        __BaseSettlement_init_unchained();
        stakingContract = _stakingContract;
        leadingBlockTolerance = _leadingBlockTolerance;
        lastAcceptedBlockHeight = _lastAcceptedBlockHeight;
        stakingContract.registerDomain(_epochDuration, _custodians);
        grantCommitmentAdmin(msg.sender);
        grantTrustedAttester(msg.sender);
    }

    function grantCommitmentAdmin(address account) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        grantRole(COMMITMENT_ADMIN, account);
    }

    function batchGrantCommitmentAdmin(address[] memory accounts) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        for (uint256 i = 0; i < accounts.length; i++) {
            grantRole(COMMITMENT_ADMIN, accounts[i]);
        }
    }

    // creates a commitment
    function createBlockCommitment(
        uint256 height,
        bytes32 commitment,
        bytes32 blockId
    ) public pure returns (BlockCommitment memory) {
        return BlockCommitment(height, commitment, blockId);
    }

    // gets the max tolerable block height
    function getMaxTolerableBlockHeight() public view returns (uint256) {
        return lastAcceptedBlockHeight + leadingBlockTolerance;
    }

    // gets the would be epoch for the current block time
    function getEpochByBlockTime() public view returns (uint256) {
        return stakingContract.getEpochByBlockTime(address(this));
    }

    // gets the current epoch up to which blocks have been accepted
    function getCurrentEpoch() public view returns (uint256) {
        return stakingContract.getCurrentEpoch(address(this));
    }

    // gets the next epoch
    function getNextEpoch() public view returns (uint256) {
        return stakingContract.getNextEpoch(address(this));
    }

    // gets the stake for a given attester at a given epoch
    function getStakeAtEpoch(
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            stakingContract.getStakeAtEpoch(
                address(this),
                epoch,
                custodian,
                attester
            );
    }

    // todo: memoize this
    function computeAllStakeAtEpoch(
        uint256 epoch,
        address attester
    ) public view returns (uint256) {
        address[] memory custodians = stakingContract.getCustodiansByDomain(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getStakeAtEpoch(epoch, custodians[i], attester);
        }
        return totalStake;
    }

    // gets the stake for a given attester at the current epoch
    function getCurrentEpochStake(
        address custodian,
        address attester
    ) public view returns (uint256) {
        return getStakeAtEpoch(getCurrentEpoch(), custodian, attester);
    }

    function computeAllCurrentEpochStake(
        address attester
    ) public view returns (uint256) {
        return computeAllStakeAtEpoch(getCurrentEpoch(), attester);
    }

    // gets the total stake for a given epoch
    function getTotalStakeForEpoch(
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return
            stakingContract.getTotalStakeForEpoch(
                address(this),
                epoch,
                custodian
            );
    }

    function acceptGenesisCeremony() public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ACCEPT_GENESIS_CEREMONY_IS_ADMIN_ONLY"
        );
        stakingContract.acceptGenesisCeremony();
    }

    function computeAllTotalStakeForEpoch(
        uint256 epoch
    ) public view returns (uint256) {
        address[] memory custodians = stakingContract.getCustodiansByDomain(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getTotalStakeForEpoch(epoch, custodians[i]);
        }
        return totalStake;
    }

    // gets the total stake for the current epoch
    function getTotalStakeForCurrentEpoch(
        address custodian
    ) public view returns (uint256) {
        return getTotalStakeForEpoch(getCurrentEpoch(), custodian);
    }

    function computeAllTotalStakeForCurrentEpoch()
        public
        view
        returns (uint256)
    {
        return computeAllTotalStakeForEpoch(getCurrentEpoch());
    }

    function getValidatorCommitmentAtBlockHeight(
        uint256 height,
        address attester
    ) public view returns (BlockCommitment memory) {
        return commitments[height][attester];
    }

    // Sets the accepted commitment at a give block height
    function setAcceptedCommitmentAtBlockHeight(BlockCommitment memory blockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_ACCEPTED_COMMITMENT_AT_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        versionedAcceptedBlocks[acceptedBlocksVersion][blockCommitment.height] = blockCommitment;  
    }

    // Sets the last accepted block height. 
    function setLastAcceptedBlockHeight(uint256 height) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_ACCEPTED_BLOCK_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        lastAcceptedBlockHeight = height;
    }

    // Forces the latest attestation by setting the block height
    // Note: this only safe when we are running with a single validator as it does not zero out follow-on commitments.
    function forceLatestCommitment(BlockCommitment memory blockCommitment) public {
        /*require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "FORCE_LATEST_COMMITMENT_IS_COMMITMENT_ADMIN_ONLY"
        );*/

        // increment the acceptedBlocksVersion (effectively removing all other accepted blocks)
        acceptedBlocksVersion += 1;
        versionedAcceptedBlocks[acceptedBlocksVersion][blockCommitment.height] = blockCommitment;
        lastAcceptedBlockHeight = blockCommitment.height; 
    }

    function getAcceptedCommitmentAtBlockHeight(uint256 height) public view returns (BlockCommitment memory) {
        return versionedAcceptedBlocks[acceptedBlocksVersion][height];
    }

    function getAttesters() public view returns (address[] memory) {
        return stakingContract.getAttestersByDomain(address(this));
    }

    /**
     * @dev submits a block commitment for an attester.
     */
    function submitBlockCommitmentForAttester(
        address attester,
        BlockCommitment memory blockCommitment
    ) internal {
        // Attester has already committed to a block at this height
        if (commitments[blockCommitment.height][attester].height != 0)
            revert AttesterAlreadyCommitted();

        // note: do no uncomment the below, we want to allow this in case we have lagging attesters
        // Attester has committed to an already accepted block
        // if ( lastAcceptedBlockHeight > blockCommitment.height) revert AlreadyAcceptedBlock();
        // Attester has committed to a block too far ahead of the last accepted block
        if (
            lastAcceptedBlockHeight + leadingBlockTolerance <
            blockCommitment.height
        ) revert AttesterAlreadyCommitted();

        // assign the block height to the current epoch if it hasn't been assigned yet
        if (blockHeightEpochAssignments[blockCommitment.height] == 0) {
            // note: this is an intended race condition, but it is benign because of the tolerance
            blockHeightEpochAssignments[
                blockCommitment.height
            ] = getEpochByBlockTime();
        }

        // register the attester's commitment
        commitments[blockCommitment.height][attester] = blockCommitment;

        // increment the commitment count by stake
        uint256 allCurrentEpochStake = computeAllCurrentEpochStake(attester);
        commitmentStakes[blockCommitment.height][
            blockCommitment.commitment
        ] += allCurrentEpochStake;

        emit BlockCommitmentSubmitted(
            blockCommitment.blockId,
            blockCommitment.commitment,
            allCurrentEpochStake
        );

        // keep ticking through to find accepted blocks
        // note: this is what allows for batching to be successful
        // we can commit to blocks out to the tolerance point
        // then we can accept them in order
        // ! however, this does potentially become very costly for whomever submits this last block
        // ! rewards need to be managed accordingly
        while (tickOnBlockHeight(lastAcceptedBlockHeight + 1)) {}
    }

    /**
     */
    function tickOnBlockHeight(uint256 blockHeight) internal returns (bool) {
        // get the epoch assigned to the block height
        uint256 blockEpoch = blockHeightEpochAssignments[blockHeight];

        // if the current epoch is far behind, that's okay that just means there weren't blocks submitted
        // so long as we ensure that we go through the blocks in order and that the block to epoch assignment is non-decreasing, we're good
        // so, we'll just keep rolling over the epoch until we catch up
        while (getCurrentEpoch() < blockEpoch) {
            rollOverEpoch();
        }

        // note: we could keep track of seen commitments in a set
        // but since the operations we're doing are very cheap, the set actually adds overhead
        uint256 supermajority = (2 * computeAllTotalStakeForEpoch(blockEpoch)) /
            3;
        address[] memory attesters = getAttesters();

        // iterate over the attester set
        for (uint256 i = 0; i < attesters.length; i++) {
            address attester = attesters[i];

            // get a commitment for the attester at the block height
            BlockCommitment memory blockCommitment = commitments[blockHeight][
                attester
            ];

            // check the total stake on the commitment
            uint256 totalStakeOnCommitment = commitmentStakes[
                blockCommitment.height
            ][blockCommitment.commitment];

            if (totalStakeOnCommitment > supermajority) {
                // accept the block commitment (this may trigger a roll over of the epoch)
                _acceptBlockCommitment(blockCommitment);

                // we found a commitment that was accepted
                return true;
            }
        }

        return false;
    }

    function grantTrustedAttester(address attester) public onlyRole(COMMITMENT_ADMIN) {
        grantRole(TRUSTED_ATTESTER, attester);
    }

    function batchGrantTrustedAttester(address[] memory attesters) public onlyRole(COMMITMENT_ADMIN) {
        for (uint256 i = 0; i < attesters.length; i++) {
            grantRole(TRUSTED_ATTESTER, attesters[i]);
        }

    }

    function setOpenAttestationEnabled(bool enabled) public onlyRole(COMMITMENT_ADMIN) {
        openAttestationEnabled = enabled;
    }

    function submitBlockCommitment(BlockCommitment memory blockCommitment) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_BLOCK_COMMITMENT"
        );
        submitBlockCommitmentForAttester(msg.sender, blockCommitment);
    }

    function submitBatchBlockCommitment(BlockCommitment[] memory blockCommitments) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_BLOCK_COMMITMENT"
        );
        for (uint256 i = 0; i < blockCommitments.length; i++) {
            submitBlockCommitmentForAttester(msg.sender, blockCommitments[i]);
        }
    }

    /**
     * @dev Accepts a block commitment.
     * @dev Under the current implementation this shares in recursion with the tickOnBlockHeight, so it should be reentrant.
     */
    function _acceptBlockCommitment(
        BlockCommitment memory blockCommitment
    ) internal {
        uint256 currentEpoch = getCurrentEpoch();
        // get the epoch for the block commitment
        //  Block commitment is not in the current epoch, it cannot be accepted. This indicates a bug in the protocol.
        if (blockHeightEpochAssignments[blockCommitment.height] != currentEpoch)
            revert UnacceptableBlockCommitment();

        // set accepted block commitment
        versionedAcceptedBlocks[acceptedBlocksVersion][blockCommitment.height] = blockCommitment;

        // set last accepted block height
        lastAcceptedBlockHeight = blockCommitment.height;

        // slash minority attesters w.r.t. to the accepted block commitment
        slashMinority(blockCommitment);

        // emit the block accepted event
        emit BlockAccepted(
            blockCommitment.blockId,
            blockCommitment.commitment,
            blockCommitment.height
        );

        // if the timestamp epoch is greater than the current epoch, roll over the epoch
        if (getEpochByBlockTime() > currentEpoch) {
            rollOverEpoch();
        }
    }

    /**
     */
    function slashMinority(BlockCommitment memory blockCommitment) internal {
        // stakingContract.slash(custodians, attesters, amounts, refundAmounts);
    }

    /**
     * @dev nonReentrant because there is no need to reenter this function. It should be called iteratively. Marked on the internal method to simplify risks from complex calling patterns. This also calls an external contract.
     */
    function rollOverEpoch() internal {
        stakingContract.rollOverEpoch();
    }
}

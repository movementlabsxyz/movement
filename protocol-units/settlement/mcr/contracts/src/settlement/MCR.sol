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
        uint256 _lastAcceptedSuperBlockHeight,
        uint256 _leadingSuperBlockTolerance,
        uint256 _epochDuration,
        address[] memory _custodians
    ) public initializer {
        __BaseSettlement_init_unchained();
        stakingContract = _stakingContract;
        leadingSuperBlockTolerance = _leadingSuperBlockTolerance;
        lastAcceptedSuperBlockHeight = _lastAcceptedSuperBlockHeight;
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
    function createSuperBlockCommitment(
        uint256 height,
        bytes32 commitment,
        bytes32 blockId
    ) public pure returns (SuperBlockCommitment memory) {
        return SuperBlockCommitment(height, commitment, blockId);
    }

    // gets the max tolerable superBlock height
    function getMaxTolerableSuperBlockHeight() public view returns (uint256) {
        return lastAcceptedSuperBlockHeight + leadingSuperBlockTolerance;
    }

    // gets the would be epoch for the current L1Block time
    function getPresentEpoch() public view returns (uint256) {
        return stakingContract.getEpochByL1BlockTime(address(this));
    }

    // gets the epoch up to which superBlocks have been accepted
    function getAcceptingEpoch() public view returns (uint256) {
        return stakingContract.getAcceptingEpoch(address(this));
    }

    // gets the next epoch
    function getNextAcceptingEpoch() public view returns (uint256) {
        return stakingContract.getNextAcceptingEpoch(address(this));
    }

    // gets the stake for a given attester at a given epoch
    function getStake(
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            stakingContract.getStake(
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
            totalStake += getStake(epoch, custodians[i], attester);
        }
        return totalStake;
    }

    // gets the stake for a given attester at the current epoch
    function getStakeForAcceptingEpoch(
        address custodian,
        address attester
    ) public view returns (uint256) {
        return getStake(getAcceptingEpoch(), custodian, attester);
    }

    function computeAllStakeFromAcceptingEpoch(
        address attester
    ) public view returns (uint256) {
        return computeAllStakeAtEpoch(getAcceptingEpoch(), attester);
    }

    // gets the total stake for a given epoch
    function getCustodianStake(
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return
            stakingContract.getCustodianStake(
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
            totalStake += getCustodianStake(epoch, custodians[i]);
        }
        return totalStake;
    }

    // gets the total stake for the current epoch for a given custodian
    function getCustodianStakeForAcceptingEpoch(
        address custodian
    ) public view returns (uint256) {
        return getCustodianStake(getAcceptingEpoch(), custodian);
    }

    function computeAllTotalStakeForAcceptingEpoch()
        public
        view
        returns (uint256)
    {
        return computeAllTotalStakeForEpoch(getAcceptingEpoch());
    }

    function getValidatorCommitmentAtSuperBlockHeight(
        uint256 height,
        address attester
    ) public view returns (SuperBlockCommitment memory) {
        return commitments[height][attester];
    }

    // Sets the accepted commitment at a given superBlock height
    function setAcceptedCommitmentAtBlockHeight(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_ACCEPTED_COMMITMENT_AT_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        versionedAcceptedSuperBlocks[acceptedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;
        setlastAcceptedSuperBlockHeight(superBlockCommitment.height);
    }

    // Sets the last accepted superBlock height. 
    function setlastAcceptedSuperBlockHeight(uint256 height) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_ACCEPTED_SUPERBLOCK_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        lastAcceptedSuperBlockHeight = height;
    }

    // Forces the latest attestation by setting the superBlock height
    // Note: this only safe when we are running with a single validator as it does not zero out follow-on commitments.
    function forceLatestCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "FORCE_LATEST_COMMITMENT_IS_COMMITMENT_ADMIN_ONLY"
        );
        setAcceptedCommitmentAtBlockHeight(superBlockCommitment);
    }

    function getAcceptedCommitmentAtSuperBlockHeight(uint256 height) public view returns (SuperBlockCommitment memory) {
        return versionedAcceptedSuperBlocks[acceptedSuperBlocksVersion][height];
    }

    function getAttesters() public view returns (address[] memory) {
        return stakingContract.getAttestersByDomain(address(this));
    }

    /**
     * @dev submits a superBlock commitment for an attester.
     */
    function submitSuperBlockCommitmentForAttester(
        address attester,
        SuperBlockCommitment memory superBlockCommitment
    ) internal {
        // Attester has already committed to a superBlock at this height
        if (commitments[superBlockCommitment.height][attester].height != 0)
            revert AttesterAlreadyCommitted();

        // note: do no uncomment the below, we want to allow this in case we have lagging attesters
        // Attester has committed to an already accepted superBlock
        // if ( lastAcceptedSuperBlockHeight > superBlockCommitment.height) revert AlreadyAcceptedSuperBlock();
        // Attester has committed to a superBlock too far ahead of the last accepted superBlock
        if (
            lastAcceptedSuperBlockHeight + leadingSuperBlockTolerance <
            superBlockCommitment.height
        ) revert AttesterAlreadyCommitted();

        // assign the superBlock height to the current epoch if it hasn't been assigned yet
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] == 0) {
            // note: this is an intended race condition, but it is benign because of the tolerance
            superBlockHeightAssignedEpoch[
                superBlockCommitment.height
            ] = getPresentEpoch();
        }

        // register the attester's commitment
        commitments[superBlockCommitment.height][attester] = superBlockCommitment;

        // increment the commitment count by stake
        uint256 allStakeFromAcceptingEpoch = computeAllStakeFromAcceptingEpoch(attester);
        commitmentStakes[superBlockCommitment.height][
            superBlockCommitment.commitment
        ] += allStakeFromAcceptingEpoch;

        emit SuperBlockCommitmentSubmitted(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            allStakeFromAcceptingEpoch
        );

        // keep ticking through to find accepted superBlocks
        // note: this is what allows for batching to be successful
        // we can commit to superBlocks out to the tolerance point
        // then we can accept them in order
        // ! however, this does potentially become very costly for whomever submits this last superBlock
        // ! rewards need to be managed accordingly
        while (tickOnSuperBlockHeight(lastAcceptedSuperBlockHeight + 1)) {}
    }

    /**
     */
    function tickOnSuperBlockHeight(uint256 superBlockHeight) internal returns (bool) {
        // get the epoch assigned to the superBlock height
        uint256 superBlockEpoch = superBlockHeightAssignedEpoch[superBlockHeight];

        // if the current epoch is far behind, that's okay that just means there weren't superBlocks submitted
        // so long as we ensure that we go through the superBlocks in order and that the superBlock to epoch assignment is non-decreasing, we're good
        // so, we'll just keep rolling over the epoch until we catch up
        while (getAcceptingEpoch() < superBlockEpoch) {
            rollOverEpoch();
        }

        // note: we could keep track of seen commitments in a set
        // but since the operations we're doing are very cheap, the set actually adds overhead
        uint256 supermajority = (2 * computeAllTotalStakeForEpoch(superBlockEpoch)) /
            3;
        address[] memory attesters = getAttesters();

        // iterate over the attester set
        for (uint256 i = 0; i < attesters.length; i++) {
            address attester = attesters[i];

            // get a commitment for the attester at the superBlock height
            SuperBlockCommitment memory superBlockCommitment = commitments[superBlockHeight][
                attester
            ];

            // check the total stake on the commitment
            uint256 totalStakeOnCommitment = commitmentStakes[
                superBlockCommitment.height
            ][superBlockCommitment.commitment];

            if (totalStakeOnCommitment > supermajority) {
                // accept the superBlock commitment (this may trigger a roll over of the epoch)
                _acceptSuperBlockCommitment(superBlockCommitment);

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

    function submitSuperBlockCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitment);
    }

    function submitBatchSuperBlockCommitment(SuperBlockCommitment[] memory superBlockCommitments) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        for (uint256 i = 0; i < superBlockCommitments.length; i++) {
            submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitments[i]);
        }
    }

    /**
     * @dev Accepts a superBlock commitment.
     * @dev Under the current implementation this shares in recursion with the tickOnSuperBlockHeight, so it should be reentrant.
     */
    function _acceptSuperBlockCommitment(
        SuperBlockCommitment memory superBlockCommitment
    ) internal {
        uint256 currentAcceptingEpoch = getAcceptingEpoch();
        // get the epoch for the superBlock commitment
        //  SuperBlock commitment is not in the current epoch, it cannot be accepted. This indicates a bug in the protocol.
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] != currentAcceptingEpoch)
            revert UnacceptableSuperBlockCommitment();

        // set accepted superBlock commitment
        versionedAcceptedSuperBlocks[acceptedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;

        // set last accepted superBlock height
        lastAcceptedSuperBlockHeight = superBlockCommitment.height;

        // slash minority attesters w.r.t. to the accepted superBlock commitment
        slashMinority(superBlockCommitment);

        // emit the superBlock accepted event
        emit BlockAccepted(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            superBlockCommitment.height
        );

        // if the timestamp epoch is greater than the current epoch, roll over the epoch
        if (getPresentEpoch() > currentAcceptingEpoch) {
            rollOverEpoch();
        }
    }

    /**
     */
    function slashMinority(SuperBlockCommitment memory superBlockCommitment) internal {
        // stakingContract.slash(custodians, attesters, amounts, refundAmounts);
    }

    /**
     * @dev nonReentrant because there is no need to reenter this function. It should be called iteratively. Marked on the internal method to simplify risks from complex calling patterns. This also calls an external contract.
     */
    function rollOverEpoch() internal {
        stakingContract.rollOverEpoch();
    }
}

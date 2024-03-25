//! This module implements the various "hooks" that are called by the STF during execution.
//! These hooks can be used to add custom logic at various points in the slot lifecycle:
//! - Before and after each transaction is executed.
//! - At the beginning and end of each batch ("blob")
//! - At the beginning and end of each slot (DA layer block)

use super::runtime::Runtime;
use sov_modules_api::batch::BatchWithId;
use sov_modules_api::hooks::{ApplyBatchHooks, FinalizeHook, SlotHooks, TxHooks};
use sov_modules_api::runtime::capabilities::{
    ContextResolver, GasEnforcer, TransactionDeduplicator,
};
use sov_modules_api::transaction::Transaction;
use sov_modules_api::{
    BlobReaderTrait, Context, Gas, DaSpec, Spec, StateCheckpoint, WorkingSet,
    StateReaderAndWriter
};
use sov_modules_stf_blueprint::SequencerOutcome;
use sov_sequencer_registry::SequencerRegistry;
use tracing::info;
use sov_modules_api::namespaces::Accessory;

impl<S: Spec, Da: DaSpec> GasEnforcer<S, Da> for Runtime<S, Da> {
    /// The transaction type that the gas enforcer knows how to parse
    type Tx = Transaction<S>;
    /// Reserves enough gas for the transaction to be processed, if possible.
    fn try_reserve_gas(
        &self,
        tx: &Self::Tx,
        context: &Context<S>,
        gas_price: &<S::Gas as Gas>::Price,
        mut state_checkpoint: StateCheckpoint<S>,
    ) -> Result<WorkingSet<S>, StateCheckpoint<S>> {
        match self
            .bank
            .reserve_gas(tx, gas_price, context.sender(), &mut state_checkpoint)
        {
            Ok(gas_meter) => Ok(state_checkpoint.to_revertable(gas_meter)),
            Err(e) => {
                tracing::debug!("Unable to reserve gas from {}. {}", e, context.sender());
                Err(state_checkpoint)
            }
        }
    }

    /// Refunds any remaining gas to the payer after the transaction is processed.
    fn refund_remaining_gas(
        &self,
        tx: &Self::Tx,
        context: &Context<S>,
        gas_meter: &sov_modules_api::GasMeter<S::Gas>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.bank
            .refund_remaining_gas(tx, gas_meter, context.sender(), state_checkpoint);
    }
}

impl<S: Spec, Da: DaSpec> TransactionDeduplicator<S, Da> for Runtime<S, Da> {
    /// The transaction type that the deduplicator knows how to parse.
    type Tx = Transaction<S>;
    /// Prevents duplicate transactions from running.
    // TODO(@preston-evans98): Use type system to prevent writing to the `StateCheckpoint` during this check
    fn check_uniqueness(
        &self,
        tx: &Self::Tx,
        context: &Context<S>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> Result<(), anyhow::Error> {
        self.accounts
            .check_uniqueness(tx, context, state_checkpoint)
    }

    /// Marks a transaction as having been executed, preventing it from executing again.
    fn mark_tx_attempted(
        &self,
        tx: &Self::Tx,
        _sequencer: &Da::Address,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.accounts.mark_tx_attempted(tx, state_checkpoint);
    }
}

/// Resolves the context for a transaction.
impl<S: Spec, Da: DaSpec> ContextResolver<S, Da> for Runtime<S, Da> {
    /// The transaction type that the resolver knows how to parse.
    type Tx = Transaction<S>;
    /// Resolves the context for a transaction.
    fn resolve_context(
        &self,
        tx: &Self::Tx,
        sequencer: &Da::Address,
        height: u64,
        working_set: &mut StateCheckpoint<S>,
    ) -> Context<S> {
        // TODO(@preston-evans98): This is a temporary hack to get the sequencer address
        // This should be resolved by the sequencer registry during blob selection
        let sequencer = self
            .sequencer_registry
            .resolve_da_address(sequencer, working_set)
            .ok_or(anyhow::anyhow!("Sequencer was no longer registered by the time of context resolution. This is a bug")).unwrap();
        let sender = self.accounts.resolve_sender_address(tx, working_set);
        Context::new(sender, sequencer, height)
    }
}

impl<S: Spec, Da: DaSpec> TxHooks for Runtime<S, Da> {
    type Spec = S;

    fn pre_dispatch_tx_hook(
        &self,
        _tx: &Transaction<Self::Spec>,
        _working_set: &mut WorkingSet<S>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn post_dispatch_tx_hook(
        &self,
        _tx: &Transaction<Self::Spec>,
        _ctx: &Context<S>,
        _working_set: &mut WorkingSet<S>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: Spec, Da: DaSpec> ApplyBatchHooks<Da> for Runtime<S, Da> {
    type Spec = S;
    type BatchResult =
    SequencerOutcome<<<Da as DaSpec>::BlobTransaction as BlobReaderTrait>::Address>;

    fn begin_batch_hook(
        &self,
        batch: &mut BatchWithId,
        sender: &Da::Address,
        working_set: &mut StateCheckpoint<S>,
    ) -> anyhow::Result<()> {
        // Before executing each batch, check that the sender is registered as a sequencer
        self.sequencer_registry
            .begin_batch_hook(batch, sender, working_set)
    }

    fn end_batch_hook(&self, result: Self::BatchResult, state_checkpoint: &mut StateCheckpoint<S>) {
        match result {
            SequencerOutcome::Rewarded(reward) => {
                // TODO: Process reward here or above.
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Rewarded { amount: reward },
                    state_checkpoint,
                )
            }
            SequencerOutcome::Ignored => {}
            SequencerOutcome::Slashed {
                reason,
                sequencer_da_address,
            } => {
                info!(%sequencer_da_address, ?reason, "Slashing sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Slashed {
                        sequencer: sequencer_da_address,
                    },
                    state_checkpoint,
                )
            }
            SequencerOutcome::Penalized(amount) => {
                info!(amount, "Penalizing sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Penalized { amount },
                    state_checkpoint,
                )
            }
        }
    }
}

impl<S: Spec, Da: DaSpec> SlotHooks for Runtime<S, Da> {
    type Spec = S;
    fn begin_slot_hook(
        &self,
        _pre_state_root: <Self::Spec as Spec>::VisibleHash,
        _versioned_working_set: &mut sov_modules_api::VersionedStateReadWriter<StateCheckpoint<Self::Spec>>,
    ) {
    }

    fn end_slot_hook(&self, _working_set: &mut StateCheckpoint<S>) {}
}

impl<S: Spec, Da: sov_modules_api::DaSpec> FinalizeHook for Runtime<S, Da> {
    type Spec = S;

    fn finalize_hook(
        &self,
        _root_hash: S::VisibleHash,
        _accessory_working_set: &mut impl StateReaderAndWriter<Accessory>,
    ) {
    }
}
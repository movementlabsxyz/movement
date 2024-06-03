import EventEmitter from "eventemitter3";
import { TransactionWorkerEvents, TransactionWorker, TransactionWorkerEventsEnum } from "../../transactions/management";
import { InputGenerateTransactionPayloadData, InputGenerateTransactionOptions } from "../../transactions";
import { AptosConfig } from "../aptosConfig";
import { Account } from "../../account";

export class TransactionManagement extends EventEmitter<TransactionWorkerEvents> {
  account!: Account;

  transactionWorker!: TransactionWorker;

  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    super();
    this.config = config;
  }

  /**
   * Internal function to start the transaction worker and
   * listen to worker events
   *
   * @param args.sender The sender account to sign and submit the transaction
   */
  private start(args: { sender: Account }): void {
    const { sender } = args;
    this.account = sender;
    this.transactionWorker = new TransactionWorker(this.config, sender);

    this.transactionWorker.start();
    this.registerToEvents();
  }

  /**
   * Internal function to push transaction data to the transaction worker.
   *
   * @param args.data An array of transaction payloads
   * @param args.options optional. Transaction generation configurations (excluding accountSequenceNumber)
   *
   * TODO - make this public once worker supports adding transactions to existing queue
   */
  private push(args: {
    data: InputGenerateTransactionPayloadData[];
    options?: Omit<InputGenerateTransactionOptions, "accountSequenceNumber">;
  }): void {
    const { data, options } = args;

    for (const d of data) {
      this.transactionWorker.push(d, options);
    }
  }

  /**
   * Internal function to start listening to transaction worker events
   *
   * TODO - should we ask events to listen to as an input?
   */
  private registerToEvents() {
    this.transactionWorker.on(TransactionWorkerEventsEnum.TransactionSent, async (data) => {
      this.emit(TransactionWorkerEventsEnum.TransactionSent, data);
    });
    this.transactionWorker.on(TransactionWorkerEventsEnum.TransactionSendFailed, async (data) => {
      this.emit(TransactionWorkerEventsEnum.TransactionSendFailed, data);
    });
    this.transactionWorker.on(TransactionWorkerEventsEnum.TransactionExecuted, async (data) => {
      this.emit(TransactionWorkerEventsEnum.TransactionExecuted, data);
    });
    this.transactionWorker.on(TransactionWorkerEventsEnum.TransactionExecutionFailed, async (data) => {
      this.emit(TransactionWorkerEventsEnum.TransactionExecutionFailed, data);
    });
    this.transactionWorker.on(TransactionWorkerEventsEnum.ExecutionFinish, async (data) => {
      this.emit(TransactionWorkerEventsEnum.ExecutionFinish, data);
    });
  }

  /**
   * Send batch transactions for a single account.
   *
   * This function uses a transaction worker that receives payloads to be processed
   * and submitted to chain.
   * Note that this process is best for submitting multiple transactions that
   * dont rely on each other, i.e batch funds, batch token mints, etc.
   *
   * If any worker failure, the functions throws an error.
   *
   * @param args.sender The sender account to sign and submit the transaction
   * @param args.data An array of transaction payloads
   * @param args.options optional. Transaction generation configurations (excluding accountSequenceNumber)
   *
   * @return void. Throws if any error
   */
  forSingleAccount(args: {
    sender: Account;
    data: InputGenerateTransactionPayloadData[];
    options?: Omit<InputGenerateTransactionOptions, "accountSequenceNumber">;
  }): void {
    try {
      const { sender, data, options } = args;
      this.start({ sender });

      this.push({ data, options });
    } catch (error: any) {
      throw new Error(`failed to submit transactions with error: ${error}`);
    }
  }
}

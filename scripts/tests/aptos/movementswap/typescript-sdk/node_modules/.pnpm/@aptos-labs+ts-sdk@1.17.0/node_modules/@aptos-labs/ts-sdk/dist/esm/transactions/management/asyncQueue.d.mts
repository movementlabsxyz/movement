/**
 * The AsyncQueue class is an async-aware data structure that provides a queue-like
 * behavior for managing asynchronous tasks or operations.
 * It allows to enqueue items and dequeue them asynchronously.
 * This is not thread-safe but it is async concurrency safe and
 * it does not guarantee ordering for those that call into and await on enqueue.
 */
declare class AsyncQueue<T> {
    readonly queue: T[];
    private pendingDequeue;
    private cancelled;
    /**
     * The enqueue method adds an item to the queue. If there are pending dequeued promises,
     * in the pendingDequeue, it resolves the oldest promise with the enqueued item immediately.
     * Otherwise, it adds the item to the queue.
     *
     * @param item T
     */
    enqueue(item: T): void;
    /**
     * The dequeue method returns a promise that resolves to the next item in the queue.
     * If the queue is not empty, it resolves the promise immediately with the next item.
     * Otherwise, it creates a new promise. The promise's resolve function is stored
     * in the pendingDequeue with a unique counter value as the key.
     * The newly created promise is then returned, and it will be resolved later when an item is enqueued.
     *
     * @returns Promise<T>
     */
    dequeue(): Promise<T>;
    /**
     * The isEmpty method returns whether the queue is empty or not.
     *
     * @returns boolean
     */
    isEmpty(): boolean;
    /**
     * The cancel method cancels all pending promises in the queue.
     * It rejects the promises with a AsyncQueueCancelledError error,
     * ensuring that any awaiting code can handle the cancellation appropriately.
     */
    cancel(): void;
    /**
     * The isCancelled method returns whether the queue is cancelled or not.
     *
     * @returns boolean
     */
    isCancelled(): boolean;
    /**
     * The pendingDequeueLength method returns the length of the pendingDequeue.
     *
     * @returns number
     */
    pendingDequeueLength(): number;
}
declare class AsyncQueueCancelledError extends Error {
}

export { AsyncQueue, AsyncQueueCancelledError };

/**
 * Sleep the current thread for the given amount of time
 * @param timeMs time in milliseconds to sleep
 */
declare function sleep(timeMs: number): Promise<null>;

export { sleep };

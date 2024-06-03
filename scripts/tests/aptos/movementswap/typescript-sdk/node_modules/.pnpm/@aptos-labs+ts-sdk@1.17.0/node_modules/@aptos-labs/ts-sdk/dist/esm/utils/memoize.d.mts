/**
 * A memoize high order function to cache async function response
 *
 * @param func An async function to cache the result of
 * @param key The provided cache key
 * @param ttlMs time-to-live in milliseconds for cached data
 * @returns the cached or latest result
 */
declare function memoizeAsync<T>(func: (...args: any[]) => Promise<T>, key: string, ttlMs?: number): (...args: any[]) => Promise<T>;
/**
 * A memoize high order function to cache function response
 *
 * @param func A function to cache the result of
 * @param key The provided cache key
 * @param ttlMs time-to-live in milliseconds for cached data
 * @returns the cached or latest result
 */
declare function memoize<T>(func: (...args: any[]) => T, key: string, ttlMs?: number): (...args: any[]) => T;

export { memoize, memoizeAsync };

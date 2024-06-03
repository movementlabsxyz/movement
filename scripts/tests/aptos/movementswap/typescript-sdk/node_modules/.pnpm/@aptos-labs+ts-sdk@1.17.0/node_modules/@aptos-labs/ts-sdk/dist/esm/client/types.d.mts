import { AptosRequest } from '../types/index.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

/**
 * The API response type
 *
 * @param status - the response status. i.e. 200
 * @param statusText - the response message
 * @param data the response data
 * @param url the url the request was made to
 * @param headers the response headers
 * @param config (optional) - the request object
 * @param request (optional) - the request object
 */
interface AptosResponse<Req, Res> {
    status: number;
    statusText: string;
    data: Res;
    url: string;
    headers: any;
    config?: any;
    request?: Req;
}
/**
 * The type returned from an API error
 *
 * @param name - the error name "AptosApiError"
 * @param url the url the request was made to
 * @param status - the response status. i.e. 400
 * @param statusText - the response message
 * @param data the response data
 * @param request - the AptosRequest
 */
declare class AptosApiError extends Error {
    readonly url: string;
    readonly status: number;
    readonly statusText: string;
    readonly data: any;
    readonly request: AptosRequest;
    constructor(request: AptosRequest, response: AptosResponse<any, any>, message: string);
}

export { AptosApiError, type AptosResponse };

import { AptosConfig } from '../api/aptosConfig.mjs';
import { AptosResponse } from './types.mjs';
import { MimeType, AnyNumber, ClientConfig } from '../types/index.mjs';
import { AptosApiType } from '../utils/const.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

type GetRequestOptions = {
    /**
     * The config for the API client
     */
    aptosConfig: AptosConfig;
    /**
     * The type of API endpoint to call e.g. fullnode, indexer, etc
     */
    type: AptosApiType;
    /**
     * The name of the API method
     */
    originMethod: string;
    /**
     * The URL path to the API method
     */
    path: string;
    /**
     * The content type of the request body
     */
    contentType?: MimeType;
    /**
     * The accepted content type of the response of the API
     */
    acceptType?: MimeType;
    /**
     * The query parameters for the request
     */
    params?: Record<string, string | AnyNumber | boolean | undefined>;
    /**
     * Specific client overrides for this request to override aptosConfig
     */
    overrides?: ClientConfig;
};
type GetAptosRequestOptions = Omit<GetRequestOptions, "type">;
/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @returns
 */
declare function get<Req extends {}, Res extends {}>(options: GetRequestOptions): Promise<AptosResponse<Req, Res>>;
declare function getAptosFullNode<Req extends {}, Res extends {}>(options: GetAptosRequestOptions): Promise<AptosResponse<Req, Res>>;
declare function paginateWithCursor<Req extends Record<string, any>, Res extends Array<{}>>(options: GetAptosRequestOptions): Promise<Res>;

export { type GetAptosRequestOptions, type GetRequestOptions, get, getAptosFullNode, paginateWithCursor };

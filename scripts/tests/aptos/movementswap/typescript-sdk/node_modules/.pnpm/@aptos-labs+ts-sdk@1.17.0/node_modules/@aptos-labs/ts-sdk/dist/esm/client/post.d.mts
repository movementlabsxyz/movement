import { AptosConfig } from '../api/aptosConfig.mjs';
import { AptosResponse } from './types.mjs';
import { MimeType, AnyNumber, ClientConfig } from '../types/index.mjs';
import { AptosApiType } from '../utils/const.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

type PostRequestOptions = {
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
     * The body of the request, should match the content type of the request
     */
    body?: any;
    /**
     * Specific client overrides for this request to override aptosConfig
     */
    overrides?: ClientConfig;
};
type PostAptosRequestOptions = Omit<PostRequestOptions, "type">;
/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @returns
 */
declare function post<Req extends {}, Res extends {}>(options: PostRequestOptions): Promise<AptosResponse<Req, Res>>;
declare function postAptosFullNode<Req extends {}, Res extends {}>(options: PostAptosRequestOptions): Promise<AptosResponse<Req, Res>>;
declare function postAptosIndexer<Req extends {}, Res extends {}>(options: PostAptosRequestOptions): Promise<AptosResponse<Req, Res>>;
declare function postAptosFaucet<Req extends {}, Res extends {}>(options: PostAptosRequestOptions): Promise<AptosResponse<Req, Res>>;

export { type PostAptosRequestOptions, type PostRequestOptions, post, postAptosFaucet, postAptosFullNode, postAptosIndexer };

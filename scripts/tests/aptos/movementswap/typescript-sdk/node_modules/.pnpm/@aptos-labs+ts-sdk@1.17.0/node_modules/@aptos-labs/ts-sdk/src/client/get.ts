import { AptosConfig } from "../api/aptosConfig";
import { aptosRequest } from "./core";
import { AptosResponse } from "./types";
import { AnyNumber, ClientConfig, MimeType } from "../types";
import { AptosApiType } from "../utils/const";

export type GetRequestOptions = {
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

export type GetAptosRequestOptions = Omit<GetRequestOptions, "type">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @returns
 */
export async function get<Req extends {}, Res extends {}>(
  options: GetRequestOptions,
): Promise<AptosResponse<Req, Res>> {
  const { aptosConfig, overrides, params, contentType, acceptType, path, originMethod, type } = options;
  const url = aptosConfig.getRequestUrl(type);

  return aptosRequest<Req, Res>(
    {
      url,
      method: "GET",
      originMethod,
      path,
      contentType,
      acceptType,
      params,
      overrides: {
        ...aptosConfig.clientConfig,
        ...overrides,
      },
    },
    aptosConfig,
    options.type,
  );
}

export async function getAptosFullNode<Req extends {}, Res extends {}>(
  options: GetAptosRequestOptions,
): Promise<AptosResponse<Req, Res>> {
  const { aptosConfig } = options;

  return get<Req, Res>({
    ...options,
    type: AptosApiType.FULLNODE,
    overrides: {
      ...aptosConfig.clientConfig,
      ...aptosConfig.fullnodeConfig,
      ...options.overrides,
      HEADERS: { ...aptosConfig.clientConfig?.HEADERS, ...aptosConfig.fullnodeConfig?.HEADERS },
    },
  });
}

/// This function is a helper for paginating using a function wrapping an API
export async function paginateWithCursor<Req extends Record<string, any>, Res extends Array<{}>>(
  options: GetAptosRequestOptions,
): Promise<Res> {
  const out: any[] = [];
  let cursor: string | undefined;
  const requestParams = options.params as { start?: string; limit?: number };
  do {
    // eslint-disable-next-line no-await-in-loop
    const response = await get<Req, Res>({
      type: AptosApiType.FULLNODE,
      aptosConfig: options.aptosConfig,
      originMethod: options.originMethod,
      path: options.path,
      params: requestParams,
      overrides: options.overrides,
    });
    /**
     * the cursor is a "state key" from the API perspective. Client
     * should not need to "care" what it represents but just use it
     * to query the next chunk of data.
     */
    cursor = response.headers["x-aptos-cursor"];
    // Now that we have the cursor (if any), we remove the headers before
    // adding these to the output of this function.
    delete response.headers;
    out.push(...response.data);
    requestParams.start = cursor;
  } while (cursor !== null && cursor !== undefined);
  return out as Res;
}

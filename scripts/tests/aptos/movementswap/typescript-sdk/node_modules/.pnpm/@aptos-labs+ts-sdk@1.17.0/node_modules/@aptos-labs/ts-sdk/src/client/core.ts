// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosConfig } from "../api/aptosConfig";
import { AptosApiError, AptosResponse } from "./types";
import { VERSION } from "../version";
import { AnyNumber, AptosRequest, Client, ClientRequest, ClientResponse, MimeType } from "../types";
import { AptosApiType } from "../utils";

/**
 * Meaningful errors map
 */
const errors: Record<number, string> = {
  400: "Bad Request",
  401: "Unauthorized",
  403: "Forbidden",
  404: "Not Found",
  429: "Too Many Requests",
  500: "Internal Server Error",
  502: "Bad Gateway",
  503: "Service Unavailable",
};

/**
 * Given a url and method, sends the request with axios and
 * returns the response.
 */
export async function request<Req, Res>(options: ClientRequest<Req>, client: Client): Promise<ClientResponse<Res>> {
  const { url, method, body, contentType, params, overrides, originMethod } = options;
  const headers: Record<string, string | AnyNumber | boolean | undefined> = {
    ...overrides?.HEADERS,
    "x-aptos-client": `aptos-typescript-sdk/${VERSION}`,
    "content-type": contentType ?? MimeType.JSON,
    "x-aptos-typescript-sdk-origin-method": originMethod,
  };

  if (overrides?.AUTH_TOKEN) {
    headers.Authorization = `Bearer ${overrides?.AUTH_TOKEN}`;
  }
  if (overrides?.API_KEY) {
    headers.Authorization = `Bearer ${overrides?.API_KEY}`;
  }

  /*
   * make a call using the @aptos-labs/aptos-client package
   * {@link https://www.npmjs.com/package/@aptos-labs/aptos-client}
   */
  return client.provider<Req, Res>({
    url,
    method,
    body,
    params,
    headers,
    overrides,
  });
}

/**
 * The main function to use when doing an API request.
 *
 * @param options AptosRequest
 * @param aptosConfig The config information for the SDK client instance
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req extends {}, Res extends {}>(
  options: AptosRequest,
  aptosConfig: AptosConfig,
  apiType: AptosApiType,
): Promise<AptosResponse<Req, Res>> {
  const { url, path } = options;
  const fullUrl = path ? `${url}/${path}` : url;
  const response = await request<Req, Res>({ ...options, url: fullUrl }, aptosConfig.client);

  const result: AptosResponse<Req, Res> = {
    status: response.status,
    statusText: response.statusText!,
    data: response.data,
    headers: response.headers,
    config: response.config,
    request: response.request,
    url: fullUrl,
  };

  // Handle case for `Unauthorized` error (i.e API_KEY error)
  if (result.status === 401) {
    throw new AptosApiError(options, result, `Error: ${result.data}`);
  }

  // to support both fullnode and indexer responses,
  // check if it is an indexer query, and adjust response.data
  if (apiType === AptosApiType.INDEXER) {
    const indexerResponse = result.data as any;
    // Handle Indexer general errors
    if (indexerResponse.errors) {
      throw new AptosApiError(
        options,
        result,
        `Indexer error: ${indexerResponse.errors[0].message}` ??
          `Indexer unhandled Error ${response.status} : ${response.statusText}`,
      );
    }
    result.data = indexerResponse.data as Res;
  }

  if (result.status >= 200 && result.status < 300) {
    return result;
  }

  let errorMessage: string;

  if (result && result.data && "message" in result.data && "error_code" in result.data) {
    errorMessage = JSON.stringify(result.data);
  } else if (result.status in errors) {
    // If it's not an API type, it must come form infra, these are prehandled
    errorMessage = errors[result.status];
  } else {
    // Everything else is unhandled
    errorMessage = `Unhandled Error ${result.status} : ${result.statusText}`;
  }

  // We have to explicitly check for all request types, because if the error is a non-indexer error, but
  // comes from an indexer request (e.g. 404), we'll need to mention it appropriately
  throw new AptosApiError(options, result, `${apiType} error: ${errorMessage}`);
}

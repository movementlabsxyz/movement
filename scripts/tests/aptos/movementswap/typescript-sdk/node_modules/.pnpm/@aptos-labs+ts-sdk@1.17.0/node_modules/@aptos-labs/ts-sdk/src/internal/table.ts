import { AptosConfig } from "../api/aptosConfig";
import { postAptosFullNode } from "../client";
import {
  TableItemRequest,
  LedgerVersionArg,
  PaginationArgs,
  WhereArg,
  OrderByArg,
  GetTableItemsDataResponse,
  GetTableItemsMetadataResponse,
} from "../types";
import { GetTableItemsDataQuery, GetTableItemsMetadataQuery } from "../types/generated/operations";
import { GetTableItemsData, GetTableItemsMetadata } from "../types/generated/queries";
import { TableItemsBoolExp, TableMetadatasBoolExp } from "../types/generated/types";
import { queryIndexer } from "./general";

export async function getTableItem<T>(args: {
  aptosConfig: AptosConfig;
  handle: string;
  data: TableItemRequest;
  options?: LedgerVersionArg;
}): Promise<T> {
  const { aptosConfig, handle, data, options } = args;
  const response = await postAptosFullNode<TableItemRequest, any>({
    aptosConfig,
    originMethod: "getTableItem",
    path: `tables/${handle}/item`,
    params: { ledger_version: options?.ledgerVersion },
    body: data,
  });
  return response.data as T;
}

export async function getTableItemsData(args: {
  aptosConfig: AptosConfig;
  options?: PaginationArgs & WhereArg<TableItemsBoolExp> & OrderByArg<GetTableItemsDataResponse[0]>;
}) {
  const { aptosConfig, options } = args;

  const graphqlQuery = {
    query: GetTableItemsData,
    variables: {
      where_condition: options?.where,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetTableItemsDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getTableItemsData",
  });

  return data.table_items;
}

export async function getTableItemsMetadata(args: {
  aptosConfig: AptosConfig;
  options?: PaginationArgs & WhereArg<TableMetadatasBoolExp> & OrderByArg<GetTableItemsMetadataResponse[0]>;
}): Promise<GetTableItemsMetadataResponse> {
  const { aptosConfig, options } = args;

  const graphqlQuery = {
    query: GetTableItemsMetadata,
    variables: {
      where_condition: options?.where,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetTableItemsMetadataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getTableItemsMetadata",
  });

  return data.table_metadatas;
}

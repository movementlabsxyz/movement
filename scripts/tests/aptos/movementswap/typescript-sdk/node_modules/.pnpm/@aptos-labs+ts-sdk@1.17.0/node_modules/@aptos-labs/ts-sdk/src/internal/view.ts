// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { LedgerVersionArg, MimeType, MoveValue } from "../types";
import { AptosConfig } from "../api/aptosConfig";
import { generateViewFunctionPayload, InputViewFunctionData } from "../transactions";
import { Serializer } from "../bcs";
import { postAptosFullNode } from "../client";

export async function view<T extends Array<MoveValue> = Array<MoveValue>>(args: {
  aptosConfig: AptosConfig;
  payload: InputViewFunctionData;
  options?: LedgerVersionArg;
}): Promise<T> {
  const { aptosConfig, payload, options } = args;
  const viewFunctionPayload = await generateViewFunctionPayload({
    ...payload,
    aptosConfig,
  });

  const serializer = new Serializer();
  viewFunctionPayload.serialize(serializer);
  const bytes = serializer.toUint8Array();

  const { data } = await postAptosFullNode<Uint8Array, MoveValue[]>({
    aptosConfig,
    path: "view",
    originMethod: "view",
    contentType: MimeType.BCS_VIEW_FUNCTION,
    params: { ledger_version: options?.ledgerVersion },
    body: bytes,
  });

  return data as T;
}

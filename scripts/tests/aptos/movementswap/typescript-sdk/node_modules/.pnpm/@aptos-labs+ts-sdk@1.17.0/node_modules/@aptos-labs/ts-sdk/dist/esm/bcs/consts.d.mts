import { Uint8, Uint16, Uint32, Uint64, Uint128, Uint256 } from '../types/index.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

declare const MAX_U8_NUMBER: Uint8;
declare const MAX_U16_NUMBER: Uint16;
declare const MAX_U32_NUMBER: Uint32;
declare const MAX_U64_BIG_INT: Uint64;
declare const MAX_U128_BIG_INT: Uint128;
declare const MAX_U256_BIG_INT: Uint256;

export { MAX_U128_BIG_INT, MAX_U16_NUMBER, MAX_U256_BIG_INT, MAX_U32_NUMBER, MAX_U64_BIG_INT, MAX_U8_NUMBER };

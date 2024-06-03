import { Serializable, Serializer } from '../../bcs/serializer.mjs';
import { Deserializer } from '../../bcs/deserializer.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';
import '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';

/**
 * Representation of a ChainId that can serialized and deserialized
 */
declare class ChainId extends Serializable {
    readonly chainId: number;
    constructor(chainId: number);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): ChainId;
}

export { ChainId };

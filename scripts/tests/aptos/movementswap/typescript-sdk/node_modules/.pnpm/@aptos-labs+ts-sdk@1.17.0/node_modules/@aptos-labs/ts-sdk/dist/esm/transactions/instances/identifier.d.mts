import { Deserializer } from '../../bcs/deserializer.mjs';
import { Serializable, Serializer } from '../../bcs/serializer.mjs';
import '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';

/**
 * Representation of an Identifier that can serialized and deserialized.
 * We use Identifier to represent the module "name" in "ModuleId" and
 * the "function name" in "EntryFunction"
 */
declare class Identifier extends Serializable {
    identifier: string;
    constructor(identifier: string);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Identifier;
}

export { Identifier };

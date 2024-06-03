import { Serializable, Serializer } from '../../bcs/serializer.mjs';
import { Deserializer } from '../../bcs/deserializer.mjs';
import { AccountAddress } from '../../core/accountAddress.mjs';
import { MoveModuleId } from '../../types/index.mjs';
import { Identifier } from './identifier.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import './transactionArgument.mjs';

/**
 * Representation of a ModuleId that can serialized and deserialized
 * ModuleId means the module address (e.g "0x1") and the module name (e.g "coin")
 */
declare class ModuleId extends Serializable {
    readonly address: AccountAddress;
    readonly name: Identifier;
    /**
     * Full name of a module.
     * @param address The account address. e.g "0x1"
     * @param name The module name under the "address". e.g "coin"
     */
    constructor(address: AccountAddress, name: Identifier);
    /**
     * Converts a string literal to a ModuleId
     * @param moduleId String literal in format "account_address::module_name", e.g. "0x1::coin"
     * @returns ModuleId
     */
    static fromStr(moduleId: MoveModuleId): ModuleId;
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): ModuleId;
}

export { ModuleId };

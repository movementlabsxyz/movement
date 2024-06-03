import { Deserializer } from '../deserializer.mjs';
import { Serializable, Serializer } from '../serializer.mjs';
import { TransactionArgument } from '../../transactions/instances/transactionArgument.mjs';
import { Uint8, Uint16, Uint32, AnyNumber } from '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';

declare class Bool extends Serializable implements TransactionArgument {
    readonly value: boolean;
    constructor(value: boolean);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Bool;
}
declare class U8 extends Serializable implements TransactionArgument {
    readonly value: Uint8;
    constructor(value: Uint8);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U8;
}
declare class U16 extends Serializable implements TransactionArgument {
    readonly value: Uint16;
    constructor(value: Uint16);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U16;
}
declare class U32 extends Serializable implements TransactionArgument {
    readonly value: Uint32;
    constructor(value: Uint32);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U32;
}
declare class U64 extends Serializable implements TransactionArgument {
    readonly value: bigint;
    constructor(value: AnyNumber);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U64;
}
declare class U128 extends Serializable implements TransactionArgument {
    readonly value: bigint;
    constructor(value: AnyNumber);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U128;
}
declare class U256 extends Serializable implements TransactionArgument {
    readonly value: bigint;
    constructor(value: AnyNumber);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): U256;
}

export { Bool, U128, U16, U256, U32, U64, U8 };

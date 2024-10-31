CREATE TABLE bridge_transfers (
    id SERIAL PRIMARY KEY,
    source_chain VARCHAR NOT NULL,
    source_address VARCHAR NOT NULL,
    destination_chain VARCHAR NOT NULL,
    destination_address VARCHAR NOT NULL,
    bridge_transfer_id VARCHAR NOT NULL UNIQUE,
    hash_lock VARCHAR NOT NULL,
    amount DECIMAL NOT NULL
);

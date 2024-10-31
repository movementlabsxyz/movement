CREATE TABLE lock_bridge_transfers (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL,
    hash_lock BYTEA NOT NULL,
    initiator BYTEA NOT NULL,      -- Address stored as bytes
    recipient BYTEA NOT NULL,      -- Address stored as bytes
    amount NUMERIC NOT NULL        -- Using NUMERIC to avoid floating-point issues
);

CREATE TABLE wait_and_complete_initiators (
    id SERIAL PRIMARY KEY,
    wait_time_secs BIGINT NOT NULL,     -- u64 field
    pre_image BYTEA NOT NULL       -- Pre-image of the hash lock, stored as bytes
);

CREATE TABLE initiated_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL,
    initiator_address BYTEA NOT NULL,
    recipient_address BYTEA NOT NULL,
    hash_lock BYTEA NOT NULL,
    time_lock BIGINT NOT NULL,
    amount NUMERIC NOT NULL,
    state SMALLINT NOT NULL
);

CREATE TABLE locked_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL,
    initiator BYTEA NOT NULL,         -- Initiator address as bytes
    recipient BYTEA NOT NULL,         -- Recipient address as bytes
    hash_lock BYTEA NOT NULL,
    time_lock BIGINT NOT NULL,
    amount NUMERIC NOT NULL
);

CREATE TABLE initiator_completed_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL
);

CREATE TABLE counter_part_completed_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL,
    pre_image BYTEA NOT NULL          -- Pre-image of the hash lock
);

CREATE TABLE cancelled_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL
);

CREATE TABLE refunded_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id BYTEA NOT NULL
);

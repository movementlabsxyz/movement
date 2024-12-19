CREATE TABLE lock_bridge_transfers (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    hash_lock VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,      -- Address stored as bytes
    recipient VARCHAR(64) NOT NULL,      -- Address stored as bytes
    amount NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE wait_and_complete_initiators (
    id SERIAL PRIMARY KEY,
    wait_time_secs BIGINT NOT NULL,     -- u64 field
    pre_image VARCHAR(64) NOT NULL,       -- Pre-image of the hash lock, stored as bytes
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE initiated_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,
    recipient VARCHAR(64) NOT NULL,
    hash_lock VARCHAR(64) NOT NULL,
    time_lock BIGINT NOT NULL,
    amount NUMERIC NOT NULL,
    state SMALLINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE locked_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,         -- Initiator address as bytes
    recipient VARCHAR(64) NOT NULL,         -- Recipient address as bytes
    hash_lock VARCHAR(64) NOT NULL,
    time_lock BIGINT NOT NULL,
    amount NUMERIC NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE initiator_completed_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE counter_party_completed_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    pre_image VARCHAR(64) NOT NULL,          -- Pre-image of the hash lock
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE cancelled_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE refunded_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE initiated_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,      -- Address stored as bytes
    recipient VARCHAR(64) NOT NULL,      -- Address stored as bytes
    amount NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    nonce NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE completed_events (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,
    recipient VARCHAR(64) NOT NULL,
    amount NUMERIC NOT NULL,
    nonce NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE complete_bridge_transfers (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,
    recipient VARCHAR(64) NOT NULL,
    amount NUMERIC NOT NULL,
    nonce NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE completed_remove_state (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE abort_replay_transfers (
    id SERIAL PRIMARY KEY,
    bridge_transfer_id VARCHAR(64) NOT NULL,
    initiator VARCHAR(64) NOT NULL,
    recipient VARCHAR(64) NOT NULL,
    amount NUMERIC NOT NULL,
    nonce NUMERIC NOT NULL,        -- Using NUMERIC to avoid floating-point issues
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);


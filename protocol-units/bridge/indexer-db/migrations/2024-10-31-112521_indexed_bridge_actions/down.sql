-- This file should undo anything in `up.sql`
DROP TABLE refunded_events;
DROP TABLE cancelled_events;
DROP TABLE counter_part_completed_events;
DROP TABLE initiator_completed_events;
DROP TABLE locked_events;
DROP TABLE initiated_events;
DROP TABLE wait_and_complete_initiators;
DROP TABLE lock_bridge_transfers;
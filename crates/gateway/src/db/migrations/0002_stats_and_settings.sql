-- Legacy results do not have a recorded receipt time.
ALTER TABLE results ADD COLUMN received_at_ms INTEGER;

CREATE TABLE player_settings (
    guest_id TEXT PRIMARY KEY NOT NULL REFERENCES guests (id) ON DELETE CASCADE,
    settings TEXT NOT NULL CHECK (json_valid(settings) AND json_type(settings) = 'object')
);

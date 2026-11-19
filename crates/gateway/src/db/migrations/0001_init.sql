CREATE TABLE guests (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 32),
    nick TEXT NOT NULL
);

CREATE TABLE matches (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 32),
    registration TEXT NOT NULL CHECK (json_valid(registration)),
    ticket_key BLOB NOT NULL CHECK (length(ticket_key) = 32),
    cap INTEGER NOT NULL CHECK (cap BETWEEN 0 AND 255),
    phase TEXT NOT NULL DEFAULT 'lobby',
    time_left_s INTEGER NOT NULL DEFAULT 0,
    humans INTEGER NOT NULL DEFAULT 0,
    bots INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_ms INTEGER NOT NULL,
    ended INTEGER NOT NULL DEFAULT 0,
    next_player INTEGER NOT NULL DEFAULT 0 CHECK (next_player BETWEEN 0 AND 127)
);

CREATE INDEX matches_live ON matches (ticket_key, last_heartbeat_ms) WHERE ended = 0;

CREATE TABLE results (
    match_id TEXT PRIMARY KEY NOT NULL REFERENCES matches (id),
    payload TEXT NOT NULL CHECK (json_valid(payload))
);

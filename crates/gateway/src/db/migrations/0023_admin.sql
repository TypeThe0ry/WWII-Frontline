ALTER TABLE matches ADD COLUMN listed INTEGER NOT NULL DEFAULT 1 CHECK (listed IN (0, 1));

CREATE TABLE moderation_tickets (
    match_id TEXT NOT NULL REFERENCES matches (id),
    jti TEXT NOT NULL,
    player_id INTEGER NOT NULL CHECK (player_id BETWEEN 0 AND 127),
    guest_id TEXT NOT NULL REFERENCES guests (id),
    PRIMARY KEY (match_id, jti)
);
CREATE INDEX moderation_tickets_slot ON moderation_tickets (match_id, player_id, guest_id);
CREATE INDEX moderation_tickets_guest ON moderation_tickets (guest_id, match_id, player_id);

CREATE TABLE reports (
    id INTEGER PRIMARY KEY,
    match_id TEXT NOT NULL REFERENCES matches (id),
    reporter_id TEXT NOT NULL REFERENCES guests (id),
    target_id TEXT NOT NULL REFERENCES guests (id),
    reason INTEGER NOT NULL CHECK (reason BETWEEN 0 AND 3),
    note TEXT NOT NULL,
    replay_ref TEXT,
    chat_excerpt TEXT CHECK (json_valid(chat_excerpt)),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    created_at INTEGER NOT NULL,
    UNIQUE (match_id, reporter_id, target_id, reason)
);
CREATE INDEX reports_target_status ON reports (target_id, status);
CREATE INDEX reports_reporter_time ON reports (reporter_id, created_at DESC);
CREATE INDEX reports_status_id ON reports (status, id DESC);
CREATE INDEX reports_chat_retention ON reports (created_at) WHERE chat_excerpt IS NOT NULL;

CREATE TABLE bans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('account', 'guest', 'ip')),
    subject_hash TEXT,
    ip_cidr TEXT,
    reason TEXT NOT NULL,
    expires_at INTEGER,
    created_by TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX bans_subject ON bans (subject_kind, subject_hash, expires_at);
CREATE INDEX bans_ip ON bans (created_at) WHERE ip_cidr IS NOT NULL;

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY,
    ts INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT,
    params_json TEXT CHECK (json_valid(params_json)),
    result TEXT NOT NULL,
    ip TEXT
);
CREATE INDEX audit_ts ON audit_log (ts);

CREATE TABLE announcements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    text TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE moderation_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id TEXT NOT NULL REFERENCES matches (id),
    action TEXT NOT NULL,
    payload TEXT NOT NULL CHECK (json_valid(payload)),
    created_at INTEGER NOT NULL,
    result TEXT
);
CREATE INDEX moderation_actions_pending ON moderation_actions (match_id, id) WHERE result IS NULL;

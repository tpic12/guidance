CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_instance_owner BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE user_permissions (
    user_id TEXT NOT NULL REFERENCES users(id),
    permission TEXT NOT NULL,
    granted_by TEXT REFERENCES users(id),
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, permission)
);

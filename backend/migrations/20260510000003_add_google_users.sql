CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT NOT NULL,
    picture_url TEXT,
    google_refresh_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT users_id_not_empty CHECK (length(id) > 0),
    CONSTRAINT users_email_not_empty CHECK (length(email) > 0),
    CONSTRAINT users_display_name_not_empty CHECK (length(display_name) > 0)
);

CREATE UNIQUE INDEX users_email_unique_idx ON users (lower(email));
CREATE INDEX users_display_name_idx ON users (lower(display_name));

CREATE TABLE todo_calendar_syncs (
    todo_id UUID NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    calendar_id TEXT NOT NULL DEFAULT 'primary',
    google_event_id TEXT,
    google_event_link TEXT,
    sync_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    sync_status TEXT NOT NULL DEFAULT 'not_synced',
    sync_error TEXT,
    synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (todo_id, user_id),
    CONSTRAINT todo_calendar_syncs_user_id_not_empty CHECK (length(user_id) > 0),
    CONSTRAINT todo_calendar_syncs_calendar_id_not_empty CHECK (length(calendar_id) > 0),
    CONSTRAINT todo_calendar_syncs_status_not_empty CHECK (length(sync_status) > 0)
);

CREATE INDEX todo_calendar_syncs_user_id_idx ON todo_calendar_syncs(user_id);

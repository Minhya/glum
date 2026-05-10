ALTER TABLE notes
    ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT '__legacy_unowned__';

ALTER TABLE notes
    ALTER COLUMN owner_user_id DROP DEFAULT;

ALTER TABLE notes
    ADD CONSTRAINT notes_owner_user_id_not_empty CHECK (length(owner_user_id) > 0);

CREATE TABLE note_shares (
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    can_write BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (note_id, user_id),
    CONSTRAINT note_shares_user_id_not_empty CHECK (length(user_id) > 0)
);

CREATE INDEX note_shares_user_id_idx ON note_shares(user_id);

ALTER TABLE todos
    ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT '__legacy_unowned__';

ALTER TABLE todos
    ALTER COLUMN owner_user_id DROP DEFAULT;

ALTER TABLE todos
    ADD CONSTRAINT todos_owner_user_id_not_empty CHECK (length(owner_user_id) > 0);

CREATE TABLE todo_shares (
    todo_id UUID NOT NULL REFERENCES todos(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    can_write BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (todo_id, user_id),
    CONSTRAINT todo_shares_user_id_not_empty CHECK (length(user_id) > 0)
);

CREATE INDEX todo_shares_user_id_idx ON todo_shares(user_id);

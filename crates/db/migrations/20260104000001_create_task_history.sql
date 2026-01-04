-- Create task_history table for storing audit trail of task changes
CREATE TABLE task_history (
    id          BLOB PRIMARY KEY,
    task_id     BLOB NOT NULL,
    field_changed TEXT NOT NULL CHECK(field_changed != ''),
    old_value   TEXT,
    new_value   TEXT,
    changed_by  TEXT NOT NULL CHECK(changed_by != ''),
    changed_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for efficient lookup by task_id
CREATE INDEX idx_task_history_task_id ON task_history(task_id);

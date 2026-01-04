-- Create task_comments table for storing comments on tasks
CREATE TABLE task_comments (
    id          BLOB PRIMARY KEY,
    task_id     BLOB NOT NULL,
    content     TEXT NOT NULL CHECK(content != ''),
    author      TEXT NOT NULL CHECK(author != ''),
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for efficient lookup by task_id
CREATE INDEX idx_task_comments_task_id ON task_comments(task_id);

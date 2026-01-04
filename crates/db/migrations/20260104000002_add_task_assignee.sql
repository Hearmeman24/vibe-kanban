-- Add assignee field to tasks for assigning tasks to agents/users
ALTER TABLE tasks ADD COLUMN assignee TEXT;

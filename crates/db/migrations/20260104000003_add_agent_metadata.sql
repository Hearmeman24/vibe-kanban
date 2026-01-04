-- Add agent_metadata field to tasks for tracking which agents worked on each task
-- Stored as JSON array of AgentMetadataEntry objects
-- Format: [{"agent_name": "Ferris", "action": "started", "timestamp": "2026-01-04T12:00:00Z", "summary": "..."}]
ALTER TABLE tasks ADD COLUMN agent_metadata TEXT;

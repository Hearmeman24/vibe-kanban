-- Add workspace_mode column to track whether workspace uses worktree or branch-only mode
-- This prevents ensure_container_exists from creating worktrees for branch-only workspaces

-- Add the workspace_mode column with a default of 'worktree' for existing workspaces
ALTER TABLE workspaces ADD COLUMN workspace_mode TEXT NOT NULL DEFAULT 'worktree';

-- For workspaces with setup_completed_at set but no container_ref, they are likely branch-only mode
-- Update them accordingly (this handles existing ORCHESTRATOR_MANAGED workspaces)
UPDATE workspaces
SET workspace_mode = 'branch'
WHERE setup_completed_at IS NOT NULL
  AND container_ref IS NULL;

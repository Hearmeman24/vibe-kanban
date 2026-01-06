import { useMemo } from 'react';
import { useProjects } from './useProjects';
import { useQueries } from '@tanstack/react-query';
import { fileSystemApi } from '@/lib/api';
import type { DiscoveredAgent } from './useProjectAgents';

// Re-export the type for convenience
export type { DiscoveredAgent };

/**
 * The default Claude agent that is always included in the global agents list
 */
const CLAUDE_AGENT: DiscoveredAgent = {
  name: 'Claude',
  role: 'Orchestrator',
};

/**
 * Parse agent name and role from markdown file content
 * Looks for patterns like:
 * - **Name:** Ada
 * - **Role:** Architect (Planning/Design)
 */
function parseAgentFromContent(content: string): Partial<DiscoveredAgent> {
  const result: Partial<DiscoveredAgent> = {};

  // Match patterns like "- **Name:** Ada" or "**Name:** Ada"
  const nameMatch = content.match(/\*\*Name:\*\*\s*(.+?)(?:\n|$)/i);
  if (nameMatch) {
    result.name = nameMatch[1].trim();
  }

  // Match patterns like "- **Role:** Architect" or "**Role:** Frontend Supervisor"
  const roleMatch = content.match(/\*\*Role:\*\*\s*(.+?)(?:\n|$)/i);
  if (roleMatch) {
    // Clean up the role - remove parenthetical notes and extra whitespace
    let role = roleMatch[1].trim();
    // Remove trailing parenthetical like "(Planning/Design)"
    role = role.replace(/\s*\([^)]*\)\s*$/, '').trim();
    result.role = role;
  }

  return result;
}

/**
 * Extract agent name from filename as fallback
 * e.g., "frontend-supervisor.md" -> "Frontend Supervisor"
 */
function agentNameFromFilename(filename: string): string {
  // Remove .md extension
  const baseName = filename.replace(/\.md$/i, '');
  // Convert kebab-case to Title Case
  return baseName
    .split('-')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Fetch file content from the backend
 */
async function fetchFileContent(path: string): Promise<string> {
  const response = await fetch(
    '/api/filesystem/file?path=' + encodeURIComponent(path)
  );
  if (!response.ok) {
    throw new Error('Failed to fetch file: ' + response.statusText);
  }
  const result = await response.json();
  return result.data?.content ?? '';
}

/**
 * Fetch agents for a single repository path
 */
async function fetchAgentsForRepo(repoPath: string): Promise<DiscoveredAgent[]> {
  const agentsPath = repoPath + '/.claude/agents';

  try {
    // List files in the .claude/agents directory
    const dirResponse = await fileSystemApi.list(agentsPath);

    if (!dirResponse?.entries) {
      return [];
    }

    // Filter for .md files only
    const agentFiles = dirResponse.entries.filter(
      (entry) => !entry.is_directory && entry.name.endsWith('.md')
    );

    // Fetch and parse each agent file
    const agentPromises = agentFiles.map(async (file) => {
      try {
        const content = await fetchFileContent(file.path);
        const parsed = parseAgentFromContent(content);

        return {
          name: parsed.name || agentNameFromFilename(file.name),
          role: parsed.role,
          filename: file.name,
        } as DiscoveredAgent;
      } catch (err) {
        // If we can't read the file, use filename as fallback
        return {
          name: agentNameFromFilename(file.name),
          filename: file.name,
        } as DiscoveredAgent;
      }
    });

    return Promise.all(agentPromises);
  } catch {
    // If the .claude/agents folder doesn't exist, return empty array
    return [];
  }
}

export interface UseGlobalAgentsResult {
  /** All unique agents discovered across all projects, plus Claude */
  agents: DiscoveredAgent[];
  /** Whether any project's agents are still loading */
  isLoading: boolean;
  /** Whether any project's agents failed to load */
  isError: boolean;
}

/**
 * Hook to get all unique agents across all projects
 *
 * Aggregates agents from all projects' .claude/agents folders,
 * deduplicates by name, and always includes the Claude agent.
 *
 * @returns Object containing agents array and loading/error states
 *
 * @example
 * ```tsx
 * const { agents, isLoading } = useGlobalAgents();
 * // agents includes Claude plus all discovered agents
 * ```
 */
export function useGlobalAgents(): UseGlobalAgentsResult {
  const { projects, isLoading: projectsLoading } = useProjects();

  // Create queries for each project's first repository
  const repoQueries = useQueries({
    queries: projects.map((project) => ({
      queryKey: ['project-repos-for-agents', project.id],
      queryFn: async () => {
        const response = await fetch('/api/projects/' + project.id + '/repositories');
        if (!response.ok) {
          return [];
        }
        const result = await response.json();
        return result.data ?? [];
      },
      staleTime: 5 * 60 * 1000,
      enabled: !!project.id,
    })),
  });

  // Extract primary repo paths from all projects
  const repoPaths = useMemo(() => {
    return repoQueries
      .map((query) => {
        const repos = query.data as Array<{ path: string }> | undefined;
        return repos?.[0]?.path;
      })
      .filter((path): path is string => !!path);
  }, [repoQueries]);

  // Create queries for each repo's agents
  const agentQueries = useQueries({
    queries: repoPaths.map((repoPath) => ({
      queryKey: ['global-agents-repo', repoPath],
      queryFn: () => fetchAgentsForRepo(repoPath),
      staleTime: 5 * 60 * 1000,
      retry: false,
    })),
  });

  // Aggregate and deduplicate agents
  const agents = useMemo(() => {
    const agentMap = new Map<string, DiscoveredAgent>();

    // Always include Claude first
    agentMap.set(CLAUDE_AGENT.name.toLowerCase(), CLAUDE_AGENT);

    // Add agents from all projects
    agentQueries.forEach((query) => {
      const projectAgents = query.data ?? [];
      projectAgents.forEach((agent) => {
        if (agent.name) {
          const key = agent.name.toLowerCase();
          // Only add if not already present (first occurrence wins)
          if (!agentMap.has(key)) {
            agentMap.set(key, agent);
          }
        }
      });
    });

    // Convert to array and sort alphabetically (Claude stays at natural position)
    return Array.from(agentMap.values()).sort((a, b) =>
      a.name.localeCompare(b.name)
    );
  }, [agentQueries]);

  const isLoading =
    projectsLoading ||
    repoQueries.some((q) => q.isLoading) ||
    agentQueries.some((q) => q.isLoading);

  const isError =
    repoQueries.some((q) => q.isError) || agentQueries.some((q) => q.isError);

  return {
    agents,
    isLoading,
    isError,
  };
}

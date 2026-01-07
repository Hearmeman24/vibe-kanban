import { useQuery } from '@tanstack/react-query';
import { useProjectRepos } from './useProjectRepos';
import { fileSystemApi } from '@/lib/api';

/**
 * Represents a discovered agent from the .claude/agents folder
 */
export interface DiscoveredAgent {
  /** Agent name extracted from frontmatter (e.g., "Ada", "Miley") */
  name: string;
  /** Agent role extracted from frontmatter (e.g., "Architect", "Frontend Supervisor") */
  role?: string;
  /** The filename of the agent definition */
  filename?: string;
}

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
 * Note: This requires a backend endpoint at /api/filesystem/file
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

interface UseProjectAgentsOptions {
  enabled?: boolean;
}

export interface UseProjectAgentsResult {
  agents: DiscoveredAgent[];
  isLoading: boolean;
  isError: boolean;
  error: Error | null;
}

/**
 * Hook to discover agents from a project's .claude/agents folder
 *
 * Scans the project's repository for agent definition files and extracts
 * agent names and roles from the markdown frontmatter.
 *
 * @param projectId - The project ID to scan for agents
 * @param options - Optional configuration
 * @returns Object containing agents array and loading/error states
 *
 * @example
 * ```tsx
 * const { agents, isLoading } = useProjectAgents(projectId);
 * agents.forEach(agent => console.log(agent.name, agent.role));
 * ```
 */
export function useProjectAgents(
  projectId: string | undefined,
  options?: UseProjectAgentsOptions
): UseProjectAgentsResult {
  const { data: repos, isLoading: reposLoading } = useProjectRepos(projectId, {
    enabled: options?.enabled !== false && !!projectId,
  });

  // Get the primary repository path (first repo)
  const primaryRepoPath = repos?.[0]?.path;

  const {
    data: agents = [],
    isLoading: agentsLoading,
    isError,
    error,
  } = useQuery<DiscoveredAgent[], Error>({
    queryKey: ['project-agents', projectId, primaryRepoPath],
    queryFn: async (): Promise<DiscoveredAgent[]> => {
      if (!primaryRepoPath) {
        return [];
      }

      const agentsPath = primaryRepoPath + '/.claude/agents';

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
            console.warn('Failed to parse agent file ' + file.name + ':', err);
            return {
              name: agentNameFromFilename(file.name),
              filename: file.name,
            } as DiscoveredAgent;
          }
        });

        const results = await Promise.all(agentPromises);

        // Filter out any agents without names and sort alphabetically
        return results
          .filter((agent): agent is DiscoveredAgent => !!agent.name)
          .sort((a, b) => a.name.localeCompare(b.name));
      } catch (err) {
        // If the .claude/agents folder doesn't exist, return empty array
        console.debug(
          'No .claude/agents folder found for project ' + projectId + ':',
          err
        );
        return [];
      }
    },
    enabled: options?.enabled !== false && !!projectId && !!primaryRepoPath,
    staleTime: 5 * 60 * 1000, // 5 minutes cache
    retry: false, // Don't retry if folder doesn't exist
  });

  return {
    agents,
    isLoading: reposLoading || agentsLoading,
    isError,
    error: error ?? null,
  };
}

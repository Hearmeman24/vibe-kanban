import { useQuery } from '@tanstack/react-query';
import { agentsApi, type AgentMetadata } from '@/lib/api';

/**
 * Query keys for project agents - follows TanStack Query key factory pattern
 */
export const projectAgentKeys = {
  all: ['projectAgents'] as const,
  byProject: (projectId: string | undefined) =>
    ['projectAgents', projectId] as const,
};

type Options = {
  enabled?: boolean;
};

/**
 * Hook to fetch agents defined in a project's `.claude/agents` folder.
 *
 * @param projectId - Optional project ID to scope agents to a specific project
 * @param opts - Optional configuration (enabled flag)
 * @returns TanStack Query result with AgentMetadata[]
 *
 * @example
 * ```tsx
 * const { data: agents, isLoading } = useProjectAgents(projectId);
 *
 * // Render agent list
 * agents?.map(agent => (
 *   <div key={agent.path}>
 *     <span>{agent.name}</span>
 *     <span>{agent.description}</span>
 *   </div>
 * ));
 * ```
 */
export function useProjectAgents(projectId?: string | null, opts?: Options) {
  const enabled = (opts?.enabled ?? true) && !!projectId;

  return useQuery<AgentMetadata[]>({
    queryKey: projectAgentKeys.byProject(projectId ?? undefined),
    queryFn: () => agentsApi.getProjectAgents(projectId!),
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
    refetchOnWindowFocus: false,
    // Return empty array on error (graceful degradation)
    placeholderData: [],
  });
}

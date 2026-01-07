import { useQuery } from '@tanstack/react-query';
import { agentsApi, type AgentMetadata } from '@/lib/api';

/**
 * Query keys for global agents - follows TanStack Query key factory pattern
 */
export const globalAgentKeys = {
  all: ['globalAgents'] as const,
};

type Options = {
  enabled?: boolean;
};

/**
 * Hook to fetch agents defined in the root `.claude/agents` folder.
 * These are global agents available across all projects.
 *
 * The hook automatically deduplicates agents by name (first occurrence wins).
 *
 * @param opts - Optional configuration (enabled flag)
 * @returns TanStack Query result with AgentMetadata[]
 *
 * @example
 * ```tsx
 * const { data: agents, isLoading } = useGlobalAgents();
 *
 * // Render agent list with avatar fallback
 * agents?.map(agent => (
 *   <div key={agent.path}>
 *     <Avatar>{agent.avatarLetter}</Avatar>
 *     <span>{agent.name}</span>
 *     <span>{agent.description}</span>
 *   </div>
 * ));
 * ```
 */
export function useGlobalAgents(opts?: Options) {
  const enabled = opts?.enabled ?? true;

  return useQuery<AgentMetadata[]>({
    queryKey: globalAgentKeys.all,
    queryFn: async () => {
      const agents = await agentsApi.getGlobalAgents();
      // Deduplicate by name (first occurrence wins)
      const seen = new Set<string>();
      return agents.filter((agent) => {
        if (seen.has(agent.name)) {
          return false;
        }
        seen.add(agent.name);
        return true;
      });
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
    refetchOnWindowFocus: false,
    // Return empty array on error (graceful degradation)
    placeholderData: [],
  });
}

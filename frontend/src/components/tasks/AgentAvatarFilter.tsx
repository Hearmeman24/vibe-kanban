import { useMemo, useCallback } from 'react';
import { Users } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useProjectAgents, useAgentAvatars } from '@/hooks';
import type { AgentMetadata } from '@/lib/api';
import { AgentAvatarDisplay } from '@/components/AgentAvatarDisplay';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';

/**
 * The default Claude agent (orchestrator) that is always included
 */
const CLAUDE_AGENT: AgentMetadata = {
  name: 'Claude',
  description: 'Orchestrator',
  path: '',
  avatarLetter: 'C',
};

export interface AgentAvatarFilterProps {
  /** Project ID to fetch agents for */
  projectId: string;
  /** Currently selected agent names */
  selectedAgents: string[];
  /** Callback when selection changes */
  onSelectionChange: (agents: string[]) => void;
  /** Task counts by assignee name */
  taskCounts?: Record<string, number>;
  /** Optional className for the container */
  className?: string;
}

interface AgentWithCount extends AgentMetadata {
  taskCount: number;
}

/**
 * Horizontal scrollable bar of agent avatars for filtering tasks.
 *
 * Features:
 * - Displays all project agents plus Claude (orchestrator)
 * - Shows task count badge on each avatar
 * - Multi-select capability (click to toggle)
 * - "All Agents" button to clear selection
 * - Tooltip with agent name, description, and task count on hover
 */
export function AgentAvatarFilter({
  projectId,
  selectedAgents,
  onSelectionChange,
  taskCounts = {},
  className,
}: AgentAvatarFilterProps) {
  const { data: projectAgents = [], isLoading } = useProjectAgents(projectId);
  const { getAvatar } = useAgentAvatars();

  // Merge project agents with Claude, ensuring Claude is always present
  const allAgents = useMemo((): AgentWithCount[] => {
    const agentMap = new Map<string, AgentMetadata>();

    // Always include Claude first
    agentMap.set(CLAUDE_AGENT.name.toLowerCase(), CLAUDE_AGENT);

    // Add project agents
    projectAgents.forEach((agent) => {
      const key = agent.name.toLowerCase();
      if (!agentMap.has(key)) {
        agentMap.set(key, agent);
      }
    });

    // Convert to array with task counts, sorted alphabetically
    return Array.from(agentMap.values())
      .map((agent) => ({
        ...agent,
        taskCount: taskCounts[agent.name] || 0,
      }))
      .sort((a, b) => {
        // Claude always first
        if (a.name === 'Claude') return -1;
        if (b.name === 'Claude') return 1;
        return a.name.localeCompare(b.name);
      });
  }, [projectAgents, taskCounts]);

  const isAllSelected = selectedAgents.length === 0;

  const handleAgentClick = useCallback(
    (agentName: string) => {
      if (selectedAgents.includes(agentName)) {
        // Deselect the agent
        const newSelection = selectedAgents.filter((a) => a !== agentName);
        onSelectionChange(newSelection);
      } else {
        // Add to selection
        onSelectionChange([...selectedAgents, agentName]);
      }
    },
    [selectedAgents, onSelectionChange]
  );

  const handleSelectAll = useCallback(() => {
    onSelectionChange([]);
  }, [onSelectionChange]);

  // Calculate total task count
  const totalTaskCount = useMemo(() => {
    return Object.values(taskCounts).reduce((sum, count) => sum + count, 0);
  }, [taskCounts]);

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 px-4 py-2', className)}>
        <div className="flex items-center gap-2">
          {/* Loading skeleton */}
          {[1, 2, 3].map((i) => (
            <div
              key={i}
              className="w-12 h-12 rounded-full bg-muted animate-pulse"
            />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        'flex items-center gap-3 px-4 py-2 overflow-x-auto',
        className
      )}
    >
      {/* All Agents button */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant={isAllSelected ? 'default' : 'outline'}
            size="sm"
            onClick={handleSelectAll}
            className={cn(
              'flex items-center gap-2 shrink-0 h-12 px-4',
              isAllSelected && 'ring-2 ring-primary ring-offset-2'
            )}
          >
            <Users className="h-4 w-4" />
            <span>All</span>
            {totalTaskCount > 0 && (
              <span className="ml-1 text-xs bg-muted text-muted-foreground px-1.5 py-0.5 rounded-full">
                {totalTaskCount}
              </span>
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          <p className="font-medium">All Agents</p>
          <p className="text-xs text-muted-foreground">
            Show all tasks ({totalTaskCount})
          </p>
        </TooltipContent>
      </Tooltip>

      {/* Separator */}
      <div className="h-8 w-px bg-border shrink-0" />

      {/* Agent avatars */}
      <div className="flex items-center gap-2">
        {allAgents.map((agent) => {
          const isSelected = selectedAgents.includes(agent.name);
          const avatar = getAvatar(agent.name);

          return (
            <AgentAvatarItem
              key={agent.name}
              agent={agent}
              avatar={avatar}
              isSelected={isSelected}
              onClick={() => handleAgentClick(agent.name)}
            />
          );
        })}
      </div>
    </div>
  );
}

interface AgentAvatarItemProps {
  agent: AgentWithCount;
  avatar: string | null;
  isSelected: boolean;
  onClick: () => void;
}

function AgentAvatarItem({
  agent,
  avatar,
  isSelected,
  onClick,
}: AgentAvatarItemProps) {
  const tooltipLabel = agent.description
    ? `${agent.name} (${agent.description})`
    : agent.name;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onClick}
          className={cn(
            'relative flex-shrink-0 rounded-full transition-all duration-200',
            'focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
            isSelected
              ? 'ring-2 ring-primary ring-offset-2 scale-105'
              : 'hover:scale-105 opacity-70 hover:opacity-100'
          )}
        >
          <AgentAvatarDisplay
            agentName={agent.name}
            avatar={avatar ?? undefined}
            size="sm"
            role={agent.description}
          />
          {/* Task count badge */}
          {agent.taskCount > 0 && (
            <span
              className={cn(
                'absolute -top-1 -right-1 min-w-[18px] h-[18px] flex items-center justify-center',
                'text-[10px] font-semibold rounded-full px-1',
                isSelected
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted-foreground text-background'
              )}
            >
              {agent.taskCount > 99 ? '99+' : agent.taskCount}
            </span>
          )}
        </button>
      </TooltipTrigger>
      <TooltipContent>
        <p className="font-medium">{tooltipLabel}</p>
        <p className="text-xs text-muted-foreground">
          {agent.taskCount === 1
            ? '1 task'
            : `${agent.taskCount} tasks`}
        </p>
      </TooltipContent>
    </Tooltip>
  );
}

export default AgentAvatarFilter;

import { useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Users, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useProjectRemoteMembers } from '@/hooks/useProjectRemoteMembers';
import { UserAvatar } from '@/components/tasks/UserAvatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { TaskWithAttemptStatus, OrganizationMemberWithProfile } from 'shared/types';

interface AgentAvatarFilterProps {
  /** Current assignee filter value(s) - comma-separated usernames or 'all' */
  assigneeFilter: string;
  /** Handler to update filter value */
  onFilterChange: (value: string) => void;
  /** Project ID to fetch members from */
  projectId?: string;
  /** Tasks to calculate counts from */
  tasks: TaskWithAttemptStatus[];
}

interface AssigneeWithCount {
  username: string;
  firstName: string | null;
  lastName: string | null;
  avatarUrl: string | null;
  taskCount: number;
  member?: OrganizationMemberWithProfile;
}

/**
 * AgentAvatarFilter - A horizontal scrollable bar of agent avatars for filtering tasks.
 * Replaces the dropdown-based assignee filter with a visual team roster.
 */
export function AgentAvatarFilter({
  assigneeFilter,
  onFilterChange,
  projectId,
  tasks,
}: AgentAvatarFilterProps) {
  const { t } = useTranslation(['tasks', 'common']);
  const { data: remoteMembersData } = useProjectRemoteMembers(projectId);
  const members = remoteMembersData?.members ?? [];

  // Parse selected assignees from filter string
  const selectedAssignees = useMemo(() => {
    if (assigneeFilter === 'all' || !assigneeFilter) return new Set<string>();
    return new Set(assigneeFilter.split(',').filter(Boolean));
  }, [assigneeFilter]);

  // Calculate task counts per assignee
  const taskCountsByAssignee = useMemo(() => {
    const counts = new Map<string, number>();
    let unassignedCount = 0;

    tasks.forEach((task) => {
      if (task.assignee) {
        counts.set(task.assignee, (counts.get(task.assignee) || 0) + 1);
      } else {
        unassignedCount++;
      }
    });

    return { counts, unassignedCount };
  }, [tasks]);

  // Build list of assignees with their counts
  const assigneesWithCounts = useMemo<AssigneeWithCount[]>(() => {
    // Start with organization members
    const memberMap = new Map<string, AssigneeWithCount>();

    members.forEach((member) => {
      const username = member.username || member.user_id;
      memberMap.set(username, {
        username,
        firstName: member.first_name,
        lastName: member.last_name,
        avatarUrl: member.avatar_url,
        taskCount: taskCountsByAssignee.counts.get(username) || 0,
        member,
      });
    });

    // Add any assignees from tasks that might not be in members list
    taskCountsByAssignee.counts.forEach((count, assignee) => {
      if (!memberMap.has(assignee)) {
        memberMap.set(assignee, {
          username: assignee,
          firstName: null,
          lastName: null,
          avatarUrl: null,
          taskCount: count,
        });
      }
    });

    // Sort by task count descending, then by name
    return Array.from(memberMap.values()).sort((a, b) => {
      if (b.taskCount !== a.taskCount) return b.taskCount - a.taskCount;
      const nameA = a.firstName || a.username || '';
      const nameB = b.firstName || b.username || '';
      return nameA.localeCompare(nameB);
    });
  }, [members, taskCountsByAssignee.counts]);

  // Toggle selection for an assignee
  const toggleAssignee = useCallback(
    (username: string) => {
      const newSelected = new Set(selectedAssignees);

      if (newSelected.has(username)) {
        newSelected.delete(username);
      } else {
        newSelected.add(username);
      }

      if (newSelected.size === 0) {
        onFilterChange('all');
      } else {
        onFilterChange(Array.from(newSelected).join(','));
      }
    },
    [selectedAssignees, onFilterChange]
  );

  // Toggle unassigned filter
  const toggleUnassigned = useCallback(() => {
    if (assigneeFilter === 'unassigned') {
      onFilterChange('all');
    } else {
      onFilterChange('unassigned');
    }
  }, [assigneeFilter, onFilterChange]);

  // Clear all filters
  const clearFilter = useCallback(() => {
    onFilterChange('all');
  }, [onFilterChange]);

  const hasActiveFilter = assigneeFilter !== 'all' && assigneeFilter !== '';
  const isUnassignedSelected = assigneeFilter === 'unassigned';
  const totalTaskCount = tasks.length;

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex items-center gap-2 px-4 py-2 overflow-x-auto">
        {/* All Agents Button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={clearFilter}
              className={cn(
                'flex items-center gap-2 px-3 py-1.5 rounded-full border transition-all duration-200 shrink-0',
                !hasActiveFilter
                  ? 'bg-primary text-primary-foreground border-primary shadow-sm'
                  : 'bg-muted/50 text-muted-foreground border-border hover:bg-muted hover:border-muted-foreground/30'
              )}
            >
              <Users className="h-4 w-4" />
              <span className="text-sm font-medium">
                {t('filter.allAgents', { defaultValue: 'All' })}
              </span>
              <Badge
                variant="secondary"
                className={cn(
                  'h-5 min-w-[20px] px-1.5 text-xs',
                  !hasActiveFilter && 'bg-primary-foreground/20 text-primary-foreground'
                )}
              >
                {totalTaskCount}
              </Badge>
            </button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{t('filter.showAllTasks', { defaultValue: 'Show all tasks' })} ({totalTaskCount})</p>
          </TooltipContent>
        </Tooltip>

        {/* Divider */}
        <div className="h-8 w-px bg-border shrink-0" />

        {/* Unassigned Button */}
        {taskCountsByAssignee.unassignedCount > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={toggleUnassigned}
                className={cn(
                  'relative flex items-center justify-center h-[50px] w-[50px] rounded-full border-2 transition-all duration-200 shrink-0',
                  isUnassignedSelected
                    ? 'border-primary ring-2 ring-primary/30 shadow-lg'
                    : 'border-dashed border-muted-foreground/40 hover:border-muted-foreground/60 hover:bg-muted/50'
                )}
              >
                <span className="text-muted-foreground text-lg">?</span>
                <Badge
                  variant="secondary"
                  className="absolute -top-1 -right-1 h-5 min-w-[20px] px-1.5 text-xs"
                >
                  {taskCountsByAssignee.unassignedCount}
                </Badge>
              </button>
            </TooltipTrigger>
            <TooltipContent>
              <p>
                {t('filter.unassigned', { defaultValue: 'Unassigned' })} ({taskCountsByAssignee.unassignedCount}{' '}
                {taskCountsByAssignee.unassignedCount === 1
                  ? t('filter.task', { defaultValue: 'task' })
                  : t('filter.tasks', { defaultValue: 'tasks' })}
                )
              </p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* Agent Avatars */}
        {assigneesWithCounts.map((assignee) => {
          const isSelected = selectedAssignees.has(assignee.username);
          const displayName =
            [assignee.firstName, assignee.lastName].filter(Boolean).join(' ') ||
            assignee.username;

          return (
            <Tooltip key={assignee.username}>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={() => toggleAssignee(assignee.username)}
                  className={cn(
                    'relative shrink-0 transition-all duration-200',
                    isSelected
                      ? 'ring-2 ring-primary ring-offset-2 ring-offset-background rounded-full'
                      : 'hover:scale-105'
                  )}
                >
                  <UserAvatar
                    firstName={assignee.firstName}
                    lastName={assignee.lastName}
                    username={assignee.username}
                    imageUrl={assignee.avatarUrl}
                    className={cn(
                      'h-[50px] w-[50px] text-lg cursor-pointer',
                      isSelected && 'shadow-lg'
                    )}
                  />
                  {assignee.taskCount > 0 && (
                    <Badge
                      variant="secondary"
                      className="absolute -top-1 -right-1 h-5 min-w-[20px] px-1.5 text-xs"
                    >
                      {assignee.taskCount}
                    </Badge>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <p className="font-medium">{displayName}</p>
                <p className="text-xs text-muted-foreground">
                  {assignee.taskCount}{' '}
                  {assignee.taskCount === 1
                    ? t('filter.task', { defaultValue: 'task' })
                    : t('filter.tasks', { defaultValue: 'tasks' })}
                </p>
              </TooltipContent>
            </Tooltip>
          );
        })}

        {/* Clear Filter Button */}
        {hasActiveFilter && (
          <>
            <div className="h-8 w-px bg-border shrink-0" />
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2 shrink-0"
                  onClick={clearFilter}
                >
                  <X className="h-4 w-4 mr-1" />
                  {t('filter.clear', { defaultValue: 'Clear' })}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{t('filter.clearFilter', { defaultValue: 'Clear filter' })}</p>
              </TooltipContent>
            </Tooltip>
          </>
        )}
      </div>
    </TooltipProvider>
  );
}

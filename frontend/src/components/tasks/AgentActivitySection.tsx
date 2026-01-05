import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import {
  Bot,
  Play,
  CheckCircle,
  MessageSquare,
  RefreshCw,
  AlertCircle,
} from 'lucide-react';
import { agentMetadataApi } from '@/lib/api';
import type { AgentMetadataEntry } from 'shared/types';
import { cn } from '@/lib/utils';

interface AgentActivitySectionProps {
  taskId: string;
  className?: string;
}

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleString();
  } catch {
    return dateStr;
  }
}

function formatTimeAgo(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);
    const diffHour = Math.floor(diffMin / 60);
    const diffDay = Math.floor(diffHour / 24);

    if (diffSec < 60) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    if (diffHour < 24) return `${diffHour}h ago`;
    if (diffDay < 7) return `${diffDay}d ago`;
    return date.toLocaleDateString();
  } catch {
    return dateStr;
  }
}

function getActionIcon(action: string) {
  switch (action.toLowerCase()) {
    case 'started':
      return <Play className="w-3.5 h-3.5 text-blue-500" />;
    case 'completed':
      return <CheckCircle className="w-3.5 h-3.5 text-green-500" />;
    case 'commented':
      return <MessageSquare className="w-3.5 h-3.5 text-purple-500" />;
    case 'updated':
      return <RefreshCw className="w-3.5 h-3.5 text-orange-500" />;
    case 'failed':
    case 'error':
      return <AlertCircle className="w-3.5 h-3.5 text-red-500" />;
    default:
      return <Bot className="w-3.5 h-3.5 text-muted-foreground" />;
  }
}

function getActionLabel(action: string): string {
  switch (action.toLowerCase()) {
    case 'started':
      return 'started working';
    case 'completed':
      return 'completed work';
    case 'commented':
      return 'added a comment';
    case 'updated':
      return 'made updates';
    case 'failed':
    case 'error':
      return 'encountered an error';
    default:
      return action;
  }
}

function ActivityEntry({ entry }: { entry: AgentMetadataEntry }) {
  return (
    <div className="py-2 border-b border-border last:border-b-0">
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">{getActionIcon(entry.action)}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-medium text-foreground">
              {entry.agent_name}
            </span>
            <span
              className="text-xs text-muted-foreground flex-shrink-0"
              title={formatDate(entry.timestamp)}
            >
              {formatTimeAgo(entry.timestamp)}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            {getActionLabel(entry.action)}
          </p>
          {entry.summary && (
            <p className="text-sm text-muted-foreground mt-1 whitespace-pre-wrap break-words">
              {entry.summary}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

export function AgentActivitySection({
  taskId,
  className,
}: AgentActivitySectionProps) {
  const { t } = useTranslation('tasks');
  const [isExpanded, setIsExpanded] = useState(false);

  const {
    data,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['agentMetadata', taskId],
    queryFn: () => agentMetadataApi.getByTaskId(taskId),
    enabled: !!taskId && isExpanded, // Only fetch when expanded
  });

  const metadata = data?.metadata ?? [];
  const activityCount = data?.count ?? 0;

  // Sort by timestamp descending (most recent first)
  const sortedMetadata = [...metadata].sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
  );

  return (
    <div className={cn('space-y-3', className)}>
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
      >
        <Bot className="w-4 h-4" />
        <span>
          {t('agentActivity.title', { defaultValue: 'Agent Activity' })}
          {isExpanded && activityCount > 0 && ` (${activityCount})`}
        </span>
      </button>

      {isExpanded && (
        <div className="pl-6">
          {isLoading && (
            <p className="text-sm text-muted-foreground">
              {t('agentActivity.loading', {
                defaultValue: 'Loading agent activity...',
              })}
            </p>
          )}

          {error && (
            <p className="text-sm text-destructive">
              {t('agentActivity.error', {
                defaultValue: 'Failed to load agent activity',
              })}
            </p>
          )}

          {!isLoading && !error && sortedMetadata.length === 0 && (
            <p className="text-sm text-muted-foreground">
              {t('agentActivity.empty', {
                defaultValue: 'No agent activity yet',
              })}
            </p>
          )}

          {sortedMetadata.length > 0 && (
            <div className="bg-muted/50 rounded-md border border-border p-3 max-h-[250px] overflow-y-auto">
              {sortedMetadata.map((entry, index) => (
                <ActivityEntry
                  key={`${entry.agent_name}-${entry.timestamp}-${index}`}
                  entry={entry}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { History, ArrowRight } from 'lucide-react';
import { taskHistoryApi } from '@/lib/api';
import type { TaskHistory } from 'shared/types';
import { cn } from '@/lib/utils';

interface TaskHistorySectionProps {
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

function formatFieldName(field: string): string {
  // Convert snake_case to Title Case
  return field
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

function HistoryEntry({ entry }: { entry: TaskHistory }) {
  const { t } = useTranslation('tasks');

  const getChangeDescription = () => {
    const field = formatFieldName(entry.field_changed);
    const oldVal = entry.old_value || t('history.empty', { defaultValue: '(empty)' });
    const newVal = entry.new_value || t('history.empty', { defaultValue: '(empty)' });

    return { field, oldVal, newVal };
  };

  const { field, oldVal, newVal } = getChangeDescription();

  return (
    <div className="py-2 border-b border-border last:border-b-0">
      <div className="flex items-center justify-between gap-2 mb-1">
        <span className="text-xs font-medium text-muted-foreground">
          {entry.changed_by}
        </span>
        <span className="text-xs text-muted-foreground">
          {formatDate(entry.changed_at)}
        </span>
      </div>
      <div className="flex items-center gap-2 text-sm">
        <span className="font-medium">{field}:</span>
        <span className="text-muted-foreground truncate max-w-[100px]" title={oldVal}>
          {oldVal}
        </span>
        <ArrowRight className="w-3 h-3 text-muted-foreground flex-shrink-0" />
        <span className="text-foreground truncate max-w-[100px]" title={newVal}>
          {newVal}
        </span>
      </div>
    </div>
  );
}

export function TaskHistorySection({
  taskId,
  className,
}: TaskHistorySectionProps) {
  const { t } = useTranslation('tasks');
  const [isExpanded, setIsExpanded] = useState(false);

  const {
    data: history = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: ['taskHistory', taskId],
    queryFn: () => taskHistoryApi.getByTaskId(taskId),
    enabled: !!taskId && isExpanded, // Only fetch when expanded
  });

  const historyCount = history.length;

  return (
    <div className={cn('space-y-3', className)}>
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
      >
        <History className="w-4 h-4" />
        <span>
          {t('history.title', { defaultValue: 'History' })}
          {isExpanded && historyCount > 0 && ` (${historyCount})`}
        </span>
      </button>

      {isExpanded && (
        <div className="pl-6">
          {isLoading && (
            <p className="text-sm text-muted-foreground">
              {t('history.loading', { defaultValue: 'Loading history...' })}
            </p>
          )}

          {error && (
            <p className="text-sm text-destructive">
              {t('history.error', {
                defaultValue: 'Failed to load history',
              })}
            </p>
          )}

          {!isLoading && !error && history.length === 0 && (
            <p className="text-sm text-muted-foreground">
              {t('history.empty', { defaultValue: 'No history yet' })}
            </p>
          )}

          {history.length > 0 && (
            <div className="bg-muted/50 rounded-md border border-border p-3 max-h-[200px] overflow-y-auto">
              {history.map((entry) => (
                <HistoryEntry key={entry.id} entry={entry} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

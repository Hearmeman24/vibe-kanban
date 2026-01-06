import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { MessageSquare, Send, User } from 'lucide-react';
import { taskCommentsApi, type TaskComment } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

interface TaskCommentsSectionProps {
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

function CommentCard({ comment }: { comment: TaskComment }) {
  return (
    <div className="p-3 bg-muted/50 rounded-md border border-border">
      <div className="flex items-center justify-between gap-2 mb-2">
        <div className="flex items-center gap-2 min-w-0">
          <User className="w-4 h-4 text-muted-foreground flex-shrink-0" />
          <span className="font-medium text-sm">{comment.author}</span>
        </div>
        <span className="text-xs text-muted-foreground flex-shrink-0">
          {formatDate(comment.created_at)}
        </span>
      </div>
      <p className="text-sm text-muted-foreground whitespace-pre-wrap break-words">
        {comment.content}
      </p>
    </div>
  );
}

export function TaskCommentsSection({
  taskId,
  className,
}: TaskCommentsSectionProps) {
  const { t } = useTranslation('tasks');
  const queryClient = useQueryClient();
  const [newComment, setNewComment] = useState('');
  const [isExpanded, setIsExpanded] = useState(true);

  const {
    data: comments = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: ['taskComments', taskId],
    queryFn: () => taskCommentsApi.getByTaskId(taskId),
    enabled: !!taskId,
  });

  const addCommentMutation = useMutation({
    mutationFn: (content: string) =>
      taskCommentsApi.create(taskId, {
        content,
        author: 'User', // TODO: Get actual user name from auth context
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['taskComments', taskId] });
      setNewComment('');
    },
  });

  const handleSubmitComment = useCallback(() => {
    const trimmed = newComment.trim();
    if (!trimmed) return;
    addCommentMutation.mutate(trimmed);
  }, [newComment, addCommentMutation]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleSubmitComment();
      }
    },
    [handleSubmitComment]
  );

  const commentCount = comments.length;

  return (
    <div className={cn('space-y-3', className)}>
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
      >
        <MessageSquare className="w-4 h-4" />
        <span>
          {t('comments.title', { defaultValue: 'Comments' })} ({commentCount})
        </span>
      </button>

      {isExpanded && (
        <div className="space-y-3 pl-6">
          {isLoading && (
            <p className="text-sm text-muted-foreground">
              {t('comments.loading', { defaultValue: 'Loading comments...' })}
            </p>
          )}

          {error && (
            <p className="text-sm text-destructive">
              {t('comments.error', {
                defaultValue: 'Failed to load comments',
              })}
            </p>
          )}

          {!isLoading && !error && comments.length === 0 && (
            <p className="text-sm text-muted-foreground">
              {t('comments.empty', { defaultValue: 'No comments yet' })}
            </p>
          )}

          {comments.map((comment) => (
            <CommentCard key={comment.id} comment={comment} />
          ))}

          <div className="space-y-2">
            <Textarea
              value={newComment}
              onChange={(e) => setNewComment(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t('comments.placeholder', {
                defaultValue: 'Add a comment...',
              })}
              className="min-h-[80px] resize-none"
              disabled={addCommentMutation.isPending}
            />
            <div className="flex justify-end">
              <Button
                size="sm"
                onClick={handleSubmitComment}
                disabled={
                  !newComment.trim() || addCommentMutation.isPending
                }
              >
                <Send className="w-4 h-4 mr-1" />
                {addCommentMutation.isPending
                  ? t('comments.sending', { defaultValue: 'Sending...' })
                  : t('comments.send', { defaultValue: 'Send' })}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

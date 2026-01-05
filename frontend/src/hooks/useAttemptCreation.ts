import { useMutation, useQueryClient } from '@tanstack/react-query';
import { attemptsApi } from '@/lib/api';
import type {
  ExecutorProfileId,
  WorkspaceRepoInput,
  Workspace,
} from 'shared/types';

type CreateAttemptArgs = {
  profile: ExecutorProfileId;
  repos: WorkspaceRepoInput[];
  mode?: 'worktree' | 'branch';
};

type UseAttemptCreationArgs = {
  taskId: string;
  onSuccess?: (attempt: Workspace) => void;
};

export function useAttemptCreation({
  taskId,
  onSuccess,
}: UseAttemptCreationArgs) {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ profile, repos, mode = 'worktree' }: CreateAttemptArgs) =>
      attemptsApi.create({
        task_id: taskId,
        executor_profile_id: profile,
        repos,
        mode,
      } as Parameters<typeof attemptsApi.create>[0]),
    onSuccess: (newAttempt: Workspace) => {
      queryClient.setQueryData(
        ['taskAttempts', taskId],
        (old: Workspace[] = []) => [newAttempt, ...old]
      );
      onSuccess?.(newAttempt);
    },
  });

  return {
    createAttempt: mutation.mutateAsync,
    isCreating: mutation.isPending,
    error: mutation.error,
  };
}

import { useCallback } from 'react';
import { useJsonPatchWsStream } from './useJsonPatchWsStream';
import { scratchApi } from '@/lib/api';
import { ScratchType, type Scratch, type UpdateScratch } from 'shared/types';

type ScratchState = {
  scratch: Scratch | null;
};

export interface UseScratchResult {
  scratch: Scratch | null;
  isLoading: boolean;
  isConnected: boolean;
  error: string | null;
  updateScratch: (update: UpdateScratch) => Promise<void>;
  deleteScratch: () => Promise<void>;
}

/**
 * Stream a single scratch item via WebSocket (JSON Patch).
 * Server sends the scratch object directly at /scratch.
 */
export const useScratch = (
  scratchType: ScratchType,
  id: string
): UseScratchResult => {
  // Don't construct endpoint or enable stream if id is empty
  // This happens for ORCHESTRATOR_MANAGED workspaces that have no session
  const hasValidId = id.length > 0;
  const endpoint = hasValidId
    ? scratchApi.getStreamUrl(scratchType, id)
    : undefined;

  const initialData = useCallback((): ScratchState => ({ scratch: null }), []);

  const { data, isConnected, error } = useJsonPatchWsStream<ScratchState>(
    endpoint,
    hasValidId,
    initialData
  );

  // Treat deleted scratches as null
  const rawScratch = data?.scratch as (Scratch & { deleted?: boolean }) | null;
  const scratch = rawScratch?.deleted ? null : rawScratch;

  const updateScratch = useCallback(
    async (update: UpdateScratch) => {
      await scratchApi.update(scratchType, id, update);
    },
    [scratchType, id]
  );

  const deleteScratch = useCallback(async () => {
    await scratchApi.delete(scratchType, id);
  }, [scratchType, id]);

  const isLoading = !data && !error && !isConnected;

  return {
    scratch,
    isLoading,
    isConnected,
    error,
    updateScratch,
    deleteScratch,
  };
};

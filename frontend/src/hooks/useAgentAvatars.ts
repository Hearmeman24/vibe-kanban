import { useCallback, useMemo } from 'react';
import { useAvatarStore, useAvatars, useAvatar } from '../stores/useAvatarStore';
import { getAgentInitial } from '../lib/avatarUtils';

export interface UseAgentAvatarsReturn {
  /** Map of all agent avatars (agentName -> base64 data URL) */
  avatars: Record<string, string>;
  /** Set an avatar for an agent */
  setAvatar: (agentName: string, imageDataUrl: string) => void;
  /** Get an avatar for a specific agent (returns null if not found) */
  getAvatar: (agentName: string) => string | null;
  /** Get the initial character for an agent (fallback for missing avatars) */
  getInitial: (agentName: string) => string;
  /** Remove an avatar for an agent */
  removeAvatar: (agentName: string) => void;
  /** Clear all stored avatars */
  clearAvatars: () => void;
}

/**
 * Convenience hook for managing agent avatars.
 * Wraps the Zustand avatar store with a clean API.
 *
 * @example
 * ```tsx
 * function AgentCard({ agentName }: { agentName: string }) {
 *   const { avatars, getAvatar, getInitial, setAvatar } = useAgentAvatars();
 *
 *   const avatar = getAvatar(agentName);
 *
 *   return (
 *     <div>
 *       {avatar ? (
 *         <img src={avatar} alt={agentName} />
 *       ) : (
 *         <div className="avatar-fallback">{getInitial(agentName)}</div>
 *       )}
 *     </div>
 *   );
 * }
 * ```
 */
export function useAgentAvatars(): UseAgentAvatarsReturn {
  const avatars = useAvatars();
  const setAvatarAction = useAvatarStore((state) => state.setAvatar);
  const removeAvatarAction = useAvatarStore((state) => state.removeAvatar);
  const clearAvatarsAction = useAvatarStore((state) => state.clearAvatars);

  const getAvatar = useCallback(
    (agentName: string): string | null => {
      return avatars[agentName] ?? null;
    },
    [avatars]
  );

  const getInitial = useCallback((agentName: string): string => {
    return getAgentInitial(agentName);
  }, []);

  const setAvatar = useCallback(
    (agentName: string, imageDataUrl: string): void => {
      setAvatarAction(agentName, imageDataUrl);
    },
    [setAvatarAction]
  );

  const removeAvatar = useCallback(
    (agentName: string): void => {
      removeAvatarAction(agentName);
    },
    [removeAvatarAction]
  );

  const clearAvatars = useCallback((): void => {
    clearAvatarsAction();
  }, [clearAvatarsAction]);

  return useMemo(
    () => ({
      avatars,
      setAvatar,
      getAvatar,
      getInitial,
      removeAvatar,
      clearAvatars,
    }),
    [avatars, setAvatar, getAvatar, getInitial, removeAvatar, clearAvatars]
  );
}

/**
 * Hook to get a single agent's avatar with reactive updates.
 * More efficient than useAgentAvatars when you only need one agent's avatar.
 *
 * @param agentName - The name of the agent
 * @returns Object with avatar data URL (or null) and the agent's initial
 *
 * @example
 * ```tsx
 * function Avatar({ agentName }: { agentName: string }) {
 *   const { avatar, initial } = useAgentAvatar(agentName);
 *
 *   return avatar ? (
 *     <img src={avatar} alt={agentName} />
 *   ) : (
 *     <span>{initial}</span>
 *   );
 * }
 * ```
 */
export function useAgentAvatar(agentName: string): {
  avatar: string | null;
  initial: string;
} {
  const avatar = useAvatar(agentName);
  const initial = useMemo(() => getAgentInitial(agentName), [agentName]);

  return { avatar, initial };
}

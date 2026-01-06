import { useState, useEffect, useCallback } from 'react';

const STORAGE_KEY = 'vibe-kanban:agent-avatars';

export interface AgentAvatars {
  [agentName: string]: string; // agent name -> base64 data URL
}

export interface UseAgentAvatarsReturn {
  avatars: AgentAvatars;
  saveAvatar: (agentName: string, imageDataUrl: string) => void;
  getAvatar: (agentName: string) => string | null;
  deleteAvatar: (agentName: string) => void;
  clearAllAvatars: () => void;
}

/**
 * Loads avatars from localStorage safely.
 * Returns empty object on any error.
 */
function loadAvatarsFromStorage(): AgentAvatars {
  try {
    if (typeof window === 'undefined') return {};
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return {};
    const parsed = JSON.parse(stored);
    // Validate that it's an object with string values
    if (
      typeof parsed !== 'object' ||
      parsed === null ||
      Array.isArray(parsed)
    ) {
      return {};
    }
    // Filter to only valid string entries
    const validated: AgentAvatars = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof value === 'string' && value.startsWith('data:image/')) {
        validated[key] = value;
      }
    }
    return validated;
  } catch {
    return {};
  }
}

/**
 * Saves avatars to localStorage safely.
 * Logs warning on quota errors but doesn't throw.
 */
function saveAvatarsToStorage(avatars: AgentAvatars): boolean {
  try {
    if (typeof window === 'undefined') return false;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(avatars));
    return true;
  } catch (error) {
    // Handle quota exceeded or other storage errors
    if (error instanceof DOMException && error.name === 'QuotaExceededError') {
      console.warn(
        '[useAgentAvatars] localStorage quota exceeded. Consider removing some avatars or compressing images further.'
      );
    } else {
      console.warn(
        '[useAgentAvatars] Failed to save avatars to localStorage:',
        error
      );
    }
    return false;
  }
}

/**
 * Hook for managing agent avatars with localStorage persistence.
 * Avatars are stored globally (shared across all projects) by agent name.
 *
 * Features:
 * - Persistent storage in localStorage
 * - Cross-tab synchronization via storage events
 * - Graceful handling of quota limits
 *
 * @example
 * ```typescript
 * const { avatars, saveAvatar, getAvatar } = useAgentAvatars();
 *
 * // Get an avatar
 * const avatar = getAvatar("Ada"); // Returns data URL or null
 *
 * // Save an avatar (use compressImage first for large images)
 * const compressed = await compressImage(rawDataUrl);
 * saveAvatar("Ada", compressed);
 * ```
 */
export function useAgentAvatars(): UseAgentAvatarsReturn {
  const [avatars, setAvatars] = useState<AgentAvatars>(loadAvatarsFromStorage);

  // Sync with localStorage changes from other tabs
  useEffect(() => {
    if (typeof window === 'undefined') return;

    const handleStorageChange = (event: StorageEvent) => {
      if (event.key === STORAGE_KEY) {
        setAvatars(loadAvatarsFromStorage());
      }
    };

    window.addEventListener('storage', handleStorageChange);
    return () => {
      window.removeEventListener('storage', handleStorageChange);
    };
  }, []);

  const saveAvatar = useCallback((agentName: string, imageDataUrl: string) => {
    if (!agentName.trim()) {
      console.warn(
        '[useAgentAvatars] Cannot save avatar with empty agent name'
      );
      return;
    }

    if (!imageDataUrl.startsWith('data:image/')) {
      console.warn('[useAgentAvatars] Invalid image data URL format');
      return;
    }

    setAvatars((prev) => {
      const updated = { ...prev, [agentName]: imageDataUrl };
      saveAvatarsToStorage(updated);
      return updated;
    });
  }, []);

  const getAvatar = useCallback(
    (agentName: string): string | null => {
      return avatars[agentName] || null;
    },
    [avatars]
  );

  const deleteAvatar = useCallback((agentName: string) => {
    setAvatars((prev) => {
      const updated = { ...prev };
      delete updated[agentName];
      saveAvatarsToStorage(updated);
      return updated;
    });
  }, []);

  const clearAllAvatars = useCallback(() => {
    setAvatars({});
    try {
      if (typeof window !== 'undefined') {
        localStorage.removeItem(STORAGE_KEY);
      }
    } catch {
      // Ignore errors when clearing
    }
  }, []);

  return {
    avatars,
    saveAvatar,
    getAvatar,
    deleteAvatar,
    clearAllAvatars,
  };
}

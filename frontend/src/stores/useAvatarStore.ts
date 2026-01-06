import { create, StateCreator } from 'zustand';
import { persist, PersistOptions } from 'zustand/middleware';

const STORAGE_KEY = 'vibe-kanban:agent-avatars';

/**
 * Map of agent names to their avatar base64 data URLs.
 * Agent names include specific agent names (e.g., "Ivy", "Bree", "Nova")
 * plus "Claude" for the orchestrator.
 */
export type AvatarMap = Record<string, string>;

interface AvatarState {
  avatars: AvatarMap;
  setAvatar: (agentName: string, imageDataUrl: string) => void;
  removeAvatar: (agentName: string) => void;
  clearAvatars: () => void;
}

// Define the persisted state (subset of full state)
type PersistedState = Pick<AvatarState, 'avatars'>;

/**
 * Safe localStorage wrapper that handles errors gracefully.
 * Catches quota exceeded errors and other localStorage issues
 * (e.g., private browsing mode).
 */
const safeLocalStorage = {
  getItem: (name: string): string | null => {
    try {
      return localStorage.getItem(name);
    } catch (error) {
      console.warn('[AvatarStore] Failed to read from localStorage:', error);
      return null;
    }
  },
  setItem: (name: string, value: string): void => {
    try {
      localStorage.setItem(name, value);
    } catch (error) {
      // Handle quota exceeded or other localStorage errors
      if (
        error instanceof DOMException &&
        (error.name === 'QuotaExceededError' ||
          error.name === 'NS_ERROR_DOM_QUOTA_REACHED')
      ) {
        console.warn(
          '[AvatarStore] localStorage quota exceeded. Avatar not saved.'
        );
      } else {
        console.warn('[AvatarStore] Failed to write to localStorage:', error);
      }
    }
  },
  removeItem: (name: string): void => {
    try {
      localStorage.removeItem(name);
    } catch (error) {
      console.warn('[AvatarStore] Failed to remove from localStorage:', error);
    }
  },
};

const avatarStateCreator: StateCreator<AvatarState> = (set) => ({
  avatars: {},

  setAvatar: (agentName: string, imageDataUrl: string) =>
    set((state) => ({
      avatars: { ...state.avatars, [agentName]: imageDataUrl },
    })),

  removeAvatar: (agentName: string) =>
    set((state) => {
      const newAvatars = { ...state.avatars };
      delete newAvatars[agentName];
      return { avatars: newAvatars };
    }),

  clearAvatars: () => set({ avatars: {} }),
});

const persistOptions: PersistOptions<AvatarState, PersistedState> = {
  name: STORAGE_KEY,
  storage: {
    getItem: (name) => {
      const str = safeLocalStorage.getItem(name);
      if (!str) return null;
      try {
        return JSON.parse(str);
      } catch (error) {
        console.warn(
          '[AvatarStore] Failed to parse localStorage data:',
          error
        );
        return null;
      }
    },
    setItem: (name, value) => {
      try {
        safeLocalStorage.setItem(name, JSON.stringify(value));
      } catch (error) {
        console.warn('[AvatarStore] Failed to stringify state:', error);
      }
    },
    removeItem: (name) => {
      safeLocalStorage.removeItem(name);
    },
  },
  // Only persist the avatars map, not the actions
  partialize: (state) => ({ avatars: state.avatars }),
  // Handle invalid data gracefully during rehydration
  onRehydrateStorage: () => (state, error) => {
    if (error) {
      console.warn(
        '[AvatarStore] Failed to rehydrate from localStorage:',
        error
      );
    }
    if (state && typeof state.avatars !== 'object') {
      console.warn(
        '[AvatarStore] Invalid avatar data in localStorage, resetting.'
      );
      state.avatars = {};
    }
  },
};

export const useAvatarStore = create<AvatarState>()(
  persist(avatarStateCreator, persistOptions)
);

// Selector hooks for optimized re-renders
export const useAvatars = () => useAvatarStore((state) => state.avatars);
export const useAvatar = (agentName: string) =>
  useAvatarStore((state) => state.avatars[agentName] ?? null);

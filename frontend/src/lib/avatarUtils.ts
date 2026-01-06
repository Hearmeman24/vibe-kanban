import { useAvatarStore } from '../stores/useAvatarStore';

/**
 * Save an agent's avatar to persistent storage.
 * Handles localStorage errors gracefully (quota limits, private browsing).
 *
 * @param agentName - The name of the agent (e.g., "Ivy", "Claude")
 * @param imageDataUrl - Base64 data URL of the avatar image
 */
export function saveAgentAvatar(
  agentName: string,
  imageDataUrl: string
): void {
  try {
    useAvatarStore.getState().setAvatar(agentName, imageDataUrl);
  } catch (error) {
    console.warn(
      `[avatarUtils] Failed to save avatar for "${agentName}":`,
      error
    );
  }
}

/**
 * Get an agent's avatar from persistent storage.
 *
 * @param agentName - The name of the agent
 * @returns The avatar as a base64 data URL, or null if not found
 */
export function getAgentAvatar(agentName: string): string | null {
  try {
    const avatars = useAvatarStore.getState().avatars;
    return avatars[agentName] ?? null;
  } catch (error) {
    console.warn(
      `[avatarUtils] Failed to get avatar for "${agentName}":`,
      error
    );
    return null;
  }
}

/**
 * Get the initial character of an agent's name as a fallback for avatars.
 * Returns uppercase first letter.
 *
 * @param agentName - The name of the agent
 * @returns The first character uppercase, or "?" if name is empty
 */
export function getAgentInitial(agentName: string): string {
  const trimmed = agentName.trim();
  if (!trimmed) {
    return '?';
  }
  return trimmed.charAt(0).toUpperCase();
}

/**
 * Remove an agent's avatar from persistent storage.
 *
 * @param agentName - The name of the agent
 */
export function removeAgentAvatar(agentName: string): void {
  try {
    useAvatarStore.getState().removeAvatar(agentName);
  } catch (error) {
    console.warn(
      `[avatarUtils] Failed to remove avatar for "${agentName}":`,
      error
    );
  }
}

/**
 * Check if an agent has a stored avatar.
 *
 * @param agentName - The name of the agent
 * @returns True if the agent has a stored avatar
 */
export function hasAgentAvatar(agentName: string): boolean {
  try {
    const avatars = useAvatarStore.getState().avatars;
    return agentName in avatars;
  } catch {
    return false;
  }
}

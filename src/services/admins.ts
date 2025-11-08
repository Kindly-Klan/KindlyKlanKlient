import { invoke } from '@tauri-apps/api/core';

// Cache for admin status during session
let adminCache: Map<string, boolean> = new Map();

export class AdminService {
  /**
   * Check if a username has admin privileges
   * Caches result during the session for performance
   */
  static async checkIsAdmin(username: string): Promise<boolean> {
    try {
      // Check cache first
      if (adminCache.has(username)) {
        return adminCache.get(username)!;
      }

      // Query backend
      const isAdmin = await invoke<boolean>('check_is_admin', { username });
      
      // Cache the result
      adminCache.set(username, isAdmin);
      
      console.log(`Admin check for ${username}: ${isAdmin}`);
      return isAdmin;
    } catch (error) {
      console.error('Error checking admin status:', error);
      // On error, assume not admin for safety
      return false;
    }
  }

  /**
   * Clear the admin cache (useful when user changes or logs out)
   */
  static clearCache(): void {
    adminCache.clear();
    console.log('Admin cache cleared');
  }

  /**
   * Remove a specific user from the cache
   */
  static clearUserCache(username: string): void {
    adminCache.delete(username);
    console.log(`Admin cache cleared for user: ${username}`);
  }
}


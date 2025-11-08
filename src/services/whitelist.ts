import { invoke } from '@tauri-apps/api/core';
import type { AccessCheck } from '@/types/whitelist';

export class WhitelistService {
  /**
   * Check if a username has access to the launcher
   * Always queries the database directly 
   */
  static async checkAccess(username: string): Promise<AccessCheck> {
    try {
      // Always query database directly - no cache
      const result = await invoke<AccessCheck>('check_whitelist_access', { username });
      return result;
    } catch (error) {
      console.error('Error checking whitelist access:', error);
      // Return no access on error
      return {
        has_access: false,
        allowed_instances: [],
        global_access: false
      };
    }
  }

  /**
   * Get accessible instances for a user
   */
  static async getAccessibleInstances(username: string, allInstances: any[]): Promise<any[]> {
    try {
      const instanceIds = allInstances.map(instance => instance.id);
      const accessibleIds = await invoke<string[]>('get_accessible_instances', { 
        username, 
        allInstances: instanceIds 
      });

      // Filter instances based on accessible IDs
      return allInstances.filter(instance => accessibleIds.includes(instance.id));
    } catch (error) {
      console.error('Error getting accessible instances:', error);
      return [];
    }
  }

  /**
   * Clear the whitelist cache (no-op, cache is disabled)
   */
  static async clearCache(): Promise<void> {
    try {
      await invoke('clear_whitelist_cache');
      // Cache is disabled - always queries database
    } catch (error) {
      console.error('Error clearing whitelist cache:', error);
    }
  }

  /**
   * Check if user has access to a specific instance
   */
  static async hasInstanceAccess(username: string, instanceId: string): Promise<boolean> {
    const accessCheck = await this.checkAccess(username);
    
    if (!accessCheck.has_access) {
      return false;
    }

    if (accessCheck.global_access) {
      return true;
    }

    return accessCheck.allowed_instances.includes(instanceId);
  }
}

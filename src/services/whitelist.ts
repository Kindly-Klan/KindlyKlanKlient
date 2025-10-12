import { invoke } from '@tauri-apps/api/core';
import type { AccessCheck } from '@/types/whitelist';

export class WhitelistService {
  private static cache: Map<string, { data: AccessCheck; timestamp: number }> = new Map();
  private static readonly CACHE_TTL = 5 * 60 * 1000; // 5 minutes in milliseconds

  /**
   * Check if a username has access to the launcher
   */
  static async checkAccess(username: string): Promise<AccessCheck> {
    // Check cache first
    const cached = this.cache.get(username);
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL) {
      return cached.data;
    }

    try {
      const result = await invoke<AccessCheck>('check_whitelist_access', { username });
      
      // Cache the result
      this.cache.set(username, {
        data: result,
        timestamp: Date.now()
      });

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
   * Clear the whitelist cache
   */
  static async clearCache(): Promise<void> {
    try {
      await invoke('clear_whitelist_cache');
      this.cache.clear();
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

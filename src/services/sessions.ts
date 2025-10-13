import { invoke } from '@tauri-apps/api/core';

export interface Session {
  id: string;
  username: string;
  access_token: string;
  refresh_token: string | null;
  expires_at: number; // Unix timestamp in seconds
  created_at: number; // Unix timestamp in seconds
  updated_at: number; // Unix timestamp in seconds
}

export class SessionService {
  static async saveSession(
    username: string,
    accessToken: string,
    refreshToken: string | null,
    expiresAt: number
  ): Promise<string> {
    return await invoke<string>('save_session', {
      username,
      accessToken,
      refreshToken,
      expiresAt
    });
  }

  static async getSession(username: string): Promise<Session | null> {
    return await invoke<Session | null>('get_session', { username });
  }

  static async getActiveSession(): Promise<Session | null> {
    return await invoke<Session | null>('get_active_session');
  }

  static async updateSession(session: Session): Promise<string> {
    return await invoke<string>('update_session', { session });
  }

  static async deleteSession(username: string): Promise<string> {
    return await invoke<string>('delete_session', { username });
  }

  static async clearAllSessions(): Promise<string> {
    return await invoke<string>('clear_all_sessions');
  }

  static async cleanupExpiredSessions(): Promise<number> {
    return await invoke<number>('cleanup_expired_sessions');
  }

  static async debugSessions(): Promise<string> {
    return await invoke<string>('debug_sessions');
  }

  static isSessionExpired(session: Session): boolean {
    return session.expires_at < Date.now() / 1000;
  }

  static isSessionExpiringSoon(session: Session, minutesThreshold: number = 10): boolean {
    const threshold = (Date.now() / 1000) + (minutesThreshold * 60);
    return session.expires_at <= threshold;
  }
}

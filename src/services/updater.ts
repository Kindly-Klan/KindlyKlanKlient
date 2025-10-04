import { invoke } from '@tauri-apps/api/core';

export interface UpdateInfo {
  version: string;
  available: boolean;
  message: string;
}

export class UpdaterService {
  /**
   * Verifica si hay actualizaciones disponibles
   */
  static async checkForUpdates(): Promise<UpdateInfo> {
    try {
      const result = await invoke<string>('check_for_updates');
      const available = !result.includes('No updates available');

      return {
        version: available ? result.replace('Update available: ', '') : '',
        available,
        message: result
      };
    } catch (error) {
      console.error('Error checking for updates:', error);
      return {
        version: '',
        available: false,
        message: 'Error al verificar actualizaciones'
      };
    }
  }

  /**
   * Instala la actualización disponible
   */
  static async installUpdate(): Promise<{ success: boolean; message: string }> {
    try {
      const result = await invoke<string>('install_update');
      return {
        success: result.includes('successfully') || result.includes('No updates available'),
        message: result
      };
    } catch (error) {
      console.error('Error installing update:', error);
      return {
        success: false,
        message: 'Error al instalar la actualización'
      };
    }
  }

  /**
   * Obtiene la versión actual de la aplicación
   */
  static async getCurrentVersion(): Promise<string> {
    try {
      // Esta función podría necesitar ser implementada en Rust
      // Por ahora devolvemos una versión por defecto
      return '0.1.3';
    } catch (error) {
      console.error('Error getting current version:', error);
      return '0.1.3';
    }
  }
}

import { SkinData } from '@/types/skin';

const SKINS_STORAGE_KEY = 'kkk_user_skins';
const ACTIVE_SKIN_KEY = 'kkk_active_skin';

export class SkinStorageService {
  /**
   * Guarda una skin en el almacenamiento local
   */
  static async saveSkin(skinData: SkinData): Promise<void> {
    try {
      const existingSkins = await this.getStoredSkins();
      
      // Preparar datos para serialización (sin File object)
      const serializableSkin = {
        ...skinData,
        file: undefined, // No serializar File object
        fileData: skinData.fileData ? Array.from(new Uint8Array(skinData.fileData)) : undefined // Convertir ArrayBuffer a array
      };
      
      const updatedSkins = [...existingSkins.filter(s => s.id !== skinData.id), serializableSkin];

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(updatedSkins));
      console.log('Skin guardada localmente:', skinData.id);
    } catch (error) {
      console.error('Error guardando skin localmente:', error);
      throw new Error('Error al guardar skin localmente');
    }
  }

  /**
   * Obtiene todas las skins almacenadas localmente
   */
  static async getStoredSkins(): Promise<SkinData[]> {
    try {
      const stored = localStorage.getItem(SKINS_STORAGE_KEY);
      if (!stored) return [];

      const skins = JSON.parse(stored);
      // Convertir fechas de vuelta a objetos Date y ArrayBuffer
      return skins.map((skin: any) => ({
        ...skin,
        uploadedAt: new Date(skin.uploadedAt),
        fileData: skin.fileData ? new Uint8Array(skin.fileData).buffer : undefined
      }));
    } catch (error) {
      console.error('Error cargando skins locales:', error);
      return [];
    }
  }

  /**
   * Elimina una skin del almacenamiento local
   */
  static async deleteSkin(skinId: string): Promise<void> {
    try {
      const skins = await this.getStoredSkins();
      const filteredSkins = skins.filter(skin => skin.id !== skinId);

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(filteredSkins));
      console.log('Skin eliminada localmente:', skinId);
    } catch (error) {
      console.error('Error eliminando skin localmente:', error);
      throw new Error('Error al eliminar skin localmente');
    }
  }

  /**
   * Establece una skin como activa
   */
  static async setActiveSkin(skinId: string): Promise<void> {
    try {
      const skins = await this.getStoredSkins();

      // Marcar todas como inactivas y la seleccionada como activa
      const updatedSkins = skins.map(skin => ({
        ...skin,
        isActive: skin.id === skinId
      }));

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(updatedSkins));
      localStorage.setItem(ACTIVE_SKIN_KEY, skinId);

      console.log('Skin activa establecida:', skinId);
    } catch (error) {
      console.error('Error estableciendo skin activa:', error);
      throw new Error('Error al establecer skin activa');
    }
  }

  /**
   * Obtiene la skin activa actualmente
   */
  static async getActiveSkin(): Promise<SkinData | null> {
    try {
      const activeSkinId = localStorage.getItem(ACTIVE_SKIN_KEY);
      if (!activeSkinId) return null;

      const skins = await this.getStoredSkins();
      return skins.find(skin => skin.id === activeSkinId) || null;
    } catch (error) {
      console.error('Error obteniendo skin activa:', error);
      return null;
    }
  }

  /**
   * Obtiene la URL de Crafatar para previsualización 3D
   */
  static getCrafatarPreviewUrl(uuid: string): string {
    return `https://crafatar.com/renders/body/${uuid}?overlay=true`;
  }

  /**
   * Obtiene la URL de Crafatar para la cabeza (avatar)
   */
  static getCrafatarHeadUrl(uuid: string, size: number = 40): string {
    return `https://crafatar.com/avatars/${uuid}?size=${size}&overlay=true`;
  }

  /**
   * Limpia todas las skins almacenadas (útil para desarrollo)
   */
  static async clearAllSkins(): Promise<void> {
    try {
      localStorage.removeItem(SKINS_STORAGE_KEY);
      localStorage.removeItem(ACTIVE_SKIN_KEY);
      console.log('Todas las skins eliminadas');
    } catch (error) {
      console.error('Error limpiando skins:', error);
      throw new Error('Error al limpiar skins');
    }
  }
}

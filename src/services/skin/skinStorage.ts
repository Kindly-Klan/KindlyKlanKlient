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
      
      
      const serializableSkin = {
        ...skinData,
        file: undefined, 
        fileData: skinData.fileData ? Array.from(new Uint8Array(skinData.fileData)) : undefined 
      };
      
      const updatedSkins = [...existingSkins.filter(s => s.id !== skinData.id), serializableSkin];

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(updatedSkins));
      console.log('Skin saved locally:', skinData.id);
    } catch (error) {
      console.error('Error saving skin locally:', error);
      throw new Error('Error al guardar skin localmente');
    }
  }


  static async getStoredSkins(): Promise<SkinData[]> {
    try {
      const stored = localStorage.getItem(SKINS_STORAGE_KEY);
      if (!stored) return [];

      const skins = JSON.parse(stored);
      
      return skins.map((skin: any) => ({
        ...skin,
        uploadedAt: new Date(skin.uploadedAt),
        fileData: skin.fileData ? new Uint8Array(skin.fileData).buffer : undefined
      }));
    } catch (error) {
      console.error('Error loading local skins:', error);
      return [];
    }
  }


  static async deleteSkin(skinId: string): Promise<void> {
    try {
      const skins = await this.getStoredSkins();
      const filteredSkins = skins.filter(skin => skin.id !== skinId);

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(filteredSkins));
      console.log('Skin deleted locally:', skinId);
    } catch (error) {
      console.error('Error deleting skin locally:', error);
      throw new Error('Error al eliminar skin localmente');
    }
  }


    
  static async setActiveSkin(skinId: string): Promise<void> {
    try {
      const skins = await this.getStoredSkins();

      
      const updatedSkins = skins.map(skin => ({
        ...skin,
        isActive: skin.id === skinId
      }));

      localStorage.setItem(SKINS_STORAGE_KEY, JSON.stringify(updatedSkins));
      localStorage.setItem(ACTIVE_SKIN_KEY, skinId);

      console.log('Skin active set:', skinId);
    } catch (error) {
      console.error('Error setting active skin:', error);
      throw new Error('Error al establecer skin activa');
    }
  }


  static async getActiveSkin(): Promise<SkinData | null> {
    try {
      const activeSkinId = localStorage.getItem(ACTIVE_SKIN_KEY);
      if (!activeSkinId) return null;

      const skins = await this.getStoredSkins();
      return skins.find(skin => skin.id === activeSkinId) || null;
    } catch (error) {
      console.error('Error getting active skin:', error);
      return null;
    }
  }


  static getCrafatarPreviewUrl(uuid: string): string {
    return `https://crafatar.com/renders/body/${uuid}?overlay=true`;
  }


  static getCrafatarHeadUrl(uuid: string, size: number = 40): string {
    return `https://crafatar.com/avatars/${uuid}?size=${size}&overlay=true`;
  }


  static async clearAllSkins(): Promise<void> {
    try {
      localStorage.removeItem(SKINS_STORAGE_KEY);
      localStorage.removeItem(ACTIVE_SKIN_KEY);
      console.log('All skins deleted');
    } catch (error) {
      console.error('Error clearing skins:', error);
      throw new Error('Error al limpiar skins');
    }
  }
}

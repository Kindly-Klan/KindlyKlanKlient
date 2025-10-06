import { SkinData } from '@/types/skin';
import { invoke } from '@tauri-apps/api/core';

// Mojang skin API service
export class MojangSkinApiService {
  // Upload skin directly to Mojang API
  static async uploadSkin(file: File, variant: 'classic' | 'slim' = 'classic'): Promise<SkinData> {
    try {
      const validation = this.validateSkinFile(file);
      if (!validation.valid) {
        throw new Error(validation.error);
      }

      console.log('Uploading skin to Mojang API...', { variant, fileSize: file.size });

      const authSession = await this.getAuthSession();

      if (!authSession?.access_token) {
        throw new Error('User not authenticated. Please log in to change your skin.');
      }

      const tempFilePath = await this.createTempFile(file);

      const result = await invoke<string>('upload_skin_to_mojang', {
        filePath: tempFilePath,
        variant,
        accessToken: authSession.access_token
      });

      console.log('Skin uploaded successfully to Mojang:', result);

      const skinData: SkinData = {
        id: `skin_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        name: file.name,
        file,
        url: '',
        textureId: '',
        variant,
        uploadedAt: new Date(),
        isActive: true
      };

      return skinData;

    } catch (error) {
      console.error('Error uploading skin to Mojang:', error);
      throw new Error(`Failed to upload skin: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  // Get current authentication session
  private static async getAuthSession(): Promise<any> {
    try {
      const savedSession = localStorage.getItem('kkk_session');
      if (savedSession) {
        return JSON.parse(savedSession);
      }
      return null;
    } catch (error) {
      console.error('Error getting authentication session:', error);
      return null;
    }
  }

  // Create temporary file for backend to read
  private static async createTempFile(_file: File): Promise<string> {
    try {
      const tempFileName = `skin_${Date.now()}_${Math.random().toString(36).substr(2, 9)}.png`;
      const arrayBuffer = await _file.arrayBuffer();
      const tempPath = await invoke<string>('create_temp_file', {
        fileName: tempFileName,
        fileData: arrayBuffer
      });
      return tempPath;
    } catch (error) {
      console.error('Error creating temporary file:', error);
      throw new Error('Failed to create temporary file');
    }
  }

  // Validate skin file (PNG, <24KB)
  static validateSkinFile(file: File): { valid: boolean; error?: string } {
    if (file.type !== 'image/png') {
      return { valid: false, error: 'File must be a PNG image' };
    }

    if (file.size > 24 * 1024) {
      return { valid: false, error: 'Image must be smaller than 24KB' };
    }

    return { valid: true };
  }
}

import { SkinData } from '@/types/skin';
import { invoke } from '@tauri-apps/api/core';

export class MojangSkinApiService {
  /**
   * Sube una imagen de skin directamente a la API de Mojang
   */
  static async uploadSkin(file: File, variant: 'classic' | 'slim' = 'classic'): Promise<SkinData> {
    try {
      // Validar archivo primero
      const validation = this.validateSkinFile(file);
      if (!validation.valid) {
        throw new Error(validation.error);
      }

      console.log('Subiendo skin a Mojang API...', { variant, fileSize: file.size });

      // Obtener token de acceso del usuario autenticado
      const authSession = await this.getAuthSession();

      if (!authSession?.access_token) {
        throw new Error('Usuario no autenticado. Inicia sesión para cambiar tu skin.');
      }

      // Crear archivo temporal para el backend
      const tempFilePath = await this.createTempFile(file);

      // Subir directamente a Mojang usando Tauri backend
      const result = await invoke<string>('upload_skin_to_mojang', {
        filePath: tempFilePath,
        variant,
        accessToken: authSession.access_token
      });

      console.log('Skin subida exitosamente a Mojang:', result);

      // Crear objeto SkinData
      const skinData: SkinData = {
        id: `skin_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        name: file.name,
        file,
        url: '', // Mojang no devuelve URL directa
        textureId: '', // No necesitamos ID ya que se aplica directamente
        variant,
        uploadedAt: new Date(),
        isActive: true // Se aplica inmediatamente
      };

      return skinData;

    } catch (error) {
      console.error('Error subiendo skin a Mojang:', error);
      throw new Error(`Error al subir skin: ${error instanceof Error ? error.message : 'Error desconocido'}`);
    }
  }

  /**
   * Obtiene la sesión de autenticación actual
   */
  private static async getAuthSession(): Promise<any> {
    try {
      // Intentar obtener sesión guardada
      const savedSession = localStorage.getItem('kkk_session');
      if (savedSession) {
        return JSON.parse(savedSession);
      }
      return null;
    } catch (error) {
      console.error('Error obteniendo sesión de autenticación:', error);
      return null;
    }
  }

  /**
   * Crea un archivo temporal en el sistema de archivos para que el backend pueda leerlo
   */
  private static async createTempFile(_file: File): Promise<string> {
    try {
      // Crear archivo temporal real mediante comando Tauri
      const tempFileName = `skin_${Date.now()}_${Math.random().toString(36).substr(2, 9)}.png`;
      const arrayBuffer = await _file.arrayBuffer();
      const tempPath = await invoke<string>('create_temp_file', {
        fileName: tempFileName,
        fileData: arrayBuffer
      });
      return tempPath;
    } catch (error) {
      console.error('Error creando archivo temporal:', error);
      throw new Error('Error creando archivo temporal');
    }
  }

  /**
   * Valida que el archivo sea una imagen PNG válida de 64x64 píxeles
   */
  static validateSkinFile(file: File): { valid: boolean; error?: string } {
    // Verificar tipo MIME
    if (file.type !== 'image/png') {
      return { valid: false, error: 'El archivo debe ser una imagen PNG' };
    }

    // Verificar tamaño (máximo 24KB para Mojang)
    if (file.size > 24 * 1024) {
      return { valid: false, error: 'La imagen debe ser menor de 24KB' };
    }

    return { valid: true };
  }
}

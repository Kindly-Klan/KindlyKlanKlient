import React, { useState, useEffect, useCallback, useRef } from 'react';
import { SkinPreview3D } from './SkinPreview3D';
import { SkinData, SkinModel } from '@/types/skin';
import { SkinStorageService } from '@/services/skin/skinStorage';
import { invoke } from '@tauri-apps/api/core';
import { useDropzone } from 'react-dropzone';

interface SkinManagerProps {
  currentUser: any;
  addToast?: (message: string, type?: 'success' | 'error' | 'info', duration?: number) => void;
}

interface MojangSkin {
  id: string;
  state: string;
  url: string;
  variant: 'CLASSIC' | 'SLIM';
}

interface MojangProfile {
  id: string;
  name: string;
  skins?: MojangSkin[];
}

interface EnsureSessionResponse {
  status: string;
  data?: {
    session: {
      access_token: string;
      refresh_token: string | null;
      expires_at: number;
    };
    refreshed: boolean;
  };
  code?: string;
  message?: string;
}

interface ProfileResponse {
  status: string;
  profile?: any;
  code?: string;
  message?: string;
}

// Función para refrescar avatares añadiendo timestamp (SOLO Crafatar, no todas las imágenes)
const refreshAvatars = () => {
  const timestamp = Date.now();
  
  // Refrescar SOLO las imágenes de Crafatar (no tocar otras imágenes)
  document.querySelectorAll('img[src*="crafatar.com"]').forEach((img: any) => {
    try {
      const url = new URL(img.src);
      url.searchParams.set('t', timestamp.toString());
      img.src = url.toString();
    } catch (e) {
      // Si falla, forzar recarga añadiendo timestamp al final
      const separator = img.src.includes('?') ? '&' : '?';
      img.src = `${img.src}${separator}t=${timestamp}`;
    }
  });
  
  console.log('✅ Avatares de Crafatar refrescados con timestamp:', timestamp);
};

export const SkinManager: React.FC<SkinManagerProps> = ({ currentUser, addToast }) => {
  const [skins, setSkins] = useState<SkinData[]>([]);
  const [selectedSkinId, setSelectedSkinId] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [isLoadingInitial, setIsLoadingInitial] = useState(true);
  const hasInitialized = useRef(false);
  const blobUrlsRef = useRef<Map<string, string>>(new Map());

  const getSkinUrl = useCallback((skin: SkinData): string => {
    // PRIORIDAD 1: Si ya tenemos blob URL guardado, usarlo SIEMPRE
    if (blobUrlsRef.current.has(skin.id)) {
      return blobUrlsRef.current.get(skin.id)!;
    }

    // PRIORIDAD 2: Si tiene fileData, crear blob URL (incluso si también tiene URL)
    if (skin.fileData && skin.fileData instanceof ArrayBuffer && skin.fileData.byteLength > 0) {
      try {
        const uint8Array = new Uint8Array(skin.fileData);
        const buffer = new ArrayBuffer(uint8Array.length);
        const view = new Uint8Array(buffer);
        view.set(uint8Array);
        const blob = new Blob([buffer], { type: 'image/png' });
        const blobUrl = URL.createObjectURL(blob);
        blobUrlsRef.current.set(skin.id, blobUrl);
        return blobUrl;
      } catch (err) {
        // Si falla, continuar con URL de Mojang si existe
      }
    }

    // PRIORIDAD 3: Si tiene URL de Mojang, usarla
    if (skin.url && skin.url.trim() !== '') {
      return skin.url;
    }
    
    // FALLBACK: Crafatar
    return `https://crafatar.com/skins/${currentUser?.uuid || 'default'}`;
  }, [currentUser?.uuid]);

  // Limpiar blob URLs solo cuando se elimine una skin específica
  // NO limpiar al desmontar porque necesitamos mantenerlos entre recargas

  // Función para obtener token válido de Minecraft - con protección contra llamadas repetidas
  const tokenRequestRef = useRef<Promise<string | null> | null>(null);
  const getValidMinecraftToken = useCallback(async (): Promise<string | null> => {
    // Si ya hay una petición en curso, esperar a que termine
    if (tokenRequestRef.current) {
      return tokenRequestRef.current;
    }

    const requestPromise = (async () => {
      try {
        const savedSession = localStorage.getItem('kkk_session');
        if (!savedSession) {
          console.warn('⚠️ No hay sesión guardada');
          return null;
        }

        let session;
        try {
          session = JSON.parse(savedSession);
        } catch (parseError) {
          console.error('❌ Error al parsear sesión:', parseError);
          return null;
        }

        if (!session?.username) {
          console.warn('⚠️ Sesión no tiene username');
          return null;
        }

        // Validar/renovar la sesión
        const sessionResponse: EnsureSessionResponse = await invoke('ensure_valid_session', {
          username: session.username
        });

        if (sessionResponse.status === 'Ok' && sessionResponse.data?.session) {
          const validToken = sessionResponse.data.session.access_token;
          
          // Actualizar localStorage con el token renovado
          const updatedSession = {
            ...session,
            access_token: validToken,
            expires_at: sessionResponse.data.session.expires_at,
            refresh_token: sessionResponse.data.session.refresh_token
          };
          localStorage.setItem('kkk_session', JSON.stringify(updatedSession));
          
          return validToken;
        }

        console.warn('⚠️ La sesión no es válida:', sessionResponse.status);
        return null;
      } catch (error) {
        console.error('❌ Error al obtener token:', error);
        return null;
      } finally {
        // Limpiar la referencia después de un momento para permitir nuevas peticiones
        setTimeout(() => {
          tokenRequestRef.current = null;
        }, 5000);
      }
    })();

    tokenRequestRef.current = requestPromise;
    return requestPromise;
  }, []);

  // Sincronizar skin activa con Mojang (con mejor manejo de errores y sin bloquear)
  const syncWithMojang = useCallback(async (localSkins: SkinData[]): Promise<void> => {
    // No sincronizar si no hay skins locales
    if (!localSkins || localSkins.length === 0) {
      return;
    }

    try {
      // Intentar obtener token con timeout
      const tokenPromise = getValidMinecraftToken();
      const timeoutPromise = new Promise<string | null>((resolve) => 
        setTimeout(() => resolve(null), 5000)
      );
      
      const accessToken = await Promise.race([tokenPromise, timeoutPromise]);
      
      if (!accessToken) {
        // No mostrar error, simplemente no sincronizar si no hay token
        return;
      }

      // Obtener perfil de Mojang con timeout
      const profilePromise = invoke<ProfileResponse>('get_minecraft_profile_safe', {
        accessToken
      });
      const profileTimeoutPromise = new Promise<ProfileResponse>((resolve) => 
        setTimeout(() => resolve({ status: 'Err', profile: undefined, code: 'TIMEOUT', message: 'Timeout' }), 8000)
      );
      
      const profileResponse = await Promise.race([profilePromise, profileTimeoutPromise]);

      if (profileResponse.status !== 'Ok' || !profileResponse.profile) {
        // No mostrar error, simplemente no sincronizar
        return;
      }

      const profile = profileResponse.profile as any;
      const mojangSkins = profile.skins || [];
      const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');

      if (!activeMojangSkin) {
        // No hay skin activa en Mojang, desmarcar todas las locales (solo una vez)
        const hasActiveLocal = localSkins.some(s => s.isActive);
        if (hasActiveLocal) {
          await SkinStorageService.setActiveSkin(''); // Desmarcar todas
        }
        return;
      }

      // Buscar si alguna skin local coincide con la activa de Mojang
      const mojangSkinUrl = activeMojangSkin.url;
      const mojangTextureId = activeMojangSkin.id;

      const matchingLocalSkin = localSkins.find(skin => {
        // Comparar por URL
        if (skin.url && skin.url === mojangSkinUrl) {
          return true;
        }
        // Comparar por textureId
        if (skin.textureId && skin.textureId === mojangTextureId) {
          return true;
        }
        return false;
      });

      if (matchingLocalSkin) {
        // La skin activa de Mojang coincide con una local, marcarla como activa
        if (!matchingLocalSkin.isActive) {
          await SkinStorageService.setActiveSkin(matchingLocalSkin.id);
        }
      } else {
        // La skin activa de Mojang NO coincide con ninguna local, desmarcar todas (solo una vez)
        const hasActiveLocal = localSkins.some(s => s.isActive);
        if (hasActiveLocal) {
          await SkinStorageService.setActiveSkin(''); // Desmarcar todas
        }
      }
    } catch (error) {
      // Silenciar errores de sincronización, es una operación en segundo plano
      // No mostrar error al usuario ni en consola para evitar spam
    }
  }, [getValidMinecraftToken]);

  // Cargar skins al montar
  useEffect(() => {
    if (hasInitialized.current) return;
    hasInitialized.current = true;
    
    const initializeSkins = async () => {
      try {
        // 1. Cargar skins guardadas localmente PRIMERO
        const savedSkins = await SkinStorageService.getStoredSkins();
        const activeSkin = await SkinStorageService.getActiveSkin();
        
        // Crear blob URLs para TODAS las skins que tienen fileData
        // Solo crear si no existe ya (para evitar recrear innecesariamente)
        savedSkins.forEach(skin => {
          if (skin.fileData && skin.fileData instanceof ArrayBuffer && skin.fileData.byteLength > 0) {
            // Solo crear si no existe ya
            if (!blobUrlsRef.current.has(skin.id)) {
              try {
                const uint8Array = new Uint8Array(skin.fileData);
                const buffer = new ArrayBuffer(uint8Array.length);
                const view = new Uint8Array(buffer);
                view.set(uint8Array);
                const blob = new Blob([buffer], { type: 'image/png' });
                const blobUrl = URL.createObjectURL(blob);
                blobUrlsRef.current.set(skin.id, blobUrl);
              } catch (err) {
                console.error('Error creando blob URL para skin:', skin.id, err);
              }
            }
          }
        });
        
        // Pequeño delay para asegurar que los blob URLs estén completamente listos
        await new Promise(resolve => setTimeout(resolve, 50));
        
        // Establecer estado inicial PRIMERO (sin bloquear)
        setSkins([...savedSkins]);
        setSelectedSkinId(activeSkin?.id || null);
        setIsLoadingInitial(false);

        // 2. Sincronizar con Mojang en segundo plano DESPUÉS de un delay (no bloquear UI)
        // Esperar 2 segundos para asegurar que la sesión esté lista
        setTimeout(() => {
          syncWithMojang(savedSkins).then(() => {
            // Después de sincronizar, recargar skins para reflejar cambios
            SkinStorageService.getStoredSkins().then(updatedSkins => {
              const updatedActiveSkin = updatedSkins.find(s => s.isActive);
              setSkins([...updatedSkins]);
              setSelectedSkinId(updatedActiveSkin?.id || null);
            }).catch(() => {
              // Ignorar errores al recargar
            });
          }).catch(() => {
            // Ignorar errores de sincronización
          });
        }, 2000);

        // 3. Refrescar avatares después de un delay más largo
        setTimeout(() => {
          refreshAvatars();
        }, 1000);
      } catch (error) {
        // Error crítico al cargar skins desde localStorage
        console.error('❌ Error al cargar skins desde localStorage:', error);
        addToast?.('Error al cargar skins guardadas', 'error');
        setIsLoadingInitial(false);
      }
    };

    initializeSkins();
    
    // Resetear al desmontar para que se recargue al volver a montar
    return () => {
      hasInitialized.current = false;
    };
  }, [getValidMinecraftToken, addToast, syncWithMojang]);

  // Sincronizar con Mojang cuando el usuario vuelve a la pestaña (con debounce para evitar spam)
  useEffect(() => {
    let syncTimeout: NodeJS.Timeout | null = null;
    let lastSyncTime = 0;
    const MIN_SYNC_INTERVAL = 10000; // Mínimo 10 segundos entre sincronizaciones

    const handleVisibilityChange = () => {
      if (!document.hidden && currentUser) {
        const now = Date.now();
        // Solo sincronizar si han pasado al menos 10 segundos desde la última vez
        if (now - lastSyncTime < MIN_SYNC_INTERVAL) {
          return;
        }

        // Limpiar timeout anterior si existe
        if (syncTimeout) {
          clearTimeout(syncTimeout);
        }

        // Esperar 1 segundo antes de sincronizar (para evitar sincronizaciones inmediatas)
        syncTimeout = setTimeout(() => {
          lastSyncTime = Date.now();
          SkinStorageService.getStoredSkins().then(localSkins => {
            syncWithMojang(localSkins).then(() => {
              // Recargar skins después de sincronizar
              SkinStorageService.getStoredSkins().then(updatedSkins => {
                const updatedActiveSkin = updatedSkins.find(s => s.isActive);
                setSkins([...updatedSkins]);
                setSelectedSkinId(updatedActiveSkin?.id || null);
                // Refrescar avatares después de un delay
                setTimeout(() => {
                  refreshAvatars();
                }, 500);
              }).catch(() => {
                // Ignorar errores
              });
            }).catch(() => {
              // Ignorar errores de sincronización
            });
          }).catch(() => {
            // Ignorar errores
          });
        }, 1000);
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    
    return () => {
      if (syncTimeout) {
        clearTimeout(syncTimeout);
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [currentUser, syncWithMojang]);

  // Subir nueva skin
  const handleUploadSkin = useCallback(async (file: File) => {
    if (!file) return;

    // Validaciones
    if (file.type !== 'image/png') {
      addToast?.('Solo se permiten archivos PNG', 'error');
      return;
    }

    if (file.size > 24 * 1024) {
      addToast?.('El archivo debe ser menor a 24KB', 'error');
      return;
    }

    setIsUploading(true);
    addToast?.('Subiendo skin a Mojang...', 'info', 3000);

    try {
      // Obtener token válido
      const accessToken = await getValidMinecraftToken();
      if (!accessToken) {
        throw new Error('No se pudo obtener un token válido. Por favor, reinicia sesión.');
      }

      // Crear archivo temporal y subir a Mojang
      const fileData = await file.arrayBuffer();
      const tempFilePath = await invoke<string>('create_temp_file', {
        fileName: `skin_${Date.now()}.png`,
        fileData: Array.from(new Uint8Array(fileData))
      });

      console.log('📤 Subiendo skin a Mojang API...');
      console.log('   Token:', accessToken.substring(0, 30) + '...');
      console.log('   Archivo:', tempFilePath);
      console.log('   Variant: classic');
      
      await invoke('upload_skin_to_mojang', {
        filePath: tempFilePath,
        variant: 'classic',
        accessToken
      });

      console.log('✅ Skin subida exitosamente a Mojang');

      // Obtener perfil de Mojang para obtener URL y textureId de la skin recién subida
      let skinUrl = '';
      let textureId = '';
      try {
        const profileResponse = await invoke<ProfileResponse>('get_minecraft_profile_safe', {
          accessToken
        });
        if (profileResponse.status === 'Ok' && profileResponse.profile) {
          const profile = profileResponse.profile as any;
          const mojangSkins = profile.skins || [];
          const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');
          if (activeMojangSkin) {
            skinUrl = activeMojangSkin.url || '';
            textureId = activeMojangSkin.id || '';
            console.log('✅ URL y textureId obtenidos de Mojang:', { skinUrl, textureId });
          }
        }
      } catch (err) {
        console.warn('⚠️ No se pudo obtener URL/textureId de Mojang, continuando sin ellos:', err);
      }

      // Crear blob URL INMEDIATAMENTE desde el fileData original
      const uint8Array = new Uint8Array(fileData);
      const buffer = new ArrayBuffer(uint8Array.length);
      const view = new Uint8Array(buffer);
      view.set(uint8Array);
      const blob = new Blob([buffer], { type: 'image/png' });
      const blobUrl = URL.createObjectURL(blob);
      
      // Guardar skin localmente con URL y textureId si están disponibles
      const newSkin: SkinData = {
        id: `uploaded_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        name: file.name.replace('.png', ''),
        fileData: buffer,
        url: skinUrl,
        textureId: textureId,
        variant: 'classic',
        uploadedAt: new Date(),
        isActive: true
      };

      await SkinStorageService.saveSkin(newSkin);
      await SkinStorageService.setActiveSkin(newSkin.id);

      // Guardar blob URL inmediatamente para esta skin
      blobUrlsRef.current.set(newSkin.id, blobUrl);

      const updatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...updatedSkins]);
      setSelectedSkinId(newSkin.id);

      // Refrescar avatares después de 2 segundos (dar tiempo a que Mojang procese)
      setTimeout(() => {
        refreshAvatars();
      }, 2000);

      addToast?.('✅ Skin subida y aplicada correctamente', 'success');
    } catch (error) {
      console.error('❌ Error al subir skin:', error);
      addToast?.(`Error: ${error instanceof Error ? error.message : 'Error desconocido'}`, 'error');
    } finally {
      setIsUploading(false);
    }
  }, [addToast, getValidMinecraftToken]);

  // Helper: Obtener skin completa con fileData desde storage
  const getSkinWithFileData = useCallback(async (skinId: string): Promise<SkinData | null> => {
    const allSkins = await SkinStorageService.getStoredSkins();
    return allSkins.find(s => s.id === skinId) || null;
  }, []);

  // Helper: Obtener fileData de una skin (desde archivo o descargando desde URL)
  const getSkinFileData = useCallback(async (skin: SkinData): Promise<ArrayBuffer> => {
    // Primero intentar cargar desde archivo
    try {
      const fileDataArray = await invoke<number[]>('load_skin_file', { skinId: skin.id });
      const uint8Array = new Uint8Array(fileDataArray);
      const buffer = new ArrayBuffer(uint8Array.length);
      const view = new Uint8Array(buffer);
      view.set(uint8Array);
      return buffer;
    } catch (err) {
      // Si no existe el archivo, continuar
    }

    // Si tiene fileData en el objeto actual, guardarlo y usarlo
    if (skin.fileData && skin.fileData instanceof ArrayBuffer && skin.fileData.byteLength > 0) {
      await SkinStorageService.saveSkin(skin);
      return skin.fileData;
    }

    // Si tiene URL, descargar y guardar
    if (skin.url && skin.url.trim() !== '') {
      const response = await fetch(skin.url);
      if (!response.ok) {
        throw new Error(`No se pudo descargar la skin desde la URL (${response.status})`);
      }
      const blob = await response.blob();
      const arrayBuffer = await blob.arrayBuffer();
      
      // Guardar el fileData descargado
      const updatedSkin = { ...skin, fileData: arrayBuffer };
      await SkinStorageService.saveSkin(updatedSkin);
      
      return arrayBuffer;
    }

    throw new Error(`La skin "${skin.name || skin.id}" no tiene datos disponibles. Por favor, vuelve a subirla.`);
  }, []);

  // Seleccionar y aplicar skin - SIEMPRE subir a Mojang
  const handleSelectSkin = useCallback(async (skin: SkinData) => {
    if (isUploading || selectedSkinId === skin.id) return;

    setIsUploading(true);
    addToast?.('Aplicando skin a Mojang...', 'info', 3000);

    try {
      // Obtener fileData de la skin
      const fileData = await getSkinFileData(skin);

      // Obtener token
      const accessToken = await getValidMinecraftToken();
      if (!accessToken) {
        throw new Error('No se pudo autenticar. Verifica tu sesión.');
      }

      // Crear archivo temporal
      const tempFilePath = await invoke<string>('create_temp_file', {
        fileName: `skin_${Date.now()}.png`,
        fileData: Array.from(new Uint8Array(fileData))
      });

      // Obtener variant de la skin guardada
      const currentStoredSkin = await getSkinWithFileData(skin.id);
      const variant = currentStoredSkin?.variant || skin.variant || 'classic';
      
      // Subir a Mojang
      try {
        await invoke('upload_skin_to_mojang', {
          filePath: tempFilePath,
          variant: variant,
          accessToken
        });
      } catch (uploadError: any) {
        // Manejar errores específicos de la API
        if (uploadError?.message?.includes('429') || uploadError?.message?.includes('rate limit')) {
          throw new Error('Rate limit de Mojang API. Espera unos segundos e inténtalo de nuevo.');
        }
        if (uploadError?.message?.includes('401') || uploadError?.message?.includes('Unauthorized')) {
          throw new Error('Sesión expirada. Por favor, reinicia sesión.');
        }
        throw new Error(`Error al subir skin: ${uploadError?.message || 'Error desconocido'}`);
      }

      // Obtener perfil de Mojang para obtener URL y textureId actualizados
      let updatedSkinUrl = skin.url || '';
      let updatedTextureId = skin.textureId || '';
      try {
        const profileResponse = await invoke<ProfileResponse>('get_minecraft_profile_safe', {
          accessToken
        });
        if (profileResponse.status === 'Ok' && profileResponse.profile) {
          const profile = profileResponse.profile as any;
          const mojangSkins = profile.skins || [];
          const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');
          if (activeMojangSkin) {
            updatedSkinUrl = activeMojangSkin.url || '';
            updatedTextureId = activeMojangSkin.id || '';
            console.log('✅ URL y textureId actualizados de Mojang:', { updatedSkinUrl, updatedTextureId });
          }
        }
      } catch (err) {
        console.warn('⚠️ No se pudo obtener URL/textureId de Mojang, usando valores existentes:', err);
      }

      // Guardar el fileData en disco si no está guardado, y actualizar URL/textureId
      const storedSkinForSelect = await getSkinWithFileData(skin.id);
      const updatedSkin: SkinData = {
        ...(storedSkinForSelect || skin),
        fileData: fileData,
        url: updatedSkinUrl || skin.url || '',
        textureId: updatedTextureId || skin.textureId || ''
      };
      await SkinStorageService.saveSkin(updatedSkin);

      // Actualizar skin activa
      await SkinStorageService.setActiveSkin(skin.id);
      setSelectedSkinId(skin.id);

      // Recargar skins
      const allUpdatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...allUpdatedSkins]);

      // Refrescar avatares después de 2 segundos
      setTimeout(() => {
        refreshAvatars();
      }, 2000);

      addToast?.('✅ Skin aplicada correctamente', 'success');
    } catch (error) {
      console.error('❌ Error al aplicar skin:', error);
      const errorMessage = error instanceof Error ? error.message : 'Error desconocido';
      addToast?.(`❌ ${errorMessage}`, 'error');
    } finally {
      setIsUploading(false);
    }
  }, [isUploading, selectedSkinId, addToast, getValidMinecraftToken, getSkinFileData, getSkinWithFileData]);

  // Eliminar skin
  const handleDeleteSkin = useCallback(async (skinId: string, event: React.MouseEvent) => {
    event.stopPropagation();

    if (selectedSkinId === skinId) {
      addToast?.('No puedes eliminar la skin seleccionada', 'error');
      return;
    }

    try {
      // Limpiar blob URL de la skin eliminada ANTES de eliminarla
      if (blobUrlsRef.current.has(skinId)) {
        URL.revokeObjectURL(blobUrlsRef.current.get(skinId)!);
        blobUrlsRef.current.delete(skinId);
      }
      
      await SkinStorageService.deleteSkin(skinId);
      const updatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...updatedSkins]);
      addToast?.('Skin eliminada', 'success');
    } catch (error) {
      console.error('❌ Error al eliminar skin:', error);
      addToast?.('Error al eliminar skin', 'error');
    }
  }, [selectedSkinId, addToast]);

  // Cambiar modelo (slim/classic) - SELECCIONA AUTOMÁTICAMENTE
  const handleToggleModel = useCallback(async (skin: SkinData, event: React.MouseEvent) => {
    event.stopPropagation();

    if (isUploading) return;

    const newVariant: SkinModel = skin.variant === 'classic' ? 'slim' : 'classic';
    
    setIsUploading(true);
    addToast?.('Cambiando formato a Mojang...', 'info', 3000);

    try {
      // Obtener fileData de la skin
      const fileData = await getSkinFileData(skin);

      // Obtener token
      const accessToken = await getValidMinecraftToken();
      if (!accessToken) {
        throw new Error('No se pudo autenticar. Verifica tu sesión.');
      }

      // Crear archivo temporal
      const tempFilePath = await invoke<string>('create_temp_file', {
        fileName: `skin_${Date.now()}.png`,
        fileData: Array.from(new Uint8Array(fileData))
      });
      
      // Subir a Mojang con el nuevo variant
      try {
        await invoke('upload_skin_to_mojang', {
          filePath: tempFilePath,
          variant: newVariant,
          accessToken
        });
      } catch (uploadError: any) {
        // Manejar errores específicos de la API
        if (uploadError?.message?.includes('429') || uploadError?.message?.includes('rate limit')) {
          throw new Error('Rate limit de Mojang API. Espera unos segundos e inténtalo de nuevo.');
        }
        if (uploadError?.message?.includes('401') || uploadError?.message?.includes('Unauthorized')) {
          throw new Error('Sesión expirada. Por favor, reinicia sesión.');
        }
        throw new Error(`Error al cambiar formato: ${uploadError?.message || 'Error desconocido'}`);
      }

      // Obtener perfil de Mojang para obtener URL y textureId actualizados
      let updatedSkinUrl = skin.url || '';
      let updatedTextureId = skin.textureId || '';
      try {
        const profileResponse = await invoke<ProfileResponse>('get_minecraft_profile_safe', {
          accessToken
        });
        if (profileResponse.status === 'Ok' && profileResponse.profile) {
          const profile = profileResponse.profile as any;
          const mojangSkins = profile.skins || [];
          const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');
          if (activeMojangSkin) {
            updatedSkinUrl = activeMojangSkin.url || '';
            updatedTextureId = activeMojangSkin.id || '';
            console.log('✅ URL y textureId actualizados de Mojang después de cambiar formato:', { updatedSkinUrl, updatedTextureId });
          }
        }
      } catch (err) {
        console.warn('⚠️ No se pudo obtener URL/textureId de Mojang, usando valores existentes:', err);
      }

      // Actualizar skin con el nuevo variant - PRESERVAR fileData y actualizar URL/textureId
      const storedSkin = await getSkinWithFileData(skin.id);
      const updatedSkin: SkinData = { 
        ...(storedSkin || skin),
        variant: newVariant,
        fileData: fileData,
        url: updatedSkinUrl || skin.url || '',
        textureId: updatedTextureId || skin.textureId || ''
      };
      await SkinStorageService.saveSkin(updatedSkin);

      // Recargar skins
      const allUpdatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...allUpdatedSkins]);

      // SELECCIONAR AUTOMÁTICAMENTE esta skin
      await SkinStorageService.setActiveSkin(skin.id);
      setSelectedSkinId(skin.id);

      // Refrescar avatares después de 2 segundos
      setTimeout(() => {
        refreshAvatars();
      }, 2000);

      addToast?.('✅ Formato actualizado y skin seleccionada', 'success');
    } catch (error) {
      console.error('❌ Error al cambiar formato:', error);
      const errorMessage = error instanceof Error ? error.message : 'Error desconocido';
      addToast?.(`❌ ${errorMessage}`, 'error');
    } finally {
      setIsUploading(false);
    }
  }, [isUploading, addToast, getValidMinecraftToken, getSkinFileData, getSkinWithFileData]);

  // Dropzone para drag & drop
  const onDrop = useCallback((acceptedFiles: File[]) => {
    if (acceptedFiles.length > 0) {
      handleUploadSkin(acceptedFiles[0]);
    }
  }, [handleUploadSkin]);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: { 'image/png': ['.png'] },
    multiple: false,
    disabled: isUploading,
    noClick: true,
    noKeyboard: false
  });

  return (
    <div className="h-full w-full flex flex-col bg-gradient-to-br from-black via-[#0a0a0a] to-black relative overflow-hidden">
      {/* Overlay oscuro */}
      <div className="absolute inset-0 bg-black/30" />

      {/* Contenido principal */}
      <div className="relative z-10 h-full flex flex-col px-8 py-2">
        {/* Título - muy pequeño */}
        <div className="flex-shrink-0 mb-3">
          <h1 className="text-2xl font-black text-white text-center tracking-wide drop-shadow-lg">
            Gestión de Skins
          </h1>
        </div>

        {/* Grid de skins - más padding para que no se corte */}
        <div
          {...getRootProps()}
          className="flex-1 overflow-y-auto px-2 custom-scrollbar"
        >
          <input {...getInputProps()} />

          {/* Overlay de drag activo */}
          {isDragActive && (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
              <div className="glass-card px-12 py-8 rounded-2xl border-2 border-dashed border-[#00ffff] bg-[#00ffff]/10">
                <svg className="mx-auto h-16 w-16 text-[#00ffff] mb-4" stroke="currentColor" fill="none" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
                <p className="text-2xl font-bold text-white">Suelta el archivo aquí</p>
              </div>
            </div>
          )}

          {isLoadingInitial ? (
            <div className="h-full flex items-center justify-center">
              <div className="animate-spin rounded-full h-16 w-16 border-t-4 border-b-4 border-[#00ffff]" />
            </div>
          ) : (
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6 py-6">
              {/* Card para añadir skin */}
              <div
                onClick={() => {
                  if (!isUploading) {
                    const input = document.createElement('input');
                    input.type = 'file';
                    input.accept = 'image/png';
                    input.onchange = (e) => {
                      const file = (e.target as HTMLInputElement).files?.[0];
                      if (file) handleUploadSkin(file);
                    };
                    input.click();
                  }
                }}
                className={`group transition-all duration-300 ease-out ${
                  isUploading 
                    ? 'opacity-50 cursor-not-allowed scale-100' 
                    : 'cursor-pointer hover:scale-105'
                }`}
                style={{ aspectRatio: '3/4' }}
              >
                <div className="w-full h-full rounded-2xl overflow-hidden ring-1 ring-white/10 hover:ring-white/20 transition-all duration-300 bg-black/40 backdrop-blur-sm flex flex-col items-center justify-center gap-3 border-2 border-dashed border-white/20 hover:border-[#00ffff]/50">
                  <div className="w-14 h-14 rounded-full bg-[#00ffff]/10 flex items-center justify-center group-hover:bg-[#00ffff]/20 transition-colors">
                    <svg className="w-7 h-7 text-[#00ffff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                    </svg>
                  </div>
                  <div className="text-center px-4">
                    <p className="text-white text-sm font-medium">Añadir Skin</p>
                    <p className="text-white/50 text-xs mt-1">PNG · 64x64 · &lt;24KB</p>
                  </div>
                </div>
              </div>

              {/* Skins guardadas */}
              {skins.map((skin) => {
                const isSelected = selectedSkinId === skin.id;
                // getSkinUrl ya está memoizado con useCallback y reutiliza blob URLs
                const skinUrl = getSkinUrl(skin);

                return (
                  <div
                    key={skin.id}
                    className={`group relative transition-all duration-300 ease-out ${
                      isSelected ? 'scale-105' : 'hover:scale-105'
                    } ${isUploading ? 'pointer-events-none opacity-50' : ''}`}
                    style={{ aspectRatio: '3/4' }}
                  >
                    {/* Contenedor principal con ring */}
                    <div 
                      onClick={() => handleSelectSkin(skin)}
                      className={`w-full h-full rounded-2xl overflow-hidden transition-all duration-300 ease-out relative cursor-pointer flex flex-col ${
                        isSelected
                          ? 'ring-2 ring-[#00ffff]'
                          : 'ring-1 ring-white/10 hover:ring-white/20'
                      }`}
                      style={isSelected ? {
                        boxShadow: '0 0 0 2px rgba(0, 0, 0, 0.5), 0 0 20px rgba(0, 255, 255, 0.6), 0 0 40px rgba(0, 255, 255, 0.4)'
                      } : {}}
                    >
                        {/* Preview 3D - ocupa casi todo menos el footer */}
                        <div className="h-[calc(100%-44px)]">
                          <SkinPreview3D 
                            key={`preview-${skin.id}-${skin.variant}`} 
                            skinUrl={skin.fileData ? undefined : skinUrl}
                            skinFileData={skin.fileData && skin.fileData instanceof ArrayBuffer && skin.fileData.byteLength > 0 ? skin.fileData : undefined}
                            className="w-full h-full" 
                          />
                        
                        {/* Botón eliminar - solo visible en hover, top left */}
                        {!isSelected && (
                          <button
                            onClick={(e) => handleDeleteSkin(skin.id, e)}
                            className="absolute top-2 left-2 z-20 w-8 h-8 rounded-full bg-red-500/90 hover:bg-red-600 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity shadow-lg"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                            </svg>
                          </button>
                        )}

                        {/* Botón Seleccionar - visible en hover, center */}
                        {!isSelected && (
                          <button
                            onClick={() => handleSelectSkin(skin)}
                            className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-20 px-6 py-2 rounded-lg bg-[#00ffff]/90 hover:bg-[#00ffff] text-black font-medium text-sm shadow-lg opacity-0 group-hover:opacity-100 transition-all duration-200 hover:scale-105"
                          >
                            Seleccionar
                          </button>
                        )}
                      </div>

                      {/* Footer con Toggle Slim/Classic - SIEMPRE VISIBLE */}
                      <div className="h-[44px] flex items-center justify-center p-2 bg-black/90 border-t border-white/10">
                        <div className="flex items-center gap-2">
                          <span className={`text-xs font-medium transition-colors ${skin.variant === 'slim' ? 'text-[#00ffff]' : 'text-white/50'}`}>
                            Slim
                          </span>
                          <button
                            onClick={(e) => handleToggleModel(skin, e)}
                            disabled={isUploading}
                            className={`relative w-11 h-6 rounded-full transition-all ${
                              skin.variant === 'classic' ? 'bg-[#00ffff]' : 'bg-white/20'
                            } ${isUploading ? 'opacity-50 cursor-not-allowed' : 'hover:scale-105'}`}
                          >
                            <div
                              className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-md transition-transform ${
                                skin.variant === 'classic' ? 'translate-x-5' : 'translate-x-0'
                              }`}
                            />
                          </button>
                          <span className={`text-xs font-medium transition-colors ${skin.variant === 'classic' ? 'text-[#00ffff]' : 'text-white/50'}`}>
                            Classic
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

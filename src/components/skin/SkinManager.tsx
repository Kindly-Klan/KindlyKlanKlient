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

  // Rate limiting para evitar problemas con la API de Mojang (600 requests por 10 minutos)
  const lastUploadTimeRef = useRef<number>(0);
  const MIN_UPLOAD_INTERVAL = 2000; // Mínimo 2 segundos entre subidas

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

  useEffect(() => {
    if (hasInitialized.current) return;
    hasInitialized.current = true;
    
    const initializeSkins = async () => {
      try {
        const savedSkins = await SkinStorageService.getStoredSkins();
        const activeSkin = await SkinStorageService.getActiveSkin();
        
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
        
        // Establecer estado inicial
        setSkins([...savedSkins]);
        setSelectedSkinId(activeSkin?.id || null);
        setIsLoadingInitial(false);

        // Ya no sincronizamos automáticamente con Mojang
        // Las skins locales tienen prioridad y se muestran inmediatamente

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
  }, [addToast]);

  // Ya no sincronizamos automáticamente al cambiar de pestaña
  // Las skins locales se muestran siempre sin depender de Mojang

  // Guardar nueva skin localmente (sin subir a Mojang)
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
    addToast?.('Guardando skin...', 'info', 2000);

    try {
      // Leer archivo y guardar localmente
      const fileData = await file.arrayBuffer();
      const uint8Array = new Uint8Array(fileData);
      const buffer = new ArrayBuffer(uint8Array.length);
      const view = new Uint8Array(buffer);
      view.set(uint8Array);
      const blob = new Blob([buffer], { type: 'image/png' });
      const blobUrl = URL.createObjectURL(blob);
      
      // Crear skin local
      const newSkin: SkinData = {
        id: `skin_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        name: file.name.replace('.png', ''),
        fileData: buffer,
        url: '',
        textureId: '',
        variant: 'classic',
        uploadedAt: new Date(),
        isActive: false, // No activar automáticamente
        isMojangSynced: false
      };

      // Guardar skin localmente
      await SkinStorageService.saveSkin(newSkin);

      // Guardar blob URL
      blobUrlsRef.current.set(newSkin.id, blobUrl);

      // Actualizar lista de skins
      const updatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...updatedSkins]);

      addToast?.('✅ Skin guardada correctamente', 'success');
    } catch (error) {
      console.error('❌ Error al guardar skin:', error);
      addToast?.(`Error: ${error instanceof Error ? error.message : 'Error desconocido'}`, 'error');
    } finally {
      setIsUploading(false);
    }
  }, [addToast]);

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

  // Seleccionar skin: primero activar localmente, luego intentar subir a Mojang (opcional)
  const handleSelectSkin = useCallback(async (skin: SkinData) => {
    if (isUploading || selectedSkinId === skin.id) return;

    // 1. Activar skin localmente INMEDIATAMENTE (sin esperar a Mojang)
    await SkinStorageService.setActiveSkin(skin.id);
    setSelectedSkinId(skin.id);
    
    // Recargar skins para reflejar el cambio
    const allSkins = await SkinStorageService.getStoredSkins();
    setSkins([...allSkins]);

    // 2. Intentar subir a Mojang en segundo plano (opcional, no bloquea)
    setIsUploading(true);
    
    // Rate limiting: esperar si la última subida fue hace menos de MIN_UPLOAD_INTERVAL
    const timeSinceLastUpload = Date.now() - lastUploadTimeRef.current;
    if (timeSinceLastUpload < MIN_UPLOAD_INTERVAL) {
      await new Promise(resolve => setTimeout(resolve, MIN_UPLOAD_INTERVAL - timeSinceLastUpload));
    }

    try {
      // Obtener fileData de la skin
      const fileData = await getSkinFileData(skin);

      // Obtener token (si no hay token, simplemente no subimos a Mojang)
      const accessToken = await getValidMinecraftToken();
      if (!accessToken) {
        console.log('⚠️ No hay token válido, skin activada solo localmente');
        setIsUploading(false);
        return;
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
      lastUploadTimeRef.current = Date.now();
      try {
        await invoke('upload_skin_to_mojang', {
          filePath: tempFilePath,
          variant: variant,
          accessToken
        });

        // Obtener perfil de Mojang para actualizar URL y textureId
        try {
          const profileResponse = await invoke<ProfileResponse>('get_minecraft_profile_safe', {
            accessToken
          });
          if (profileResponse.status === 'Ok' && profileResponse.profile) {
            const profile = profileResponse.profile as any;
            const mojangSkins = profile.skins || [];
            const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');
            if (activeMojangSkin) {
              const updatedSkin: SkinData = {
                ...(currentStoredSkin || skin),
                fileData: fileData,
                url: activeMojangSkin.url || skin.url || '',
                textureId: activeMojangSkin.id || skin.textureId || '',
                isMojangSynced: true
              };
              await SkinStorageService.saveSkin(updatedSkin);
              
              // Recargar skins
              const updatedSkins = await SkinStorageService.getStoredSkins();
              setSkins([...updatedSkins]);
            }
          }
        } catch (err) {
          // Ignorar errores al obtener perfil
        }

        addToast?.('✅ Skin aplicada y sincronizada con Mojang', 'success');
      } catch (uploadError: any) {
        // Manejar errores específicos de la API
        if (uploadError?.message?.includes('429') || uploadError?.message?.includes('rate limit')) {
          addToast?.('⚠️ Rate limit de Mojang. La skin está activa localmente.', 'info');
        } else if (uploadError?.message?.includes('401') || uploadError?.message?.includes('Unauthorized')) {
          addToast?.('⚠️ Sesión expirada. La skin está activa localmente.', 'info');
        } else {
          // Otros errores: la skin sigue activa localmente
          console.warn('⚠️ Error al subir a Mojang (skin activa localmente):', uploadError);
        }
      }
    } catch (error) {
      // Error al obtener fileData o token: la skin ya está activa localmente
      console.warn('⚠️ Error al procesar skin para Mojang (skin activa localmente):', error);
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

  // Cambiar modelo (slim/classic) - actualizar localmente primero, luego Mojang
  const handleToggleModel = useCallback(async (skin: SkinData, event: React.MouseEvent) => {
    event.stopPropagation();

    if (isUploading) return;

    const newVariant: SkinModel = skin.variant === 'classic' ? 'slim' : 'classic';
    
    setIsUploading(true);

    try {
      // 1. Actualizar variant localmente INMEDIATAMENTE
      const storedSkin = await getSkinWithFileData(skin.id);
      const updatedSkin: SkinData = { 
        ...(storedSkin || skin),
        variant: newVariant
      };
      await SkinStorageService.saveSkin(updatedSkin);

      // Recargar skins
      const allUpdatedSkins = await SkinStorageService.getStoredSkins();
      setSkins([...allUpdatedSkins]);

      // 2. Intentar subir a Mojang en segundo plano (opcional)
      // Rate limiting
      const timeSinceLastUpload = Date.now() - lastUploadTimeRef.current;
      if (timeSinceLastUpload < MIN_UPLOAD_INTERVAL) {
        await new Promise(resolve => setTimeout(resolve, MIN_UPLOAD_INTERVAL - timeSinceLastUpload));
      }

      const fileData = await getSkinFileData(updatedSkin);
      const accessToken = await getValidMinecraftToken();
      
      if (accessToken) {
        try {
          const tempFilePath = await invoke<string>('create_temp_file', {
            fileName: `skin_${Date.now()}.png`,
            fileData: Array.from(new Uint8Array(fileData))
          });
          
          lastUploadTimeRef.current = Date.now();
          await invoke('upload_skin_to_mojang', {
            filePath: tempFilePath,
            variant: newVariant,
            accessToken
          });

          // Actualizar URL/textureId si es posible
          try {
            const profileResponse = await invoke<ProfileResponse>('get_minecraft_profile_safe', {
              accessToken
            });
            if (profileResponse.status === 'Ok' && profileResponse.profile) {
              const profile = profileResponse.profile as any;
              const mojangSkins = profile.skins || [];
              const activeMojangSkin = mojangSkins.find((s: MojangSkin) => s.state === 'ACTIVE');
              if (activeMojangSkin) {
                const finalSkin: SkinData = {
                  ...updatedSkin,
                  url: activeMojangSkin.url || updatedSkin.url || '',
                  textureId: activeMojangSkin.id || updatedSkin.textureId || '',
                  isMojangSynced: true
                };
                await SkinStorageService.saveSkin(finalSkin);
                const finalSkins = await SkinStorageService.getStoredSkins();
                setSkins([...finalSkins]);
              }
            }
          } catch (err) {
            // Ignorar errores al obtener perfil
          }

          addToast?.('✅ Formato actualizado', 'success');
        } catch (uploadError: any) {
          if (uploadError?.message?.includes('429') || uploadError?.message?.includes('rate limit')) {
            addToast?.('⚠️ Rate limit. Formato actualizado localmente.', 'info');
          } else if (uploadError?.message?.includes('401') || uploadError?.message?.includes('Unauthorized')) {
            addToast?.('⚠️ Sesión expirada. Formato actualizado localmente.', 'info');
          } else {
            addToast?.('✅ Formato actualizado localmente', 'success');
          }
        }
      } else {
        addToast?.('✅ Formato actualizado localmente', 'success');
      }
    } catch (error) {
      console.error('❌ Error al cambiar formato:', error);
      addToast?.('Error al cambiar formato', 'error');
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

      <div className="relative z-10 h-full flex flex-col px-8 py-2">
        <div className="flex-shrink-0 mb-3">
          <h1 className="text-4xl font-black text-white text-center tracking-wide drop-shadow-lg">
            Gestión de Skins
          </h1>
        </div>

        {/* Grid de skins */}
        <div
          {...getRootProps()}
          className="flex-1 pb-10 px-2 custom-scrollbar"
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

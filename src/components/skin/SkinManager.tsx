import React, { useState, useEffect } from 'react';
import { SkinUploader } from './SkinUploader';
import { SkinPreview3D } from './SkinPreview3D';
import { SkinData, SkinModel } from '@/types/skin';
import { SkinStorageService } from '@/services/skin/skinStorage';
import { invoke } from '@tauri-apps/api/core';

interface SkinManagerProps {
  currentUser: any;   
  onClose: () => void;
}

export const SkinManager: React.FC<SkinManagerProps> = ({
  currentUser,
  onClose: _onClose
}) => {
  const [storedSkins, setStoredSkins] = useState<SkinData[]>([]);
  const [currentSkin, setCurrentSkin] = useState<SkinData | null>(null);
  const [skinModel, setSkinModel] = useState<SkinModel>('classic');
  const [isUploading, setIsUploading] = useState(false);
  const [uuid, setUuid] = useState<string | null>(null);
  const [currentTextureUrl, setCurrentTextureUrl] = useState<string>('');
  const [isVisible, setIsVisible] = useState(false);
  const [uploadTimeout, setUploadTimeout] = useState<NodeJS.Timeout | null>(null);

  
  useEffect(() => {   
    const loadCurrentSkin = async () => {
      const allSkins = await SkinStorageService.getStoredSkins();
      setStoredSkins(allSkins);
      const activeSkin = await SkinStorageService.getActiveSkin();
      if (activeSkin) {
        setCurrentSkin(activeSkin);
        setSkinModel(activeSkin.variant);
      }
      
      if (currentUser) {
        setUuid(currentUser.uuid);
      }
    };
    loadCurrentSkin();

    
    const timer = setTimeout(() => setIsVisible(true), 50);
    return () => clearTimeout(timer);
  }, []);

  
  useEffect(() => {
    if (!uuid) {
      setCurrentTextureUrl('');
      return;
    }

    const loadTexture = async () => {
      try {
        
        const savedSession = localStorage.getItem('kkk_session');
        if (!savedSession) {
          throw new Error('No session found');
        }

        const session = JSON.parse(savedSession);
        const accessToken = session?.access_token;

        if (!accessToken) {
          throw new Error('No access token available');
        }

        const profileData = await invoke<string>('get_minecraft_profile', { accessToken });
        const profile = JSON.parse(profileData);

        
        if (profile.skins && profile.skins.length > 0) {
          const skin = profile.skins[0];
          const textureUrl = skin.url;
          setCurrentTextureUrl(textureUrl);
          console.log('Loaded skin texture from Mojang API:', textureUrl);
        } else {
          console.log('No skin found in profile');
          setCurrentTextureUrl('');
        }
      } catch (error) {
        console.error('Error loading user texture from Mojang API:', error);
        
        try {
          const textureUrl = `https://crafatar.com/skins/${uuid}`;
          setCurrentTextureUrl(textureUrl);
          console.log('Fallback to Crafatar:', textureUrl);
        } catch (fallbackError) {
          console.error('Error with fallback:', fallbackError);
          setCurrentTextureUrl('');
        }
      }
    };

    loadTexture();
  }, [uuid]); 

  
  useEffect(() => {
    return () => {
      if (uploadTimeout) {
        clearTimeout(uploadTimeout);
      }
    };
  }, [uploadTimeout]);

  const handleSkinUpload = async (skinData: SkinData) => {
    setIsUploading(true);
    try {
      
      await SkinStorageService.saveSkin(skinData);

      
      await SkinStorageService.setActiveSkin(skinData.id);


      setCurrentSkin({ ...skinData, isActive: true });
      setSkinModel(skinData.variant);

      
      const refreshed = await SkinStorageService.getStoredSkins();
      setStoredSkins(refreshed);

      
      setTimeout(() => {
        if (currentSkin?.id === skinData.id) {
          setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
        }
      }, 2000);
      setTimeout(() => {
        if (currentSkin?.id === skinData.id) {
          setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
        }
      }, 5000);
      setTimeout(() => {
        if (currentSkin?.id === skinData.id) {
          setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
        }
      }, 10000);

      console.log('Skin subida y aplicada exitosamente');
    } catch (error) {
      console.error('Error manejando subida de skin:', error);
    } finally {
      setIsUploading(false);
    }
  };

  const handleSkinDelete = async (skinId: string) => {
    try {
      await SkinStorageService.deleteSkin(skinId);
      const refreshed = await SkinStorageService.getStoredSkins();
      setStoredSkins(refreshed);

      if (currentSkin?.id === skinId) {
        setCurrentSkin(null);
        setSkinModel('classic');
      }

      console.log('Skin eliminada exitosamente');
    } catch (error) {
      console.error('Error eliminando skin:', error);
    }
  };

  const handleSelectSkin = async (skin: SkinData) => {
    
    if (uploadTimeout) {
      clearTimeout(uploadTimeout);
    }

    
    await SkinStorageService.setActiveSkin(skin.id);
    setCurrentSkin(skin);
    setSkinModel(skin.variant);

    
    if (!skin.fileData || skin.fileData.byteLength === 0) {
      console.warn('La skin seleccionada no tiene datos binarios almacenados (fileData). Re-súbela para poder activarla.');
      return;
    }

    
    const timeout = setTimeout(async () => {
      try {
        setIsUploading(true);
        const savedSession = localStorage.getItem('kkk_session');
        const session = savedSession ? JSON.parse(savedSession) : null;
        const accessToken = session?.access_token;

        if (!accessToken) {
          console.warn('No hay token de acceso disponible');
          return;
        }

        
        if (!skin.fileData || skin.fileData.byteLength === 0) {
          console.warn('Los datos de la skin se perdieron durante el debounce');
          return;
        }

        const tempFilePath = await invoke<string>('create_temp_file', {
          fileName: skin.name || 'skin.png',
          fileData: skin.fileData
        });

        await invoke('upload_skin_to_mojang', {
          filePath: tempFilePath,
          variant: skin.variant === 'slim' ? 'slim' : 'classic',
          accessToken
        });

        
        setTimeout(() => {
          if (currentSkin?.id === skin.id) {
            setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
          }
        }, 2000);
        setTimeout(() => {
          if (currentSkin?.id === skin.id) {
            setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
          }
        }, 5000);
        setTimeout(() => {
          if (currentSkin?.id === skin.id) {
            setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
          }
        }, 10000);

        console.log('Skin subida exitosamente a Mojang');
      } catch (error) {
        console.error('Error subiendo skin a Mojang:', error);
      } finally {
        setIsUploading(false);
      }
    }, 1000);

    setUploadTimeout(timeout);
  };

  const handleModelSwitch = async (newModel: SkinModel) => {
    if (!currentSkin) return;
    
    try {
      setSkinModel(newModel);
      const savedSession = localStorage.getItem('kkk_session');
      const session = savedSession ? JSON.parse(savedSession) : null;
      const accessToken = session?.access_token;
      
      if (accessToken) {
        let arrayBufferToUse: ArrayBuffer | null = currentSkin.fileData || null;

        
        if (!arrayBufferToUse && uuid) {
          try {
            const resp = await fetch(`https://crafatar.com/skins/${uuid}`);
            if (resp.ok) {
              arrayBufferToUse = await resp.arrayBuffer();
            }
          } catch (e) {
            console.error('No se pudo descargar la skin actual para cambiar variante:', e);
          }
        }

        if (arrayBufferToUse) {
          
          const tempFilePath = await invoke<string>('create_temp_file', {
            fileName: currentSkin.name || 'skin.png',
            fileData: arrayBufferToUse
          });
          
          
          await invoke('set_skin_variant', { 
            filePath: tempFilePath,
            variant: newModel === 'slim' ? 'slim' : 'classic', 
            accessToken 
          });


          setTimeout(() => {
            if (currentSkin?.id === updatedSkin.id) {
              setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
            }
          }, 2000);
          setTimeout(() => {
            if (currentSkin?.id === updatedSkin.id) {
              setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
            }
          }, 5000);
          setTimeout(() => {
            if (currentSkin?.id === updatedSkin.id) {
              setCurrentTextureUrl(prev => `${prev}?t=${Date.now()}`);
            }
          }, 10000);
        } else {
          console.warn('No hay datos de skin disponibles para re-subir con la nueva variante');
        }
      }
      
      
      const updatedSkin = { ...currentSkin, variant: newModel };
      await SkinStorageService.saveSkin(updatedSkin);
      setCurrentSkin(updatedSkin);
      
    } catch (error) {
      console.error('Error cambiando modelo de skin:', error);
    }
  };

  return (
    <div className={`h-full bg-gradient-to-br from-gray-900 via-slate-900 to-black flex transition-all duration-500 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
      
      <div className={`absolute top-16 left-1/2 transform -translate-x-1/2 text-white/90 transition-all duration-700 ${isVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-75'}`}>
        <div className="flex items-center justify-center gap-4">
          <h1 className="text-6xl font-black tracking-wide text-center text-white drop-shadow-lg">
            Skins
          </h1>
          <div className="bg-gradient-to-r from-orange-500 to-red-500 text-white px-3 py-1 rounded-full text-sm font-bold shadow-lg border border-orange-400/50 translate-y-2">
            BETA
          </div>
        </div>
      </div>

      <div className={`flex-1 p-6 flex items-center justify-center gap-6 transition-all duration-700 delay-300 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}`}>
        <div className="flex-shrink-0">
          <SkinUploader
            onUploadSuccess={handleSkinUpload}
            onUploadError={(error) => console.error('Error subiendo skin:', error)}
            disabled={isUploading}
          />
        </div>

      
      <div className="flex-1 grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6 max-w-4xl p-1 transition-all">
        {storedSkins.map((skin) => (
          <div key={skin.id} className="relative group flex items-center justify-center transition-all duration-300">
            <button
              className={`relative w-48 h-64 rounded-lg border-2 overflow-hidden bg-gray-800/40 transition-all duration-300 hover:scale-[1.02] ${
                currentSkin?.id === skin.id
                  ? 'border-blue-500 shadow-[0_0_0_3px_rgba(59,130,246,0.3)]'
                  : 'border-gray-600 hover:border-gray-500'
              }`}
              onClick={() => handleSelectSkin(skin)}
            >
              <SkinPreview3D
                skinUrl={currentSkin?.id === skin.id ? currentTextureUrl : `https://crafatar.com/skins/${uuid}?t=${skin.uploadedAt?.getTime() || Date.now()}`}
                className="w-48 h-64"
                key={`${skin.id}-${skin.uploadedAt?.getTime() || Date.now()}`}
                onTextureLoad={(textureUrl) => {
                  
                  if (currentSkin?.id === skin.id) {
                    setCurrentTextureUrl(textureUrl);
                  }
                }}
              />

              
              {currentSkin?.id === skin.id && (
                <div className="absolute -top-2 -right-2 w-6 h-6 rounded-full bg-green-500 text-white flex items-center justify-center shadow-lg border border-white/20 z-20">
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M20 6L9 17l-5-5" />
                  </svg>
                </div>
              )}

              
              {currentSkin?.id === skin.id && (
                <div className="absolute bottom-2 left-1/2 -translate-x-1/2 flex items-center space-x-2 bg-black/80 backdrop-blur-sm rounded-lg px-3 py-2 border border-white/20 opacity-0 group-hover:opacity-100 transition-opacity">
                  <span className="text-xs text-gray-300">Slim</span>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      className="sr-only peer"
                      checked={skinModel !== 'slim'}
                      onChange={(e) => handleModelSwitch(e.target.checked ? 'classic' : 'slim')}
                    />
                    <div className="w-10 h-5 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:bg-blue-600 transition-colors"></div>
                    <div className="absolute left-0.5 top-0.5 w-4 h-4 bg-white rounded-full transition-transform peer-checked:translate-x-5"></div>
                  </label>
                  <span className="text-xs text-gray-300">Normal</span>
                </div>
              )}

              
              <button
                type="button"
                onClick={(e) => { e.stopPropagation(); handleSkinDelete(skin.id); }}
                disabled={currentSkin?.id === skin.id}
                className={`absolute top-2 left-2 w-8 h-8 rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 shadow-lg z-20 ${currentSkin?.id === skin.id ? 'bg-red-600/40 text-white/60 cursor-not-allowed' : 'bg-red-600 text-white cursor-pointer hover:bg-red-700'}`}
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </button>
          </div>
        ))}

          
        {storedSkins.length === 0 && (
          <div className="w-48 h-64 bg-gray-800/60 rounded-lg border-2 border-dashed border-gray-600 flex items-center justify-center text-gray-400">
            Sin skins guardadas
          </div>
        )}
      </div>
      </div>

    </div>
  );
};

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import LaunchButton from './LaunchButton';


import minecraftIcon from '@/assets/icons/minecraft.svg';
import fabricmcIcon from '@/assets/icons/fabricmc.svg';
import neoforgeIcon from '@/assets/icons/neoforge.svg';

interface DistributionManifest {
  distribution: {
    name: string;
    version: string;
    description: string;
    base_url: string;
    last_updated: string;
  };
  instances: Array<{
    id: string;
    name: string;
    description: string;
    version: string;
    minecraft_version: string;
    icon?: string;
    background?: string;
    background_video?: string;
    last_updated?: string;
    instance_url: string;
    mod_loader?: {
      type: string;
      version: string;
    };
  }>;
}

interface InstanceViewProps {
  instanceId: string;
  distribution: DistributionManifest;
  distributionBaseUrl: string;
  onLaunch: (instance: any) => Promise<void>;
  isJavaInstalling?: boolean;
}

// Caché global para videos por instancia
const videoCache = new Map<string, { blobUrl: string; loaded: boolean }>();

const InstanceView: React.FC<InstanceViewProps> = ({
  instanceId,
  distribution,
  distributionBaseUrl,
  onLaunch,
  isJavaInstalling = false
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [localVideoPath, setLocalVideoPath] = useState<string | null>(null);
  const [videoLoaded, setVideoLoaded] = useState(false);
  const [showTitle, setShowTitle] = useState(true);
  const instance = distribution.instances.find(inst => inst.id === instanceId);

  // Animate on instance change
  useEffect(() => {
    if (instanceId) {
      setIsVisible(false);
      const timer = setTimeout(() => setIsVisible(true), 50);
      return () => clearTimeout(timer);
    }
  }, [instanceId]);

  // Descargar video cuando hay background_video disponible
  useEffect(() => {
    const cacheKey = `${instanceId}-${instance?.background_video}`;
    const cached = videoCache.get(cacheKey);
    
    // Si tenemos el video en caché, usarlo directamente
    if (cached) {
      setLocalVideoPath(cached.blobUrl);
      setVideoLoaded(cached.loaded);
      setShowTitle(!cached.loaded); // Solo mostrar título si no estaba cargado
      return;
    }
    
    // Si no hay caché, resetear estados solo si cambió la instancia
    if (!cached) {
      setVideoLoaded(false);
      setShowTitle(true);
    }
    
    if (instance?.background_video && instanceId && distributionBaseUrl) {
      invoke<number[]>('get_instance_background_video', {
        baseUrl: distributionBaseUrl,
        instanceId: instanceId,
        videoPath: instance.background_video
      })
        .then((videoBytes) => {
          // Convertir bytes a Uint8Array y crear un Blob URL
          const uint8Array = new Uint8Array(videoBytes);
          const blob = new Blob([uint8Array], { type: 'video/mp4' });
          const blobUrl = URL.createObjectURL(blob);
          setLocalVideoPath(blobUrl);
          // Guardar en caché sin marcar como cargado aún (se marcará cuando el video se cargue)
          videoCache.set(cacheKey, { blobUrl, loaded: false });
        })
        .catch((error) => {
          console.error('Error downloading video:', error);
          setLocalVideoPath(null);
        });
    } else {
      setLocalVideoPath(null);
    }
  }, [instance?.background_video, instanceId, distributionBaseUrl]);

  // Desvanecer el título cuando el video esté cargado
  useEffect(() => {
    if (videoLoaded) {
      const timer = setTimeout(() => {
        setShowTitle(false);
      }, 500); // Esperar 500ms antes de empezar a desvanecer
      return () => clearTimeout(timer);
    }
  }, [videoLoaded]);

  if (!instance) {
    return (
      <div className={`flex items-center justify-center h-full transition-all duration-500 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
        <div className="text-center">
          <h2 className="text-2xl font-bold text-white mb-4">
            Instancia no encontrada
          </h2>
          <p className="text-gray-300">
            La instancia seleccionada no se pudo cargar.
          </p>
        </div>
      </div>
    );
  }


  return (
    <div className="relative h-full w-full overflow-hidden">
      <div className="absolute inset-0 z-0">
        {localVideoPath ? (
          <video
            key={localVideoPath}
            autoPlay
            loop
            muted
            playsInline
            className="w-full h-full object-cover"
            style={{ 
              position: 'absolute', 
              top: 0, 
              left: 0, 
              width: '100%', 
              height: '100%',
              opacity: 0.8,
              filter: 'blur(2px)'
            }}
            onError={(e) => {
              console.error('Error loading video:', e);
              console.error('Video path:', localVideoPath);
            }}
            onLoadedData={() => {
              console.log('Video loaded successfully:', localVideoPath);
              setVideoLoaded(true);
              // Actualizar caché cuando el video se carga
              const cacheKey = `${instanceId}-${instance?.background_video}`;
              if (videoCache.has(cacheKey)) {
                videoCache.set(cacheKey, { blobUrl: localVideoPath, loaded: true });
              }
            }}
          >
            <source src={localVideoPath} type="video/mp4" />
            Tu navegador no soporta videos HTML5.
          </video>
        ) : (
          <>
            <div
              className="w-full h-full"
              style={{
                background: 'linear-gradient(135deg, #000000 0%, #0a0a0a 50%, #000000 100%)'
              }}
            />
            {/* Subtle neon accents in background */}
            <div className="absolute inset-0 z-5 opacity-10">
              <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-[#00ffff] rounded-full blur-3xl"></div>
              <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-[#ff00ff] rounded-full blur-3xl"></div>
            </div>
          </>
        )}
      </div>

      <div className="absolute inset-0 bg-black/60 z-10" />
      
      {/* Título de la instancia - se desvanece cuando el video está cargado */}
      {showTitle && instance && (
        <div 
          className={`absolute inset-0 z-15 flex items-center justify-center transition-all duration-700 ${
            videoLoaded ? 'opacity-0' : isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'
          }`}
          style={{
            fontFamily: '"Bebas Neue", cursive, sans-serif'
          }}
        >
          <h1 className="text-6xl md:text-8xl font-bold text-white drop-shadow-2xl tracking-wider">
            {instance.name}
          </h1>
        </div>
      )}


      <div className="relative z-20 h-full flex flex-col">
        {/* Spacer para empujar contenido abajo */}
        <div className="flex-1" />
        
        {/* Tags y botón al fondo */}
        <div className={`pb-12 flex flex-col items-center gap-6 transition-all duration-700 ${isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`}>
          {/* Tags */}
          <div className="flex items-center justify-center space-x-4 mb-4">
            <span 
              className="px-4 py-2 rounded-2xl border flex items-center space-x-2 shadow-xl backdrop-blur-xl transition-all duration-500 ease-out"
              style={{
                background: 'rgba(0, 0, 0, 0.6)',
                backdropFilter: 'blur(24px)',
                WebkitBackdropFilter: 'blur(24px)',
                boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.7)',
                borderColor: 'rgba(255, 255, 255, 0.2)',
                borderWidth: '1px'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.35)';
                e.currentTarget.style.background = 'rgba(0, 0, 0, 0.65)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                e.currentTarget.style.background = 'rgba(0, 0, 0, 0.6)';
              }}
            >
              <img src={minecraftIcon} alt="Minecraft" className="w-4 h-4 filter brightness-0 invert" />
              <span className="text-white font-semibold text-sm">{instance.minecraft_version}</span>
            </span>
            <span 
              className="px-4 py-2 rounded-2xl border shadow-xl backdrop-blur-xl transition-all duration-500 ease-out"
              style={{
                background: 'rgba(0, 0, 0, 0.6)',
                backdropFilter: 'blur(24px)',
                WebkitBackdropFilter: 'blur(24px)',
                boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.7)',
                borderColor: 'rgba(255, 255, 255, 0.2)',
                borderWidth: '1px'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.35)';
                e.currentTarget.style.background = 'rgba(0, 0, 0, 0.65)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                e.currentTarget.style.background = 'rgba(0, 0, 0, 0.6)';
              }}
            >
              <span className="text-white font-semibold text-sm">v{instance.version}</span>
            </span>
            {instance.mod_loader && (
              <span 
                className="px-4 py-2 rounded-2xl border flex items-center space-x-2 shadow-xl backdrop-blur-xl transition-all duration-500 ease-out"
                style={{
                  background: 'rgba(0, 0, 0, 0.6)',
                  backdropFilter: 'blur(24px)',
                  WebkitBackdropFilter: 'blur(24px)',
                  boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.7)',
                  borderColor: 'rgba(255, 255, 255, 0.2)',
                  borderWidth: '1px'
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.35)';
                  e.currentTarget.style.background = 'rgba(0, 0, 0, 0.65)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                  e.currentTarget.style.background = 'rgba(0, 0, 0, 0.6)';
                }}
              >
                <img
                  src={instance.mod_loader.type === 'fabric' ? fabricmcIcon : neoforgeIcon}
                  alt={instance.mod_loader.type}
                  className="w-4 h-4"
                />
                <span className="text-white font-semibold text-sm">{instance.mod_loader.version}</span>
              </span>
            )}
          </div>

          {/* Botón de jugar */}
          <LaunchButton
            onLaunch={() => onLaunch(instance)}
            className="text-center"
            isJavaInstalling={isJavaInstalling}
            instanceId={instanceId}
          />
        </div>
      </div>
    </div>
  );
};

export default InstanceView;

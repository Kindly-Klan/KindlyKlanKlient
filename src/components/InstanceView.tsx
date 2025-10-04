import React, { useState, useEffect } from 'react';
import LaunchButton from './LaunchButton';

// Importar íconos de mod loaders
import minecraftIcon from '@/assets/icons/minecraft.svg';
import fabricIcon from '@/assets/icons/fabricmc.svg';
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

const InstanceView: React.FC<InstanceViewProps> = ({
  instanceId,
  distribution,
  distributionBaseUrl,
  onLaunch,
  isJavaInstalling = false
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const instance = distribution.instances.find(inst => inst.id === instanceId);

  // Trigger animation when instance changes
  useEffect(() => {
    if (instanceId) {
      setIsVisible(false);
      const timer = setTimeout(() => setIsVisible(true), 50);
      return () => clearTimeout(timer);
    }
  }, [instanceId]);

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

  // Si tiene background, usar como fondo a pantalla completa
  const backgroundStyle = instance.background
    ? {
        backgroundImage: `url(${distributionBaseUrl}/${instance.background})`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        backgroundRepeat: 'no-repeat'
      }
    : {
        background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)'
      };

  // Estado para manejar carga de imágenes
  const [backgroundLoaded, setBackgroundLoaded] = React.useState(false);
  const [backgroundError, setBackgroundError] = React.useState(false);

  React.useEffect(() => {
    if (instance.background) {
      const img = new Image();
      img.onload = () => {
        setBackgroundLoaded(true);
        setBackgroundError(false);
      };
      img.onerror = () => {
        setBackgroundError(true);
        setBackgroundLoaded(false);
      };
      img.src = `${distributionBaseUrl}/${instance.background}`;
    } else {
      setBackgroundLoaded(true);
    }
  }, [instance.background, distributionBaseUrl]);

  return (
    <div className={`relative h-full w-full overflow-hidden transition-all duration-500 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
      {/* Background */}
      <div className="absolute inset-0 z-0">
        {instance.background && !backgroundLoaded && !backgroundError && (
          <div className="w-full h-full bg-gray-900 animate-pulse" />
        )}
        {backgroundError ? (
          <div
            className="w-full h-full"
            style={{
              background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)'
            }}
          />
        ) : (
          <div
            className="w-full h-full"
            style={backgroundStyle}
          />
        )}
      </div>

      {/* Overlay para mejor contraste */}
      <div className="absolute inset-0 bg-black/40 z-10" />

      {/* Content */}
      <div className="relative z-20 h-full flex flex-col">
        {/* Instance Info */}
        <div className="flex-1 flex items-center justify-center p-8">
          <div className="text-center max-w-2xl mx-auto">
            {/* Instance Icon */}
            <div className="mb-8">
              {instance.icon ? (
                <img
                  src={`${distributionBaseUrl}/${instance.icon}`}
                  alt={instance.name}
                  className="w-32 h-32 mx-auto rounded-2xl object-cover shadow-2xl border-4 border-white/20"
                />
              ) : (
                <div className="w-32 h-32 mx-auto rounded-2xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-2xl border-4 border-white/20">
                  <span className="text-white font-bold text-4xl">
                    {instance.name.charAt(0).toUpperCase()}
                  </span>
                </div>
              )}
            </div>

            {/* Instance Name */}
            <h1 className="text-5xl font-bold text-white mb-4 text-shadow-lg">
              {instance.name}
            </h1>

            {/* Instance Description */}
            <p className="text-xl text-gray-200 mb-8 leading-relaxed">
              {instance.description}
            </p>

            {/* Version Info */}
            <div className="flex items-center justify-center space-x-6 text-gray-300 mb-8">
              <span className="bg-black/50 backdrop-blur-sm px-4 py-2 rounded-full border border-white/20 flex items-center space-x-2 shadow-lg">
                <img src={minecraftIcon} alt="Minecraft" className="w-5 h-5 filter brightness-0 invert" />
                <span className="text-white font-medium">{instance.minecraft_version}</span>
              </span>
              <span className="bg-black/50 backdrop-blur-sm px-4 py-2 rounded-full border border-white/20 shadow-lg">
                <span className="text-white font-medium">v{instance.version}</span>
              </span>
              {instance.mod_loader && (
                <span className="bg-black/50 backdrop-blur-sm px-4 py-2 rounded-full border border-white/20 flex items-center space-x-2 shadow-lg">
                  <img
                    src={instance.mod_loader.type === 'fabric' ? fabricIcon : neoforgeIcon}
                    alt={instance.mod_loader.type}
                    className="w-5 h-5"
                  />
                  <span className="text-white font-medium">{instance.mod_loader.version}</span>
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Launch Button with advanced animations */}
        <div className="flex justify-center pb-12">
          <LaunchButton
            onLaunch={() => onLaunch(instance)}
            className="text-center"
            isJavaInstalling={isJavaInstalling}
          />
        </div>
      </div>
    </div>
  );
};

export default InstanceView;

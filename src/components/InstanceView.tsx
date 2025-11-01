import React, { useState, useEffect } from 'react';
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

const InstanceView: React.FC<InstanceViewProps> = ({
  instanceId,
  distribution,
  distributionBaseUrl,
  onLaunch,
  isJavaInstalling = false
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const instance = distribution.instances.find(inst => inst.id === instanceId);

  // Animate on instance change
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


  // Video de fondo si está disponible, sino fondo por defecto
  const hasVideo = instance.background_video;

  return (
    <div className="relative h-full w-full overflow-hidden">
      <div className="absolute inset-0 z-0">
        {hasVideo ? (
          <video
            autoPlay
            loop
            muted
            playsInline
            className="w-full h-full object-cover"
          >
            <source src={`${distributionBaseUrl}/${instance.background_video}`} type="video/mp4" />
          </video>
        ) : (
          <div
            className="w-full h-full"
            style={{
              background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)'
            }}
          />
        )}
      </div>

      <div className="absolute inset-0 bg-black/40 z-10" />


      <div className="relative z-20 h-full flex flex-col items-center justify-center">

        <div className={`flex flex-col items-center justify-center gap-6 transition-all duration-700 ${isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`}>
          {/* Tags */}
          <div className="flex items-center justify-center space-x-4 mb-4">
            <span className="bg-gradient-to-r from-green-600/80 to-green-700/80 backdrop-blur-sm px-4 py-2 rounded-full border border-green-400/30 flex items-center space-x-2 shadow-lg">
              <img src={minecraftIcon} alt="Minecraft" className="w-4 h-4 filter brightness-0 invert" />
              <span className="text-white font-semibold text-sm">{instance.minecraft_version}</span>
            </span>
            <span className="bg-gradient-to-r from-blue-600/80 to-blue-700/80 backdrop-blur-sm px-4 py-2 rounded-full border border-blue-400/30 shadow-lg">
              <span className="text-white font-semibold text-sm">v{instance.version}</span>
            </span>
            {instance.mod_loader && (
              <span className="bg-gradient-to-r from-purple-600/80 to-purple-700/80 backdrop-blur-sm px-4 py-2 rounded-full border border-purple-400/30 flex items-center space-x-2 shadow-lg">
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
          />
        </div>
      </div>
    </div>
  );
};

export default InstanceView;

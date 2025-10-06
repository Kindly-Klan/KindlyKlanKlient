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
      <div className={`relative h-full w-full overflow-hidden transition-all duration-700 ${isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`}>

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


      <div className="absolute inset-0 bg-black/40 z-10" />


      <div className="relative z-20 h-full flex flex-col">

        <div className={`flex-1 flex items-center justify-center p-8 transition-all duration-700 delay-200 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
          <div className="text-center max-w-2xl mx-auto">

            <div className="mb-6">
              {instance.icon ? (
                <img
                  src={`${distributionBaseUrl}/${instance.icon}`}
                  alt={instance.name}
                  className="w-40 h-40 mx-auto rounded-3xl object-cover shadow-2xl border-4 border-white/30 hover:border-white/50 transition-all duration-300"
                />
              ) : (
                <div className="w-40 h-40 mx-auto rounded-3xl bg-gradient-to-br from-blue-500 via-purple-600 to-indigo-700 flex items-center justify-center shadow-2xl border-4 border-white/30 hover:border-white/50 transition-all duration-300">
                  <span className="text-white font-black text-5xl drop-shadow-lg">
                    {instance.name.charAt(0).toUpperCase()}
                  </span>
                </div>
              )}
            </div>

            <div className="flex items-center justify-center gap-4 mb-8">
              <h1 className="text-6xl font-black tracking-wide text-center text-white drop-shadow-lg">
                {instance.name}
              </h1>
            </div>

            <p className="text-lg text-gray-300 mb-8 leading-relaxed max-w-xl mx-auto bg-black/20 backdrop-blur-sm px-6 py-4 rounded-2xl border border-white/10">
              {instance.description}
            </p>

            <div className="flex items-center justify-center space-x-4 text-gray-300 mb-8">
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
          </div>
        </div>

        <div className={`flex justify-center pb-12 transition-all duration-700 delay-500 ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
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

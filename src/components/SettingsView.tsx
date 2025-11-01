import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { UpdaterService } from '@/services/updater';
import type { UpdateState, UpdateProgress } from '@/types/updater';

interface SettingsViewProps {
  onClose: () => void;
}

const SettingsView: React.FC<SettingsViewProps> = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [minRam, setMinRam] = useState(2.0);
  const [maxRam, setMaxRam] = useState(4.0);
  const [systemRam, setSystemRam] = useState(8);
  
  // JVM Advanced Settings
  const [jvmArgs, setJvmArgs] = useState('');
  const [garbageCollector, setGarbageCollector] = useState('G1');
  
  // Window Settings
  const [windowWidth, setWindowWidth] = useState(1280);
  const [windowHeight, setWindowHeight] = useState(720);

  // Update Settings
  const [updateState, setUpdateState] = useState<UpdateState | null>(null);
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false);
  const [isDownloadingUpdate, setIsDownloadingUpdate] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<UpdateProgress | null>(null);

  // Scroll state for shadow effect
  const [isScrolled, setIsScrolled] = useState(false);

  // Animate on mount
  useEffect(() => {
    setIsVisible(true);
  }, []);

  // Get system RAM and load saved configuration
  useEffect(() => {
    const initializeConfig = async () => {
      try {
        // Get system RAM
        const ram = await invoke<number>('get_system_ram');
        setSystemRam(ram);
        
        // Load saved RAM configuration
        const [savedMinRam, savedMaxRam] = await invoke<[number, number]>('load_ram_config');
        
        // Validate saved values against system limits
        const maxRamLimit = Math.max(2, Math.floor(ram * 0.75));
        
        // Set values, ensuring they're within valid ranges
        const validMinRam = Math.max(0.5, Math.min(savedMinRam, maxRamLimit));
        const validMaxRam = Math.max(validMinRam, Math.min(savedMaxRam, maxRamLimit));
        
        setMinRam(validMinRam);
        setMaxRam(validMaxRam);
        
        // Load advanced configuration
        const [savedJvmArgs, savedGc, savedWidth, savedHeight] = 
          await invoke<[string, string, number, number]>('load_advanced_config');
        
        setJvmArgs(savedJvmArgs);
        setGarbageCollector(savedGc);
        setWindowWidth(savedWidth);
        setWindowHeight(savedHeight);
        
      } catch (error) {
        console.error('Error initializing config:', error);
        // Use defaults if loading fails
        setMinRam(2.0);
        setMaxRam(4.0);
        setJvmArgs('');
        setGarbageCollector('G1');
        setWindowWidth(1280);
        setWindowHeight(720);
      }
    };
    
    initializeConfig();
  }, []);

  // Initialize update state and event listeners
  useEffect(() => {
    const initializeUpdates = async () => {
      try {
        // Load current update state
        const state = await UpdaterService.getUpdateState();
        setUpdateState(state);

        // Set up progress callback
        UpdaterService.setProgressCallback((progress) => {
          setDownloadProgress(progress);
          if (progress.status.includes('completada') || progress.status.includes('completado')) {
            setIsDownloadingUpdate(false);
            // Refresh update state
            UpdaterService.getUpdateState().then(setUpdateState);
          }
        });

        // Start listening to update events
        await UpdaterService.startListeningToEvents();
      } catch (error) {
        console.error('Error initializing updates:', error);
      }
    };

    initializeUpdates();
  }, []);

  // Save configuration when values change
  const saveConfig = async (newMinRam: number, newMaxRam: number) => {
    try {
      await invoke('save_ram_config', { 
        minRam: newMinRam, 
        maxRam: newMaxRam 
      });
    } catch (error) {
      console.error('Error saving RAM config:', error);
    }
  };

  // Save advanced settings
  const saveAdvancedConfig = async () => {
    try {
      await invoke('save_advanced_config', {
        jvmArgs,
        garbageCollector,
        windowWidth,
        windowHeight
      });
    } catch (error) {
      console.error('Error saving advanced config:', error);
    }
  };

  // Handle scroll for shadow effect
  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const scrollTop = e.currentTarget.scrollTop;
    setIsScrolled(scrollTop > 20);
  };

  // Update handlers
  const handleCheckForUpdates = async () => {
    setIsCheckingUpdates(true);
    try {
      const result = await UpdaterService.checkForUpdates();
      if (result.available) {
        // Refresh update state
        const newState = await UpdaterService.getUpdateState();
        setUpdateState(newState);
        // Mostrar notificación de actualización disponible
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-green-500/20 border border-green-500/30 text-green-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = `✓ Actualización ${result.version} disponible`;
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 3000);
      } else {
        // Mostrar que no hay actualizaciones
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-blue-500/20 border border-blue-500/30 text-blue-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = '✓ Ya estás en la última versión';
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 3000);
      }
    } catch (error) {
      console.error('Error checking for updates:', error);
      const toast = document.createElement('div');
      toast.className = 'fixed bottom-4 right-4 bg-red-500/20 border border-red-500/30 text-red-300 px-6 py-3 rounded-lg shadow-lg z-50';
      toast.textContent = '✗ Error al verificar actualizaciones';
      document.body.appendChild(toast);
      setTimeout(() => toast.remove(), 3000);
    } finally {
      setIsCheckingUpdates(false);
    }
  };

  const handleDownloadUpdate = async () => {
    setIsDownloadingUpdate(true);
    setDownloadProgress(null);
    try {
      const result = await UpdaterService.downloadUpdateSilent();
      if (result.success) {
        // Refresh update state
        const newState = await UpdaterService.getUpdateState();
        setUpdateState(newState);
        // Mostrar notificación de descarga completada
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-green-500/20 border border-green-500/30 text-green-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = '✓ Actualización descargada correctamente';
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 3000);
      } else {
        // Mostrar error en la descarga
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-red-500/20 border border-red-500/30 text-red-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = '✗ Error al descargar la actualización';
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 3000);
      }
    } catch (error) {
      console.error('Error downloading update:', error);
      const toast = document.createElement('div');
      toast.className = 'fixed bottom-4 right-4 bg-red-500/20 border border-red-500/30 text-red-300 px-6 py-3 rounded-lg shadow-lg z-50';
      toast.textContent = '✗ Error al descargar la actualización';
      document.body.appendChild(toast);
      setTimeout(() => toast.remove(), 3000);
    } finally {
      setIsDownloadingUpdate(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateState?.download_ready) return;
    
    const confirmed = window.confirm('¿Estás seguro de que quieres instalar la actualización? La aplicación se reiniciará.');
    if (!confirmed) return;

    try {
      const result = await UpdaterService.installUpdate();
      if (result.success) {
        // Mostrar notificación de instalación exitosa
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-green-500/20 border border-green-500/30 text-green-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = '✓ Actualización instalada. Reiniciando...';
        document.body.appendChild(toast);
        // The app will restart automatically
      } else {
        // Mostrar error en la instalación
        const toast = document.createElement('div');
        toast.className = 'fixed bottom-4 right-4 bg-red-500/20 border border-red-500/30 text-red-300 px-6 py-3 rounded-lg shadow-lg z-50';
        toast.textContent = '✗ Error al instalar la actualización';
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 3000);
      }
    } catch (error) {
      console.error('Error installing update:', error);
      const toast = document.createElement('div');
      toast.className = 'fixed bottom-4 right-4 bg-red-500/20 border border-red-500/30 text-red-300 px-6 py-3 rounded-lg shadow-lg z-50';
      toast.textContent = '✗ Error al instalar la actualización';
      document.body.appendChild(toast);
      setTimeout(() => toast.remove(), 3000);
    }
  };

  const handleMinRamChange = (value: number) => {
    const maxRamLimit = Math.max(2, Math.floor(systemRam * 0.75));
    if (value <= maxRam && value <= maxRamLimit) {
      setMinRam(value);
      saveConfig(value, maxRam);
    }
  };

  const handleMaxRamChange = (value: number) => {
    const maxRamLimit = Math.max(2, Math.floor(systemRam * 0.75));
    if (value >= minRam && value <= maxRamLimit) {
      setMaxRam(value);
      saveConfig(minRam, value);
    }
  };


  return (
    <div className="relative h-full w-full overflow-hidden">
      
      {/* Background */}
      <div className="absolute inset-0 z-0">
        <div
          className="w-full h-full"
          style={{
            background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)'
          }}
        />
      </div>

      {/* Overlay */}
      <div className="absolute inset-0 bg-black/40 z-10" />

       {/* Content */}
       <div className="relative z-20 h-full flex flex-col">
         
          {/* Header with scroll shadow effect */}
          <div className={`pt-8 px-8 transition-all duration-500 ${isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'} ${
            isScrolled ? 'shadow-2xl shadow-black/50 bg-gradient-to-b from-black/20 to-transparent pb-4' : ''
          }`}>
            <div className="flex items-center gap-4 mb-6">
              <h1 className={`text-4xl font-black tracking-wide text-white drop-shadow-lg transition-all duration-500 ${
                isScrolled ? 'opacity-80 scale-95' : 'opacity-100 scale-100'
              }`}>
                Configuración
              </h1>
            </div>
            
            {/* Separator with fade effect */}
            <div className={`h-px bg-gradient-to-r from-transparent via-white/30 to-transparent mb-8 transition-all duration-500 ${
              isScrolled ? 'opacity-50 scale-y-50' : 'opacity-100 scale-y-100'
            }`}></div>
          </div>

         {/* Settings Content */}
         <div 
           className={`px-8 pb-8 scroll-container transition-all duration-700 delay-300 ${isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`} 
           style={{ 
             flex: '1 1 0',
             minHeight: '0',
             overflowY: 'auto',
             WebkitOverflowScrolling: 'touch',
             position: 'relative'
           }}
           onScroll={handleScroll}
         >
           {/* Fade overlay at top when scrolled */}
           <div className={`absolute top-0 left-0 right-0 h-8 bg-gradient-to-b from-black/60 to-transparent z-10 pointer-events-none transition-opacity duration-500 ${
             isScrolled ? 'opacity-100' : 'opacity-0'
           }`}></div>
          
          {/* Java Configuration Section */}
          <div className="max-w-4xl mx-auto">
            <div className="bg-black/20 backdrop-blur-sm rounded-2xl border border-white/10 p-6">
              
              {/* Section Header */}
              <div className="flex items-center gap-3 mb-6">
                <div className="w-8 h-8 flex items-center justify-center">
                  <svg preserveAspectRatio="xMidYMid" viewBox="0 0 256 346" className="w-6 h-6">
                    <path d="M83 267s-14 8 9 11c27 3 41 2 71-3 0 0 8 5 19 9-67 29-153-2-99-17M74 230s-15 11 8 13c29 3 52 3 92-4 0 0 6 5 15 8-82 24-173 2-115-17" fill="#5382A1"/>
                    <path d="M144 166c17 19-4 36-4 36s42-22 22-49c-18-26-32-38 44-82 0 0-119 29-62 95" fill="#E76F00"/>
                    <path d="M233 295s10 8-10 15c-39 12-163 15-197 0-12-5 11-13 18-14l12-2c-14-9-89 19-38 28 138 22 251-10 215-27M89 190s-63 15-22 21c17 2 51 2 83-1 26-2 52-7 52-7l-16 9c-64 16-187 8-151-9 30-14 54-13 54-13M202 253c64-33 34-66 13-61l-7 2s2-3 6-5c41-14 73 43-14 66l2-2" fill="#5382A1"/>
                    <path d="M162 0s36 36-34 91c-56 45-12 70 0 99-32-30-56-56-40-80 23-35 89-53 74-110" fill="#E76F00"/>
                    <path d="M95 345c62 4 158-3 160-32 0 0-4 11-51 20-53 10-119 9-158 2 0 0 8 7 49 10" fill="#5382A1"/>
                  </svg>
                </div>
                <h2 className="text-2xl font-bold text-white">Configuración de Java</h2>
              </div>

              {/* RAM Configuration */}
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold text-white mb-4">Memoria RAM</h3>
                  
                  {/* Min RAM Slider */}
                  <div className="mb-6">
                    <div className="flex items-center justify-between mb-2">
                      <label className="text-white/80 font-medium">RAM Mínima</label>
                      <span className="text-white font-bold">{minRam.toFixed(1)} GB</span>
                    </div>
                    <div className="relative ">
                      <input
                        type="range"
                        min="0.5"
                        max={Math.max(2, Math.floor(systemRam * 0.75))}
                        step="0.5"
                        value={minRam}
                        onChange={(e) => handleMinRamChange(parseFloat(e.target.value))}
                        className="w-full slider"
                      />
                    </div>
                  </div>

                  {/* Max RAM Slider */}
                  <div className="mb-4">
                    <div className="flex items-center justify-between mb-2">
                      <label className="text-white/80 font-medium">RAM Máxima</label>
                      <span className="text-white font-bold">{maxRam.toFixed(1)} GB</span>
                    </div>
                    <div className="relative">
                      <input
                        type="range"
                        min="0.5"
                        max={Math.max(2, Math.floor(systemRam * 0.75))}
                        step="0.5"
                        value={maxRam}
                        onChange={(e) => handleMaxRamChange(parseFloat(e.target.value))}
                        className="w-full slider"
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* JVM Advanced Configuration Section */}
            <div className="bg-black/20 backdrop-blur-sm rounded-2xl border border-white/10 p-6 mt-6">
              
              {/* Section Header */}
              <div className="flex items-center gap-3 mb-6">
                <div className="w-8 h-8 flex items-center justify-center">
                  <svg className="w-6 h-6 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                  </svg>
                </div>
                <h2 className="text-2xl font-bold text-white">Configuración JVM Avanzada</h2>
              </div>

              <div className="space-y-6">
                 {/* Garbage Collector */}
                 <div>
                   <div className="flex items-center gap-2 mb-3">
                     <label className="text-white/80 font-medium">Garbage Collector</label>
                     <div className="relative group">
                       <button className="faq-button-small">
                         <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 512" className="w-4 h-4">
                           <path d="M80 160c0-35.3 28.7-64 64-64h32c35.3 0 64 28.7 64 64v3.6c0 21.8-11.1 42.1-29.4 53.8l-42.2 27.1c-25.2 16.2-40.4 44.1-40.4 74V320c0 17.7 14.3 32 32 32s32-14.3 32-32v-1.4c0-8.2 4.2-15.8 11-20.2l42.2-27.1c36.6-23.6 58.8-64.1 58.8-107.7V160c0-70.7-57.3-128-128-128H144C73.3 32 16 89.3 16 160c0 17.7 14.3 32 32 32s32-14.3 32-32zm80 320a40 40 0 1 0 0-80 40 40 0 1 0 0 80z" fill="white"/>
                         </svg>
                         <span className="tooltip-small">
                           El Garbage Collector (GC) libera memoria no utilizada en Java. G1 es balanceado, ZGC tiene pausas ultra-bajas para gaming, y Parallel maximiza el rendimiento.
                         </span>
                       </button>
                     </div>
                   </div>
                  <div className="grid grid-cols-3 gap-3">
                    {['G1', 'ZGC', 'Parallel'].map((gc) => (
                      <button
                        key={gc}
                        onClick={() => {
                          setGarbageCollector(gc);
                          saveAdvancedConfig();
                        }}
                        className={`p-3 rounded-lg border transition-all duration-200 ${
                          garbageCollector === gc
                            ? 'bg-purple-500/20 border-purple-400 cursor-pointer text-purple-300'
                            : 'bg-white/5 border-white/10 text-white/70 hover:bg-white/10 cursor-pointer'
                        }`}
                      >
                        <div className="text-sm font-semibold">{gc}</div>
                        <div className="text-xs opacity-70">
                          {gc === 'G1' && 'Recomendado'}
                          {gc === 'ZGC' && 'Baja latencia'}
                          {gc === 'Parallel' && 'Alto rendimiento'}
                        </div>
                      </button>
                    ))}
                  </div>
                </div>

                {/* JVM Arguments */}
                <div>
                  <label className="block text-white/80 font-medium mb-3">Argumentos JVM Adicionales</label>
                  <textarea
                    value={jvmArgs}
                    onChange={(e) => setJvmArgs(e.target.value)}
                    onBlur={saveAdvancedConfig}
                    placeholder="-XX:+UseStringDeduplication -XX:+OptimizeStringConcat"
                    className="w-full h-20 bg-white/5 border border-white/10 rounded-lg px-4 py-3 text-white placeholder-white/50 focus:outline-none focus:border-purple-400 focus:bg-white/10 transition-all duration-200 resize-none"
                  />
                  <div className="text-xs text-red-500/50 mt-2 font-bold italic shadow-lg">
                    Utiliza este campo solo si sabes lo que estás haciendo.
                  </div>
                </div>
              </div>
            </div>

            {/* Window Configuration Section */}
            <div className="bg-black/20 backdrop-blur-sm rounded-2xl border border-white/10 p-6 mt-6">
              
              {/* Section Header */}
              <div className="flex items-center gap-3 mb-6">
                <div className="w-8 h-8 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 20 20">
                <path fill="#FFA500" d="M6 3a3 3 0 0 0-3 3v8a3 3 0 0 0 3 3h3.6a5.465 5.465 0 0 1-.393-1H6a2 2 0 0 1-2-2V7h12v2.207c.349.099.683.23 1 .393V6a3 3 0 0 0-3-3H6ZM4 6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2H4Zm8.065 5.442a2 2 0 0 1-1.43 2.478l-.462.118a4.734 4.734 0 0 0 .01 1.016l.35.083a2 2 0 0 1 1.456 2.519l-.127.422c.258.204.537.378.835.518l.325-.344a2 2 0 0 1 2.91.002l.337.358c.292-.135.568-.302.822-.498l-.156-.556a2 2 0 0 1 1.43-2.479l.46-.117a4.7 4.7 0 0 0-.01-1.017l-.348-.082a2 2 0 0 1-1.456-2.52l.126-.421a4.318 4.318 0 0 0-.835-.519l-.325.344a2 2 0 0 1-2.91-.001l-.337-.358a4.31 4.31 0 0 0-.821.497l.156.557Zm2.434 4.058a1 1 0 1 1 0-2a1 1 0 0 1 0 2Z"/>
                </svg>
                </div>
                <h2 className="text-2xl font-bold text-white">Configuración de Ventana</h2>
              </div>

              <div className="space-y-6">
                {/* Resolution */}
                <div>
                   <div>
                     <label className="block text-white/80 font-medium mb-3">Resolución</label>
                     <div className="grid grid-cols-2 gap-4">
                       <div>
                         <label className="flex items-center gap-2 text-white/60 text-sm mb-2">
                           Ancho
                           <span className="inline-flex items-center">
                             <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24">
                                <path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M22 12H2m20 0l-4 4m4-4l-4-4M2 12l4 4m-4-4l4-4"/>
                             </svg>
                           </span>
                         </label>
                         <input
                           type="number"
                           value={windowWidth}
                           onChange={(e) => {
                             const value = parseInt(e.target.value) || 1280;
                             setWindowWidth(value);
                             saveAdvancedConfig();
                           }}
                           min="800"
                           max="7680"
                           className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-400 focus:bg-white/10 transition-all duration-200"
                           placeholder="1280"
                         />
                       </div>
                       <div>
                         <label className="flex items-center gap-2 text-white/60 text-sm mb-2">
                           Alto
                           <span className="inline-flex items-center">
                           <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 15 15">
    <path fill="currentColor" fill-rule="evenodd" d="M7.181 1.682a.45.45 0 0 1 .637 0l2.5 2.5a.45.45 0 0 1-.637.636L7.95 3.086v8.828l1.731-1.732a.45.45 0 0 1 .637.636l-2.5 2.5a.45.45 0 0 1-.637 0l-2.5-2.5a.45.45 0 0 1 .637-.636l1.732 1.732V3.086L5.317 4.818a.45.45 0 0 1-.637-.636l2.5-2.5Z" clip-rule="evenodd"/>
</svg>
                           </span>
                         </label>
                           
                         <input
                           type="number"
                           value={windowHeight}
                           onChange={(e) => {
                             const value = parseInt(e.target.value) || 720;
                             setWindowHeight(value);
                             saveAdvancedConfig();
                           }}
                           min="600"
                           max="4320"
                           className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-400 focus:bg-white/10 transition-all duration-200"
                           placeholder="720"
                         />
                        </div>
                      </div>
                    </div>
                  </div>
               </div>
             </div>

            {/* Update Configuration Section */}
            <div className="bg-black/20 backdrop-blur-sm rounded-2xl border border-white/10 p-6 mt-6">
              
              {/* Section Header */}
              <div className="flex items-center gap-3 mb-6">
                <div className="w-8 h-8 flex items-center justify-center">
                  <svg className="w-6 h-6 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                </div>
                <h2 className="text-2xl font-bold text-white">Actualizaciones</h2>
              </div>

              <div className="space-y-6">
                {/* Current Version */}
                <div>
                  <div className="flex items-center justify-between mb-3">
                    <label className="text-white/80 font-medium">Versión Actual</label>
                    <span className="text-white font-bold">{updateState?.current_version || '0.1.0'}</span>
                  </div>
                </div>

                {/* Update Status */}
                <div>
                  <div className="flex items-center justify-between mb-3">
                    <label className="text-white/80 font-medium">Estado</label>
                    <div className="flex items-center gap-2">
                      {updateState?.available_version ? (
                        <span className="px-3 py-1 bg-orange-500/20 text-orange-300 rounded-full text-sm font-medium border border-orange-500/30">
                          Actualización disponible
                        </span>
                      ) : updateState?.download_ready ? (
                        <span className="px-3 py-1 bg-blue-500/20 text-blue-300 rounded-full text-sm font-medium border border-blue-500/30">
                          Lista para instalar
                        </span>
                      ) : (
                        <span className="px-3 py-1 bg-green-500/20 text-green-300 rounded-full text-sm font-medium border border-green-500/30">
                          Actualizado
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Available Version */}
                {updateState?.available_version && (
                  <div>
                    <div className="flex items-center justify-between mb-3">
                      <label className="text-white/80 font-medium">Nueva Versión</label>
                      <span className="text-white font-bold">{updateState.available_version}</span>
                    </div>
                  </div>
                )}

                {/* Download Progress */}
                {downloadProgress && (
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <label className="text-white/80 font-medium">Progreso</label>
                      <span className="text-white text-sm">{downloadProgress.status}</span>
                    </div>
                    <div className="w-full bg-white/10 rounded-full h-2">
                      <div 
                        className="bg-gradient-to-r from-blue-500 to-purple-500 h-2 rounded-full transition-all duration-300"
                        style={{ width: `${downloadProgress.percentage}%` }}
                      ></div>
                    </div>
                    <div className="text-xs text-white/60 mt-1">
                      {downloadProgress.percentage}% 
                    </div>
                  </div>
                )}

                {/* Action Buttons */}
                <div className="flex flex-wrap gap-3">
                  <button
                    onClick={handleCheckForUpdates}
                    disabled={isCheckingUpdates}
                    className="px-4 py-2 bg-blue-500/20 hover:bg-blue-500/30 disabled:bg-gray-500/20 disabled:cursor-not-allowed text-blue-300 border border-blue-500/30 rounded-lg transition-all duration-200 flex items-center gap-2"
                  >
                    {isCheckingUpdates ? (
                      <>
                        <div className="w-4 h-4 border-2 border-blue-300 border-t-transparent rounded-full animate-spin"></div>
                        Verificando...
                      </>
                    ) : (
                      <>
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        Verificar actualizaciones
                      </>
                    )}
                  </button>

                  {updateState?.available_version && !updateState.download_ready && (
                    <button
                      onClick={handleDownloadUpdate}
                      disabled={isDownloadingUpdate}
                      className="px-4 py-2 bg-orange-500/20 hover:bg-orange-500/30 disabled:bg-gray-500/20 disabled:cursor-not-allowed text-orange-300 border border-orange-500/30 rounded-lg transition-all duration-200 flex items-center gap-2"
                    >
                      {isDownloadingUpdate ? (
                        <>
                          <div className="w-4 h-4 border-2 border-orange-300 border-t-transparent rounded-full animate-spin"></div>
                          Descargando...
                        </>
                      ) : (
                        <>
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                          </svg>
                          Descargar actualización
                        </>
                      )}
                    </button>
                  )}

                  {updateState?.download_ready && (
                    <button
                      onClick={handleInstallUpdate}
                      className="px-4 py-2 bg-green-500/20 hover:bg-green-500/30 text-green-300 border border-green-500/30 rounded-lg transition-all duration-200 flex items-center gap-2"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                      Instalar actualización
                    </button>
                  )}
                </div>                
              </div>
            </div>
           </div>
         </div>

      </div>

       <style dangerouslySetInnerHTML={{
         __html: `
           .slider {
             -webkit-appearance: none;
             width: 100%;
             height: 10px;
             border-radius: 5px;
             background-color: #4158D0;
             background-image: linear-gradient(43deg, #4158D0 0%, #C850C0 46%, #FFCC70 100%);
             outline: none;
             opacity: 0.9;
             -webkit-transition: .2s;
             transition: opacity .2s;
           }

           .slider:hover {
             opacity: 1;
           }

           .slider::-webkit-slider-thumb {
             -webkit-appearance: none;
             appearance: none;
             width: 20px;
             height: 20px;
             border-radius: 50%;
             background-color: #4c00ff;
             background-image: linear-gradient(160deg, #4900f5 0%, #80D0C7 100%);
             cursor: pointer;
             box-shadow: 0 2px 8px rgba(0,0,0,0.3);
             transition: all 0.2s ease-in-out;
           }

           .slider::-webkit-slider-thumb:hover {
             transform: scale(1.1);
             box-shadow: 0 4px 12px rgba(0,0,0,0.4);
           }

           .slider::-moz-range-thumb {
             width: 20px;
             height: 20px;
             border-radius: 50%;
             background-color: #0093E9;
             background-image: linear-gradient(160deg, #0093E9 0%, #80D0C7 100%);
             cursor: pointer;
             border: none;
             box-shadow: 0 2px 8px rgba(0,0,0,0.3);
             transition: all 0.2s ease-in-out;
           }

           .slider::-moz-range-thumb:hover {
             transform: scale(1.1);
             box-shadow: 0 4px 12px rgba(0,0,0,0.4);
           }

           .slider::-moz-range-track {
             height: 10px;
             background-color: #4158D0;
             background-image: linear-gradient(43deg, #4158D0 0%, #C850C0 46%, #FFCC70 100%);
             border-radius: 5px;
             border: none;
           }

           .slider::-webkit-slider-track {
             height: 10px;
             background-color: #4158D0;
             background-image: linear-gradient(43deg, #4158D0 0%, #C850C0 46%, #FFCC70 100%);
             border-radius: 5px;
             border: none;
           }

           .faq-button-small {
             width: 24px;
             height: 24px;
             border-radius: 50%;
             border: none;
             background-color: #ffe53b;
             background-image: linear-gradient(147deg, #ffe53b 0%, #ff2525 74%);
             display: flex;
             align-items: center;
             justify-content: center;
             cursor: pointer;
             box-shadow: 0px 4px 8px rgba(0, 0, 0, 0.2);
             position: relative;
             transition: all 0.2s ease-in-out;
           }

           .faq-button-small:hover {
             transform: scale(1.1);
             box-shadow: 0px 6px 12px rgba(0, 0, 0, 0.3);
           }

           .faq-button-small svg {
             height: 1em;
             fill: white;
           }

           .faq-button-small:hover svg {
             animation: jello-vertical 0.7s both;
           }

           @keyframes jello-vertical {
             0% {
               transform: scale3d(1, 1, 1);
             }
             30% {
               transform: scale3d(0.75, 1.25, 1);
             }
             40% {
               transform: scale3d(1.25, 0.75, 1);
             }
             50% {
               transform: scale3d(0.85, 1.15, 1);
             }
             65% {
               transform: scale3d(1.05, 0.95, 1);
             }
             75% {
               transform: scale3d(0.95, 1.05, 1);
             }
             100% {
               transform: scale3d(1, 1, 1);
             }
           }

           .tooltip-small {
             position: absolute;
             top: -80px;
             left: 50%;
             transform: translateX(-50%);
             opacity: 0;
             background-color: #ffe53b;
             background-image: linear-gradient(147deg, #ffe53b 0%, #ff2525 74%);
             color: white;
             padding: 8px 12px;
             border-radius: 8px;
             display: flex;
             align-items: center;
             justify-content: center;
             transition-duration: 0.3s;
             pointer-events: none;
             letter-spacing: 0.5px;
             font-size: 12px;
             font-weight: 500;
             text-align: center;
             max-width: 280px;
             width: max-content;
             z-index: 50;
           }

           .tooltip-small::before {
             position: absolute;
             content: "";
             width: 8px;
             height: 8px;
             background-color: #ff2525;
             background-size: 1000%;
             background-position: center;
             transform: rotate(45deg);
             bottom: -4px;
             left: 50%;
             transform: translateX(-50%) rotate(45deg);
             transition-duration: 0.3s;
           }

           .faq-button-small:hover .tooltip-small {
             top: -90px;
             opacity: 1;
             transition-duration: 0.3s;
           }

           /* Custom scrollbar for Tauri */
           ::-webkit-scrollbar {
             width: 8px;
           }

           ::-webkit-scrollbar-track {
             background: rgba(255, 255, 255, 0.1);
             border-radius: 4px;
           }

           ::-webkit-scrollbar-thumb {
             background: rgba(255, 255, 255, 0.3);
             border-radius: 4px;
             transition: background 0.2s ease;
           }

           ::-webkit-scrollbar-thumb:hover {
             background: rgba(255, 255, 255, 0.5);
           }

           /* Force scrollbar to be visible */
           .scroll-container {
             scrollbar-width: thin;
             scrollbar-color: rgba(255, 255, 255, 0.3) rgba(255, 255, 255, 0.1);
           }
         `
       }} />
    </div>
  );
};

export default SettingsView;

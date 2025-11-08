import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { MinecraftVersionInfo, FabricLoaderVersion, LocalInstance } from '@/types/local-instances';

interface CreateLocalInstanceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onInstanceCreated: (instance: LocalInstance) => void;
}

const CreateLocalInstanceModal: React.FC<CreateLocalInstanceModalProps> = ({
  isOpen,
  onClose,
  onInstanceCreated,
}) => {
  const [step, setStep] = useState(1);
  const [name, setName] = useState('');
  const [generatedId, setGeneratedId] = useState('');
  const [minecraftVersions, setMinecraftVersions] = useState<MinecraftVersionInfo[]>([]);
  const [selectedMinecraftVersion, setSelectedMinecraftVersion] = useState('');
  const [fabricVersions, setFabricVersions] = useState<FabricLoaderVersion[]>([]);
  const [selectedFabricVersion, setSelectedFabricVersion] = useState('');
  const [isLoadingVersions, setIsLoadingVersions] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState('');

  // Reset state when modal opens
  useEffect(() => {
    if (isOpen) {
      setStep(1);
      setName('');
      setGeneratedId('');
      setSelectedMinecraftVersion('');
      setSelectedFabricVersion('');
      setMinecraftVersions([]);
      setFabricVersions([]);
      setError('');
    }
  }, [isOpen]);

  // Generate ID preview when name changes
  useEffect(() => {
    if (name.trim()) {
      const slug = name
        .toLowerCase()
        .replace(/[^a-z0-9\s-]/g, '')
        .replace(/\s+/g, '-')
        .replace(/-+/g, '-')
        .trim();
      setGeneratedId(`${slug}-xxxxx`);
    } else {
      setGeneratedId('');
    }
  }, [name]);

  // Load Minecraft versions when step 2 is reached
  useEffect(() => {
    if (step === 2 && minecraftVersions.length === 0) {
      loadMinecraftVersions();
    }
  }, [step]);

  // Load Fabric versions when Minecraft version is selected
  useEffect(() => {
    if (selectedMinecraftVersion && step === 3) {
      loadFabricVersions();
    }
  }, [selectedMinecraftVersion, step]);

  const loadMinecraftVersions = async () => {
    setIsLoadingVersions(true);
    setError('');
    try {
      const versions = await invoke<MinecraftVersionInfo[]>('get_minecraft_versions');
      setMinecraftVersions(versions);
      console.log('Loaded Minecraft versions:', versions.length);
    } catch (error) {
      console.error('Error loading Minecraft versions:', error);
      setError('Error al cargar versiones de Minecraft');
    } finally {
      setIsLoadingVersions(false);
    }
  };

  const loadFabricVersions = async () => {
    setIsLoadingVersions(true);
    setError('');
    try {
      const versions = await invoke<FabricLoaderVersion[]>('get_fabric_loader_versions', {
        minecraftVersion: selectedMinecraftVersion,
      });
      setFabricVersions(versions);
      console.log('Loaded Fabric versions:', versions.length);
    } catch (error) {
      console.error('Error loading Fabric versions:', error);
      setError('Error al cargar versiones de Fabric Loader');
    } finally {
      setIsLoadingVersions(false);
    }
  };

  const handleNext = () => {
    setError('');
    
    if (step === 1 && !name.trim()) {
      setError('El nombre de la instancia no puede estar vacío');
      return;
    }
    
    if (step === 2 && !selectedMinecraftVersion) {
      setError('Debes seleccionar una versión de Minecraft');
      return;
    }
    
    if (step === 3 && !selectedFabricVersion) {
      setError('Debes seleccionar una versión de Fabric Loader');
      return;
    }
    
    if (step < 4) {
      setStep(step + 1);
    }
  };

  const handleBack = () => {
    setError('');
    if (step > 1) {
      setStep(step - 1);
    }
  };

  const handleCreate = async () => {
    setIsCreating(true);
    setError('');
    
    try {
      const instance = await invoke<LocalInstance>('create_local_instance', {
        name,
        minecraftVersion: selectedMinecraftVersion,
        fabricVersion: selectedFabricVersion,
      });
      
      console.log('Instance created successfully:', instance);
      onInstanceCreated(instance);
      onClose();
    } catch (error) {
      console.error('Error creating instance:', error);
      setError(`Error al crear instancia: ${error}`);
    } finally {
      setIsCreating(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div 
      className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-fade-in"
      onClick={(e) => {
        if (e.target === e.currentTarget && !isCreating) {
          onClose();
        }
      }}
    >
      <div 
        className="glass-card rounded-3xl border border-white/10 p-8 max-w-2xl w-full shadow-2xl animate-slide-up"
        style={{
          background: 'rgba(10, 10, 10, 0.95)',
          backdropFilter: 'blur(24px)',
          WebkitBackdropFilter: 'blur(24px)',
        }}
      >
        {/* Header */}
        <div className="mb-6">
          <h2 className="text-3xl font-bold text-white mb-2">
            Nueva Instancia Local
          </h2>
          <p className="text-white/60">
            Crea una instancia personalizada para pruebas (Paso {step} de 4)
          </p>
        </div>

        {/* Progress bar */}
        <div className="mb-8">
          <div className="flex items-center gap-2">
            {[1, 2, 3, 4].map((s) => (
              <div
                key={s}
                className={`flex-1 h-2 rounded-full transition-all duration-300 ${
                  s <= step ? 'bg-[#00ffff] neon-glow-cyan' : 'bg-white/10'
                }`}
              />
            ))}
          </div>
        </div>

        {/* Content */}
        <div className="min-h-[300px]">
          {/* Step 1: Name */}
          {step === 1 && (
            <div className="space-y-6 animate-fade-in-up">
              <div>
                <label className="block text-white mb-2 font-medium">
                  Nombre de la instancia
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Mi Instancia de Pruebas"
                  className="w-full px-4 py-3 rounded-xl bg-white/5 border border-white/10 text-white placeholder-white/40 focus:outline-none focus:border-[#00ffff] focus:ring-2 focus:ring-[#00ffff]/20 transition-all"
                  autoFocus
                />
              </div>
              
              {generatedId && (
                <div className="mt-4 p-4 rounded-xl bg-white/5 border border-white/10">
                  <p className="text-white/60 text-sm mb-1">ID generado:</p>
                  <p className="text-white font-mono">{generatedId}</p>
                </div>
              )}
            </div>
          )}

          {/* Step 2: Minecraft Version */}
          {step === 2 && (
            <div className="space-y-6 animate-fade-in-up">
              <div>
                <label className="block text-white mb-2 font-medium">
                  Versión de Minecraft
                </label>
                {isLoadingVersions ? (
                  <div className="flex items-center justify-center py-12">
                    <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-[#00ffff]"></div>
                  </div>
                ) : (
                  <div className="max-h-[300px] overflow-y-auto space-y-2 pr-2 custom-scrollbar">
                    {minecraftVersions.map((version) => (
                      <button
                        key={version.id}
                        onClick={() => setSelectedMinecraftVersion(version.id)}
                        className={`w-full px-4 py-3 rounded-xl text-left transition-all duration-200 ${
                          selectedMinecraftVersion === version.id
                            ? 'bg-[#00ffff]/20 border-2 border-[#00ffff] text-white neon-glow-cyan'
                            : 'bg-white/5 border border-white/10 text-white/80 hover:bg-white/10 hover:border-white/20'
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-medium">{version.id}</span>
                          <span className="text-sm text-white/60">
                            {new Date(version.releaseTime).toLocaleDateString()}
                          </span>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Step 3: Fabric Version */}
          {step === 3 && (
            <div className="space-y-6 animate-fade-in-up">
              <div>
                <label className="block text-white mb-2 font-medium">
                  Versión de Fabric Loader
                </label>
                <p className="text-white/60 text-sm mb-4">
                  Para Minecraft {selectedMinecraftVersion}
                </p>
                {isLoadingVersions ? (
                  <div className="flex items-center justify-center py-12">
                    <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-[#00ffff]"></div>
                  </div>
                ) : (
                  <div className="max-h-[300px] overflow-y-auto space-y-2 pr-2 custom-scrollbar">
                    {fabricVersions.map((version) => (
                      <button
                        key={version.loader.version}
                        onClick={() => setSelectedFabricVersion(version.loader.version)}
                        className={`w-full px-4 py-3 rounded-xl text-left transition-all duration-200 ${
                          selectedFabricVersion === version.loader.version
                            ? 'bg-[#00ffff]/20 border-2 border-[#00ffff] text-white neon-glow-cyan'
                            : 'bg-white/5 border border-white/10 text-white/80 hover:bg-white/10 hover:border-white/20'
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-medium">{version.loader.version}</span>
                          {version.loader.stable && (
                            <span className="px-2 py-1 rounded-lg bg-green-500/20 text-green-300 text-xs">
                              Estable
                            </span>
                          )}
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Step 4: Summary */}
          {step === 4 && (
            <div className="space-y-6 animate-fade-in-up">
              <div className="space-y-4">
                <h3 className="text-xl font-bold text-white mb-4">Resumen de la instancia</h3>
                
                <div className="p-4 rounded-xl bg-white/5 border border-white/10">
                  <p className="text-white/60 text-sm mb-1">Nombre:</p>
                  <p className="text-white font-medium">{name}</p>
                </div>
                
                <div className="p-4 rounded-xl bg-white/5 border border-white/10">
                  <p className="text-white/60 text-sm mb-1">Versión de Minecraft:</p>
                  <p className="text-white font-medium">{selectedMinecraftVersion}</p>
                </div>
                
                <div className="p-4 rounded-xl bg-white/5 border border-white/10">
                  <p className="text-white/60 text-sm mb-1">Versión de Fabric Loader:</p>
                  <p className="text-white font-medium">{selectedFabricVersion}</p>
                </div>

                <div className="mt-6 p-4 rounded-xl bg-yellow-500/10 border border-yellow-500/20">
                  <p className="text-yellow-300 text-sm">
                    ⚠️ La creación puede tardar varios minutos mientras se descargan los archivos necesarios.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Error message */}
          {error && (
            <div className="mt-4 p-4 rounded-xl bg-red-500/10 border border-red-500/20">
              <p className="text-red-300 text-sm">{error}</p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="mt-8 flex items-center justify-between gap-4">
          <button
            onClick={onClose}
            disabled={isCreating}
            className="px-6 py-3 rounded-xl bg-white/5 border border-white/10 text-white hover:bg-white/10 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancelar
          </button>
          
          <div className="flex gap-4">
            {step > 1 && step < 4 && (
              <button
                onClick={handleBack}
                disabled={isCreating || isLoadingVersions}
                className="px-6 py-3 rounded-xl bg-white/5 border border-white/10 text-white hover:bg-white/10 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Atrás
              </button>
            )}
            
            {step < 4 ? (
              <button
                onClick={handleNext}
                disabled={isLoadingVersions || (step === 1 && !name.trim()) || (step === 2 && !selectedMinecraftVersion) || (step === 3 && !selectedFabricVersion)}
                className="px-6 py-3 rounded-xl bg-[#00ffff]/20 border-2 border-[#00ffff] text-white hover:bg-[#00ffff]/30 transition-all duration-200 neon-glow-cyan-hover disabled:opacity-50 disabled:cursor-not-allowed font-medium"
              >
                Siguiente
              </button>
            ) : (
              <button
                onClick={handleCreate}
                disabled={isCreating}
                className="px-8 py-3 rounded-xl bg-[#00ffff]/20 border-2 border-[#00ffff] text-white hover:bg-[#00ffff]/30 transition-all duration-200 neon-glow-cyan-hover disabled:opacity-50 disabled:cursor-not-allowed font-bold flex items-center gap-2"
              >
                {isCreating ? (
                  <>
                    <div className="animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-white"></div>
                    Creando...
                  </>
                ) : (
                  'Crear Instancia'
                )}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default CreateLocalInstanceModal;


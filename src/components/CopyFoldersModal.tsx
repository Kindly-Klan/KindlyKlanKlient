import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LocalInstance } from '@/types/local-instances';

interface CopyFoldersModalProps {
  isOpen: boolean;
  onClose: () => void;
  targetInstanceId: string;
  localInstances: LocalInstance[];
  onFoldersCopied?: () => void;
  addToast?: (message: string, type?: 'success' | 'error' | 'info', duration?: number) => void;
}

const CopyFoldersModal: React.FC<CopyFoldersModalProps> = ({
  isOpen,
  onClose,
  targetInstanceId,
  localInstances,
  onFoldersCopied,
  addToast,
}) => {
  const [sourceInstanceId, setSourceInstanceId] = useState<string>('');
  const [selectedFolders, setSelectedFolders] = useState<Set<string>>(new Set());
  const [isCopying, setIsCopying] = useState(false);
  const [copyProgress, setCopyProgress] = useState<Record<string, number>>({});

  const availableFolders = [
    { name: 'mods', icon: 'M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4' },
    { name: 'config', icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z' },
    { name: 'resourcepacks', icon: 'M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01' },
    { name: 'shaderpacks', icon: 'M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01' },
    { name: 'saves', icon: 'M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z' },
    { name: 'screenshots', icon: 'M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z' },
  ];

  useEffect(() => {
    if (!isOpen) {
      setSourceInstanceId('');
      setSelectedFolders(new Set());
      setIsCopying(false);
      setCopyProgress({});
    }
  }, [isOpen]);

  // Escuchar eventos de progreso
  useEffect(() => {
    if (!isOpen) return;

    let unlisten: (() => void) | undefined;

    (async () => {
      unlisten = await listen('copy-folders-progress', (event: any) => {
        const data = event.payload;
        if (data.target_instance_id === targetInstanceId) {
          if (data.status === 'completed') {
            setCopyProgress(prev => ({ ...prev, [data.folder]: 100 }));
            setTimeout(() => {
              setCopyProgress(prev => {
                const newProgress = { ...prev };
                delete newProgress[data.folder];
                return newProgress;
              });
            }, 2000);
          } else if (data.status === 'copying') {
            setCopyProgress(prev => ({ ...prev, [data.folder]: data.percentage || 0 }));
          }
        }
      });
    })();

    return () => {
      if (unlisten) {
        try {
          unlisten();
        } catch {}
      }
    };
  }, [isOpen, targetInstanceId]);

  const handleFolderToggle = (folder: string) => {
    setSelectedFolders(prev => {
      const newSet = new Set(prev);
      if (newSet.has(folder)) {
        newSet.delete(folder);
      } else {
        newSet.add(folder);
      }
      return newSet;
    });
  };

  const handleCopy = async () => {
    if (!sourceInstanceId || selectedFolders.size === 0) {
      if (addToast) {
        addToast('Por favor selecciona una instancia origen y al menos una carpeta', 'error');
      } else {
        alert('Por favor selecciona una instancia origen y al menos una carpeta');
      }
      return;
    }

    setIsCopying(true);
    try {
      await invoke('copy_instance_folders', {
        sourceInstanceId: sourceInstanceId,
        targetInstanceId: targetInstanceId,
        folders: Array.from(selectedFolders),
      });

      if (onFoldersCopied) {
        onFoldersCopied();
      }
      onClose();
    } catch (error) {
      console.error('Error copying folders:', error);
      if (addToast) {
        addToast(`Error al copiar carpetas: ${error}`, 'error');
      } else {
        alert(`Error al copiar carpetas: ${error}`);
      }
    } finally {
      setIsCopying(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div 
      className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-300"
      onClick={onClose}
    >
      <div
        className="glass-card rounded-3xl border border-white/10 p-8 max-w-2xl w-full shadow-2xl animate-in zoom-in-95 duration-300"
        style={{
          background: 'rgba(10, 10, 10, 0.95)',
          backdropFilter: 'blur(24px)',
          WebkitBackdropFilter: 'blur(24px)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-[#ffff00]/10 border border-[#ffff00]/20">
              <svg className="w-6 h-6 text-[#ffff00]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
            </div>
            <h2 className="text-3xl font-bold text-white">
              Copiar Carpetas entre Instancias
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl bg-white/5 border border-white/10 text-white hover:bg-white/10 hover:scale-110 transition-all duration-200 cursor-pointer"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Source instance selection */}
        <div className="mb-6">
          <label className="block text-white/80 mb-2 flex items-center gap-2">
            <svg className="w-4 h-4 text-[#ffff00]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
            Instancia Origen:
          </label>
          <div className="relative">
            <div className="absolute left-3 top-1/2 -translate-y-1/2 text-white/40">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <select
              value={sourceInstanceId}
              onChange={(e) => setSourceInstanceId(e.target.value)}
              className="w-full pl-10 p-3 rounded-xl bg-white/5 border border-white/10 text-white hover:border-[#ffff00]/30 focus:border-[#ffff00]/50 focus:ring-2 focus:ring-[#ffff00]/20 transition-all duration-200 cursor-pointer appearance-none"
              disabled={isCopying}
            >
              <option value="">Selecciona una instancia...</option>
              {localInstances
                .filter(inst => inst.id !== targetInstanceId)
                .map(inst => (
                  <option key={inst.id} value={inst.id} className="bg-gray-900">
                    {inst.name} (MC {inst.minecraft_version})
                  </option>
                ))}
            </select>
            <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
              <svg className="w-5 h-5 text-white/40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </div>
          </div>
        </div>

        {/* Folders selection */}
        <div className="mb-6">
          <label className="block text-white/80 mb-3 flex items-center gap-2">
            <svg className="w-4 h-4 text-[#ffff00]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
            Carpetas a Copiar:
          </label>
          <div className="grid grid-cols-2 gap-3">
            {availableFolders.map((folder, index) => {
              const isSelected = selectedFolders.has(folder.name);
              const progress = copyProgress[folder.name];
              const isCopyingFolder = isCopying && progress !== undefined;

              return (
                <label
                  key={folder.name}
                  className={`p-4 rounded-xl border cursor-pointer transition-all duration-200 group relative overflow-hidden animate-in fade-in slide-in-from-bottom-4 ${
                    isSelected
                      ? 'bg-[#ffff00]/20 border-[#ffff00]/50 text-[#ffff00] shadow-lg shadow-[#ffff00]/20'
                      : 'bg-white/5 border-white/10 text-white hover:bg-white/10 hover:border-[#ffff00]/30'
                  }`}
                  style={{ animationDelay: `${index * 50}ms` }}
                >
                  <div className="flex items-center gap-3 relative z-10">
                    <div className={`p-2 rounded-lg transition-colors ${
                      isSelected ? 'bg-[#ffff00]/20' : 'bg-white/5'
                    }`}>
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={folder.icon} />
                      </svg>
                    </div>
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={isSelected}
                          onChange={() => handleFolderToggle(folder.name)}
                          disabled={isCopying}
                          className="w-4 h-4 rounded cursor-pointer accent-[#ffff00]"
                        />
                        <span className="font-medium capitalize">{folder.name}</span>
                      </div>
                    </div>
                    {isSelected && (
                      <svg className="w-5 h-5 text-[#ffff00] animate-in zoom-in" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    )}
                  </div>
                  {isCopyingFolder && (
                    <div className="mt-3 w-full bg-white/5 rounded-full h-1.5 overflow-hidden relative z-10">
                      <div
                        className="bg-gradient-to-r from-[#ffff00] to-[#ffff00]/60 h-full transition-all duration-300 shadow-lg shadow-[#ffff00]/50"
                        style={{ width: `${progress}%` }}
                      />
                    </div>
                  )}
                  {isSelected && (
                    <div className="absolute inset-0 bg-gradient-to-br from-[#ffff00]/10 to-transparent pointer-events-none" />
                  )}
                </label>
              );
            })}
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3">
          <button
            onClick={onClose}
            disabled={isCopying}
            className="px-6 py-2.5 rounded-xl bg-white/5 border border-white/10 text-white hover:bg-white/10 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200 font-medium cursor-pointer hover:scale-105 active:scale-95 flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
            Cancelar
          </button>
          <button
            onClick={handleCopy}
            disabled={isCopying || !sourceInstanceId || selectedFolders.size === 0}
            className="px-6 py-2.5 rounded-xl bg-[#ffff00]/20 border-2 border-[#ffff00]/30 text-[#ffff00] hover:bg-[#ffff00]/30 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200 font-medium cursor-pointer hover:scale-105 active:scale-95 flex items-center gap-2 relative overflow-hidden min-w-[160px]"
          >
            {isCopying ? (
              <>
                <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                <span>Copiando...</span>
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                <span>Copiar Carpetas</span>
              </>
            )}
            {isCopying && Object.keys(copyProgress).length > 0 && (
              <div className="absolute bottom-0 left-0 right-0 h-1 bg-[#ffff00]/20">
                <div 
                  className="h-full bg-[#ffff00] transition-all duration-300"
                  style={{ 
                    width: `${Object.values(copyProgress).reduce((a, b) => a + b, 0) / (Object.keys(copyProgress).length * 100) * 100}%` 
                  }}
                />
              </div>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

export default CopyFoldersModal;

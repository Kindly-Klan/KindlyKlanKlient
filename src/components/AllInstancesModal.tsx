import React from 'react';
import type { LocalInstance } from '@/types/local-instances';
import { convertFileSrc } from '@tauri-apps/api/core';

interface Instance {
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
  is_local?: boolean;
}

interface AllInstancesModalProps {
  isOpen: boolean;
  onClose: () => void;
  localInstances: LocalInstance[];
  remoteInstances?: Instance[];
  selectedInstance: string | null;
  onInstanceSelect: (instanceId: string) => void;
  distributionBaseUrl?: string;
}

const AllInstancesModal: React.FC<AllInstancesModalProps> = ({
  isOpen,
  onClose,
  localInstances,
  remoteInstances = [],
  selectedInstance,
  onInstanceSelect,
  distributionBaseUrl = '',
}) => {
  if (!isOpen) return null;

  type CombinedInstance = (LocalInstance & { is_local: true }) | (Instance & { is_local: false });

  const allInstances: CombinedInstance[] = [
    ...localInstances.map(inst => ({ ...inst, is_local: true as const })),
    ...remoteInstances.map(inst => ({ ...inst, is_local: false as const }))
  ];

  return (
    <div 
      className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-300"
      onClick={onClose}
    >
      <div
        className="glass-card rounded-2xl border border-white/10 p-6 max-w-5xl w-full max-h-[90vh] overflow-hidden flex flex-col shadow-2xl animate-in zoom-in-95 duration-300"
        style={{
          background: 'rgba(10, 10, 10, 0.95)',
          backdropFilter: 'blur(24px)',
          WebkitBackdropFilter: 'blur(24px)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-[#FFD700]/10 border border-[#FFD700]/20">
              <svg className="w-5 h-5 text-[#FFD700]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
              </svg>
            </div>
            <h2 className="text-xl font-semibold text-white">
              Todas las Instancias ({allInstances.length})
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg bg-white/5 border border-white/10 text-white hover:bg-white/10 transition-all duration-200 cursor-pointer"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto custom-scrollbar pr-2">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {allInstances.map((instance) => {
              const isSelected = selectedInstance === instance.id;
              const iconUrl = instance.is_local 
                ? (instance.background ? convertFileSrc(instance.background) : undefined)
                : ('icon' in instance && instance.icon ? `${distributionBaseUrl}/${instance.icon}` : undefined);
              
              const loaderType = instance.is_local 
                ? instance.mod_loader?.type 
                : instance.mod_loader?.type;
              
              const loaderVersion = instance.is_local
                ? instance.mod_loader?.version || instance.fabric_version
                : instance.mod_loader?.version;

              return (
                <div
                  key={instance.id}
                  onClick={() => {
                    onInstanceSelect(instance.id);
                    onClose();
                  }}
                  className={`relative group cursor-pointer rounded-xl overflow-visible transition-all duration-200 ${
                    isSelected 
                      ? 'ring-2 ring-[#FFD700]' 
                      : 'ring-1 ring-white/10 hover:ring-[#FFD700]/50'
                  }`}
                >
                  {iconUrl ? (
                    <div className="aspect-square relative rounded-t-xl overflow-hidden">
                      <img
                        src={iconUrl}
                        alt={instance.name}
                        className="w-full h-full object-cover"
                      />
                      <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent" />
                    </div>
                  ) : (
                    <div className="aspect-square bg-gradient-to-br from-[#FFD700]/20 to-[#FF8C00]/20 flex items-center justify-center rounded-t-xl">
                      <svg className="w-12 h-12 text-[#FFD700]/50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                    </div>
                  )}
                  
                  <div className="absolute bottom-0 left-0 right-0 p-3 bg-gradient-to-t from-black/90 to-transparent rounded-b-xl">
                    <div className="flex items-center justify-between mb-1 gap-2">
                      <h3 className="text-white font-semibold text-sm truncate">
                        {instance.name}
                      </h3>
                      {instance.is_local && (
                        <span className="px-1.5 py-0.5 rounded bg-[#FFD700]/20 text-[#FFD700] text-xs font-medium flex-shrink-0">
                          Local
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 text-white/70 text-xs">
                      <span>MC {instance.minecraft_version}</span>
                      {loaderType && loaderVersion && (
                        <>
                          <span>•</span>
                          <span className="capitalize truncate">{loaderType} {loaderVersion}</span>
                        </>
                      )}
                    </div>
                  </div>
                  
                  {isSelected && (
                    <div className="absolute top-1.5 right-1.5 p-1.5 rounded-full bg-[#FFD700] text-black">
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};

export default AllInstancesModal;

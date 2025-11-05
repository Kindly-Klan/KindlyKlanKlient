import React from 'react';
import Tooltip from '@/components/ui/Tooltip';

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
}

interface AuthSession {
  access_token: string;
  username: string;
  uuid: string;
  user_type: string;
  expires_at?: number;
  refresh_token?: string;
}

interface SidebarProps {
  instances: Instance[];
  selectedInstance: string | null;
  onInstanceSelect: (instanceId: string) => void;
  handleSettingsToggle: () => void;
  handleSkinToggle: () => void;
  distributionBaseUrl: string;
  currentUser?: AuthSession | null;
  settingsOpen?: boolean;
}

const Sidebar: React.FC<SidebarProps> = ({
  instances,
  selectedInstance,
  onInstanceSelect,
  handleSettingsToggle,
  //handleSkinToggle,
  distributionBaseUrl,
  currentUser,
  settingsOpen = false
}) => {
  return (
    <>

      <div className="fixed left-0 top-0 h-full w-20 glass border-r border-white/10 z-40">
        <div className="p-2">
          <div className="space-y-2">
            {instances.map((instance) => (
              <Tooltip key={instance.id} content={instance.name} side="right">
                <div
                  onClick={() => onInstanceSelect(instance.id)}
                  className={`w-full aspect-square cursor-pointer transition-all duration-300 ease-out relative select-none ${
                    selectedInstance === instance.id
                      ? 'scale-105'
                      : 'hover:scale-105'
                  }`}
                >
                  <div className={`w-full h-full rounded-2xl overflow-hidden transition-all duration-300 ease-out ${
                    selectedInstance === instance.id
                      ? 'ring-2 ring-[#00ffff] ring-offset-2 ring-offset-black/50 shadow-lg neon-glow-cyan'
                      : 'ring-1 ring-white/10 hover:ring-white/20'
                  }`}>
                    {instance.icon ? (
                      <img
                        src={`${distributionBaseUrl}/${instance.icon}`}
                        alt={instance.name}
                        className="w-full h-full object-cover"
                        style={{ filter: selectedInstance === instance.id ? 'none' : 'none' }}
                      />
                    ) : (
                      <div className={`w-full h-full bg-gradient-to-br from-[#00ffff]/20 to-[#ff00ff]/20 flex items-center justify-center ${
                        selectedInstance === instance.id ? '' : ''
                      }`}>
                        <span className="text-white font-bold text-xl">
                          {instance.name.charAt(0).toUpperCase()}
                        </span>
                      </div>
                    )}
                  </div>
                </div>
              </Tooltip>
            ))}
          </div>

          {/* Settings Button at bottom - Only Icon */}
          <div className="absolute bottom-2 left-2 right-2">
            {/* Skin Management Button - Just above settings */}
            {currentUser && (
              <div className="mb-2">
                <div
                  onClick={() => {}}
                  className="relative group"
                >
                  <div className="w-full aspect-square rounded-2xl overflow-hidden ring-1 ring-white/10 cursor-not-allowed transition-all duration-300 ease-out opacity-50 select-none">
                    <img
                      src={`https://crafatar.com/avatars/${currentUser.uuid}?size=64&overlay=true`}
                      alt={`${currentUser.username}'s avatar`}
                      className="w-full h-full object-cover"
                      onError={(e) => {
                        // Fallback to default avatar if Crafatar fails
                        e.currentTarget.src = `data:image/svg+xml;base64,${btoa(`
                          <svg width="64" height="64" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
                            <rect width="64" height="64" rx="12" fill="#4A90E2"/>
                            <text x="32" y="40" font-family="Arial, sans-serif" font-size="28" font-weight="bold" text-anchor="middle" fill="white">
                              ${currentUser.username.charAt(0).toUpperCase()}
                            </text>
                          </svg>
                        `)}`;
                      }}
                    />
                  </div>

                  {/* Tooltip on hover */}
                  <div className="absolute left-full ml-2 top-1/2 transform -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none z-50">
                    <div className="glass-card text-white text-xs px-2 py-1 rounded-xl whitespace-nowrap shadow-lg border border-white/10">Cambiar Skin (Próximamente)</div>
                  </div>
                </div>
              </div>
            )}

            <div className="relative group flex items-center justify-center">
              <svg 
                onClick={() => handleSettingsToggle()}
                className={`w-12 h-12 cursor-pointer transition-[transform,color,filter] duration-500 ease-in-out ${
                  settingsOpen 
                    ? 'text-white' 
                    : 'text-white/70 hover:text-white'
                } ${settingsOpen ? '' : 'hover:scale-110'}`}
                style={{
                  transformOrigin: 'center center',
                  filter: settingsOpen 
                    ? 'drop-shadow(0 0 8px rgba(0, 255, 255, 0.6)) drop-shadow(0 0 16px rgba(0, 255, 255, 0.4))' 
                    : 'drop-shadow(0 0 0 rgba(0, 255, 255, 0))',
                  transition: 'transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1), color 0.3s ease-out, filter 0.3s ease-out',
                }}
                onMouseEnter={(e) => {
                  if (!settingsOpen) {
                    e.currentTarget.style.filter = 'drop-shadow(0 0 8px rgba(0, 255, 255, 0.6)) drop-shadow(0 0 16px rgba(0, 255, 255, 0.4))';
                    e.currentTarget.style.transform = 'scale(1.1) rotate(15deg)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (!settingsOpen) {
                    e.currentTarget.style.filter = 'drop-shadow(0 0 0 rgba(0, 255, 255, 0))';
                    e.currentTarget.style.transform = 'scale(1) rotate(0deg)';
                  } else {
                    e.currentTarget.style.filter = 'drop-shadow(0 0 8px rgba(0, 255, 255, 0.6)) drop-shadow(0 0 16px rgba(0, 255, 255, 0.4))';
                    e.currentTarget.style.transform = 'scale(1) rotate(0deg)';
                  }
                }}
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94 1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              <div className="pointer-events-none absolute left-full ml-2 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 z-50">
                <div className="glass-card text-white text-xs px-2 py-1 rounded-xl whitespace-nowrap shadow-lg border border-white/10">Ajustes</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export default Sidebar;

import React from 'react';
import { Button } from '@/components/ui/button';
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
}

const Sidebar: React.FC<SidebarProps> = ({
  instances,
  selectedInstance,
  onInstanceSelect,
  handleSettingsToggle,
  handleSkinToggle,
  distributionBaseUrl,
  currentUser
}) => {
  return (
    <>

      <div className="fixed left-0 top-0 h-full w-20 bg-black/30 backdrop-blur-md border-r border-white/10 z-40">
        <div className="p-2">
          <div className="space-y-3">
            {instances.map((instance) => (
              <Tooltip key={instance.id} content={instance.name} side="right">
                <div
                  onClick={() => onInstanceSelect(instance.id)}
                  className={`p-2 rounded-xl cursor-pointer transition-all duration-200 border relative select-none ${
                    selectedInstance === instance.id
                      ? 'bg-white/20 border-white/30 shadow-lg'
                      : 'bg-black/20 border-white/10 hover:bg-black/30 hover:border-white/20'
                  }`}
                >
                  <div className="flex items-center justify-center">
                    {instance.icon ? (
                      <img
                        src={`${distributionBaseUrl}/${instance.icon}`}
                        alt={instance.name}
                        className="w-16 h-16 rounded-lg object-contain"
                      />
                    ) : (
                      <div className="w-16 h-16 rounded-lg bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
                        <span className="text-white font-bold text-lg">
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
                  onClick={() => handleSkinToggle()}
                  className="relative group"
                >
                  <div className="p-2 rounded-xl cursor-pointer transition-all duration-200 border relative bg-black/20 border-white/10 hover:bg-black/30 hover:border-white/20 select-none">
                    <div className="flex items-center justify-center">
                      <img
                        src={`https://crafatar.com/avatars/${currentUser.uuid}?size=32&overlay=true`}
                        alt={`${currentUser.username}'s avatar`}
                        className="w-12 h-12 rounded-lg border border-white/20"
                        onError={(e) => {
                          // Fallback to default avatar if Crafatar fails
                          e.currentTarget.src = `data:image/svg+xml;base64,${btoa(`
                            <svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
                              <rect width="32" height="32" rx="8" fill="#4A90E2"/>
                              <text x="16" y="22" font-family="Arial, sans-serif" font-size="14" font-weight="bold" text-anchor="middle" fill="white">
                                ${currentUser.username.charAt(0).toUpperCase()}
                              </text>
                            </svg>
                          `)}`;
                        }}
                      />
                    </div>
                  </div>

                  {/* Tooltip on hover */}
                  <div className="absolute left-full ml-2 top-1/2 transform -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
                    <div className="bg-black text-white text-xs px-2 py-1 rounded whitespace-nowrap">
                      Cambiar Skin
                    </div>
                  </div>
                </div>
              </div>
            )}

            <div className="relative group">
              <Button
                onClick={() => handleSettingsToggle()}
                variant="ghost"
                size="sm"
                className="w-full h-10 justify-center text-white/70 hover:text-white hover:bg-black/30 transition-all duration-300 rounded-lg opacity-100 cursor-pointer"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94 1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </Button>
              <div className="pointer-events-none absolute left-full ml-2 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                <div className="bg-black text-white text-xs px-2 py-1 rounded whitespace-nowrap shadow-lg border border-white/10">Ajustes</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export default Sidebar;

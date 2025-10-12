import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface NoAccessScreenProps {
  onLogout: () => void;
  username?: string;
}

const NoAccessScreen: React.FC<NoAccessScreenProps> = ({ onLogout, username }) => {
  const [isVisible, setIsVisible] = useState(false);
  const [command, setCommand] = useState('');

  useEffect(() => {
    setIsVisible(true);
  }, []);

  const handleKeyPress = async (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (command.toLowerCase().trim() === 'logout') {
        onLogout();
      } else if (command.toLowerCase().trim() === 'discord') {
        try {
          await invoke('open_url', { url: 'https://discord.kindlyklan.com' });
          setCommand('');
        } catch (error) {
          console.error('Error opening Discord:', error);
          setCommand('');
        }
      } else if (command.trim() !== '') {
        setCommand('');
      }
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black">
      {/* Background */}
      <div className="absolute inset-0 z-0">
        <div
          className="w-full h-full"
          style={{
            background: 'linear-gradient(135deg, #0a0a0a 0%, #1a1a1a 50%, #0a0a0a 100%)'
          }}
        />
      </div>

      {/* Content */}
      <div className="relative z-20 h-full flex items-center justify-center p-4">
        <div className={`w-full max-w-2xl transition-all duration-700 ${
          isVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'
        }`}>
          
          {/* Terminal Window */}
          <div className="bg-black text-white p-6 rounded-lg w-full font-mono border border-gray-700 shadow-2xl">
            {/* Terminal Header */}
            <div className="flex justify-between items-center mb-4">
              <div className="flex space-x-2">
                <div className="w-3 h-3 rounded-full bg-red-500"></div>
                <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                <div className="w-3 h-3 rounded-full bg-green-500"></div>
              </div>
              <p className="text-sm text-gray-400">kindlyklan@klient ~</p>
            </div>

            {/* Terminal Content */}
            <div className="space-y-2 text-sm">
              <p className="text-green-400">$ ./launcher --whitelist {username || 'usuario'}</p>
              <p className="text-white">Verificando permisos de whitelist...</p>
              <p className="text-red-400">❌ Acceso denegado para usuario: {username || 'desconocido'}</p>
              <p className="text-white">Para solicitar acceso, contacta a un administrador en Discord.</p>
              <p className="text-white">Escribe 'discord' para abrir el servidor de Discord</p>
              <p className="text-white">Escribe 'logout' para cerrar sesión</p>
              
              {/* Command Input */}
              <div className="flex items-center mt-4">
                <span className="text-green-400">$ </span>
                <input
                  type="text"
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  onKeyPress={handleKeyPress}
                  className="bg-transparent text-white outline-none flex-1 ml-2 font-mono"
                  placeholder=""
                  autoFocus
                />
              </div>
            </div>
          </div>          
        </div>
      </div>
    </div>
  );
};

export default NoAccessScreen;

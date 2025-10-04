import React from 'react';
import { Button } from '@/components/ui/button';

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
  distributionUrl: string;
  onReloadDistribution: () => void;
}

const SettingsPanel: React.FC<SettingsPanelProps> = ({
  isOpen,
  onClose,
  distributionUrl,
  onReloadDistribution
}) => {
  const handleReload = () => {
    console.log('SettingsPanel: Reloading distribution');
    onReloadDistribution();
    onClose();  
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-[10000] flex items-center justify-center">
      <div className="bg-gray-900/90 backdrop-blur-md rounded-lg p-6 border border-white/20 w-full max-w-md mx-4">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-bold text-white">Configuración</h2>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="text-white/70 hover:text-white"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </Button>
        </div>

        <div className="space-y-4">
         
          <div className="bg-black/20 rounded-lg p-3 border border-white/10">
            <h3 className="text-sm font-medium text-white mb-2">Distribución</h3>
            <p className="text-xs text-gray-300 mb-1">
              URL: {distributionUrl}
            </p>
            <p className="text-xs text-gray-400">
              Esta URL está configurada permanentemente para garantizar la seguridad y consistencia de la aplicación.
            </p>
          </div>

            
          <div className="flex space-x-3 pt-4">
            <Button
              onClick={handleReload}
              className="flex-1 bg-blue-600 hover:bg-blue-700 text-white"
            >
              Recargar Distribución
            </Button>
            <Button
              onClick={onClose}
              variant="outline"
              className="flex-1 border-white/20 text-white hover:bg-white/10"
            >
              Cerrar
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsPanel;

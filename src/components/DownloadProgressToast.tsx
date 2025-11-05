import React, { useEffect, useState } from 'react';

interface DownloadProgressToastProps {
  message: string;
  percentage: number;
  onClose?: () => void;
}

const DownloadProgressToast: React.FC<DownloadProgressToastProps> = ({
  message,
  percentage,
  onClose
}) => {
  const [isVisible, setIsVisible] = useState(false);
  useEffect(() => { setTimeout(() => setIsVisible(true), 50); }, []); // mimic normal toast enter

  return (
    <div
      className={`
        z-[10000] max-w-sm w-full
        p-4 rounded-2xl border-2 shadow-2xl
        transition-all duration-300 transform
        ${isVisible ? 'translate-x-0 opacity-100' : 'translate-x-full opacity-0'}
        text-blue-200 border-blue-400/60
      `}
      style={{
        background: 'linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.6)'
      }}
    >
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium flex-1 mr-3">{message}</p>
        {onClose && (
          <button
            onClick={onClose}
            className="text-white/70 hover:text-white transition-colors duration-200 flex-shrink-0"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>
      <div className="w-full h-2 bg-blue-500/30 rounded-full overflow-hidden mt-3" style={{
        background: 'rgba(59, 130, 246, 0.2)',
        boxShadow: 'inset 0 2px 4px rgba(0, 0, 0, 0.3)'
      }}>
        <div className="h-2 bg-blue-300 rounded-full transition-all duration-300 progress-shimmer" style={{ 
          width: `${percentage}%`,
          boxShadow: '0 0 8px rgba(147, 197, 253, 0.6)'
        }} />
      </div>
    </div>
  );
};

export default DownloadProgressToast;

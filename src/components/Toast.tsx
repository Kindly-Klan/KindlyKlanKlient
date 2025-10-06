import React, { useEffect, useState } from 'react';

interface ToastProps {
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
  onClose: () => void;
  duration?: number;
}

const Toast: React.FC<ToastProps> = ({ message, type, onClose, duration = 5000 }) => {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    setTimeout(() => setIsVisible(true), 100);
    const timer = setTimeout(() => { handleClose(); }, duration);
    return () => clearTimeout(timer);
  }, [duration]);

  const handleClose = () => {
    setIsVisible(false);
    setTimeout(onClose, 300); 
  };

  const getToastStyles = () => {
    switch (type) {
      case 'success':
        return 'bg-green-500/20 text-green-200 border-green-400/30';
      case 'error':
        return 'bg-red-500/20 text-red-200 border-red-400/30';
      case 'info':
      default:
        return 'bg-blue-500/20 text-blue-200 border-blue-400/30';
    }
  };

  return (
    <div
      className={`
        z-[10000] max-w-sm w-full
        p-4 rounded-lg border backdrop-blur-sm shadow-lg bg-opacity-90
        transition-all duration-300 transform
        ${isVisible ? 'translate-x-0 opacity-100' : 'translate-x-full opacity-0'}
        ${getToastStyles()}
      `}
    >
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium flex-1 mr-3">{message}</p>
        <button
          onClick={handleClose}
          className="text-white/70 hover:text-white transition-colors duration-200 flex-shrink-0"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
};

export default Toast;

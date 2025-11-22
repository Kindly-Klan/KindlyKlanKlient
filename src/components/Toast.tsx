import React, { useEffect, useState, useRef } from 'react';

interface ToastProps {
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
  onClose: () => void;
  duration?: number;
}

const Toast: React.FC<ToastProps> = ({ message, type, onClose, duration = 5000 }) => {
  const [isVisible, setIsVisible] = useState(false);
  const [previousMessage, setPreviousMessage] = useState(message);
  const [isChanging, setIsChanging] = useState(false);
  const messageRef = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    setTimeout(() => setIsVisible(true), 100);
    const timer = setTimeout(() => { handleClose(); }, duration);
    return () => clearTimeout(timer);
  }, [duration]);

  useEffect(() => {
    if (message !== previousMessage) {
      setIsChanging(true);
      setTimeout(() => {
        setPreviousMessage(message);
        setIsChanging(false);
      }, 300);
    }
  }, [message, previousMessage]);

  const handleClose = () => {
    setIsVisible(false);
    setTimeout(onClose, 500);
  };

  const getToastStyles = () => {
    switch (type) {
      case 'success':
        return {
          text: 'text-green-200',
          border: 'border-green-400/60',
          background: 'rgba(34, 197, 94, 0.15)',
          backdrop: 'rgba(0, 0, 0, 0.5)'
        };
      case 'error':
        return {
          text: 'text-red-200',
          border: 'border-red-400/60',
          background: 'rgba(239, 68, 68, 0.15)',
          backdrop: 'rgba(0, 0, 0, 0.5)'
        };
      case 'warning':
        return {
          text: 'text-yellow-200',
          border: 'border-yellow-400/60',
          background: 'rgba(234, 179, 8, 0.15)',
          backdrop: 'rgba(0, 0, 0, 0.5)'
        };
      case 'info':
      default:
        return {
          text: 'text-cyan-200',
          border: 'border-cyan-400/60',
          background: 'rgba(34, 211, 238, 0.15)',
          backdrop: 'rgba(0, 0, 0, 0.5)'
        };
    }
  };

  const styles = getToastStyles();

  return (
    <div
      className={`
        z-[10000] max-w-sm w-full
        p-4 rounded-2xl border-2 shadow-2xl
        transition-all duration-500 ease-out transform overflow-hidden
        ${isVisible ? 'translate-x-0 opacity-100 scale-100' : 'translate-x-full opacity-0 scale-95'}
        ${styles.text} ${styles.border}
      `}
      style={{
        background: `linear-gradient(135deg, ${styles.background} 0%, ${styles.backdrop} 100%)`,
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.6)',
        transition: 'all 0.5s cubic-bezier(0.4, 0, 0.2, 1)'
      }}
    >
      <div className="flex items-center justify-between">
        <p 
          ref={messageRef}
          className={`text-sm font-medium flex-1 mr-3 transition-all duration-300 ${
            isChanging ? 'opacity-50 scale-95' : 'opacity-100 scale-100'
          }`}
        >
          {message}
        </p>
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

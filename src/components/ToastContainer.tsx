import React from 'react';
import Toast from './Toast';

interface ToastItem {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
  duration?: number;
}

interface ToastContainerProps {
  toasts: ToastItem[];
  onRemove: (id: string) => void;
  children?: React.ReactNode;
}

const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, onRemove, children }) => {
  return (
    <div className="fixed bottom-4 right-4 z-[10000] space-y-2">
      {children}
      {toasts.map((toast) => (
        <Toast
          key={toast.id}
          message={toast.message}
          type={toast.type}
          duration={toast.duration}
          onClose={() => onRemove(toast.id)}
        />
      ))}
    </div>
  );
};

export default ToastContainer;

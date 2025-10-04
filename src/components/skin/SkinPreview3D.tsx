import React, { useEffect, useRef, useState } from 'react';
import * as skinview3d from 'skinview3d';

interface SkinPreview3DProps {
  skinUrl?: string;
  className?: string;
  onTextureLoad?: (textureUrl: string) => void;
}

export const SkinPreview3D: React.FC<SkinPreview3DProps> = ({
  skinUrl,
  className = '',
  onTextureLoad
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const skinViewerRef = useRef<skinview3d.SkinViewer | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;

    // Inicializar skinview3d
    const skinViewer = new skinview3d.SkinViewer({
      canvas: canvasRef.current,
      width: 192, // 48 * 4 (para que se vea bien en el tamaño w-48)
      height: 256, // 64 * 4
      skin: skinUrl || undefined,
    });
    skinViewer.globalLight.intensity = 3;
    skinViewer.cameraLight.intensity = 0;
    skinViewer.fov = 40;
    skinViewer.zoom = 1;
    skinViewer.autoRotate = true;
    skinViewerRef.current = skinViewer;

    return () => {
      // Cleanup
      if (skinViewerRef.current) {
        skinViewerRef.current.dispose();
        skinViewerRef.current = null;
      }
    };
  }, []);

  // Cargar textura cuando cambie skinUrl
  useEffect(() => {
    if (!skinViewerRef.current || !skinUrl) return;

    setIsLoading(true);
    setError(null);

    try {
      skinViewerRef.current.loadSkin(skinUrl);
      onTextureLoad?.(skinUrl);
      setIsLoading(false);
    } catch (err) {
      console.error('Error loading skin:', err);
      setError('Error loading skin texture');
      setIsLoading(false);
    }
  }, [skinUrl, onTextureLoad]);

  if (error) {
    return (
      <div className={`relative ${className}`}>
        <div className="w-48 h-64 bg-gradient-to-b from-gray-700 to-gray-900 rounded-lg overflow-hidden border border-gray-600 flex items-center justify-center">
          <div className="text-center text-red-400">
            <svg className="mx-auto h-8 w-8 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
            <p className="text-sm">{error}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`relative ${className}`}>
      <div className="w-48 h-64 bg-gradient-to-b from-gray-700 to-gray-900 rounded-lg overflow-hidden border border-gray-600">
        <canvas
          ref={canvasRef}
          className={`w-full h-full ${isLoading ? 'opacity-50' : ''}`}
          style={{
            imageRendering: 'pixelated',
            cursor: 'grab',
          }}
          onMouseDown={(e) => {
            e.currentTarget.style.cursor = 'grabbing';
          }}
          onMouseUp={(e) => {
            e.currentTarget.style.cursor = 'grab';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.cursor = 'grab';
          }}
        />
      </div>

      {isLoading && (
        <div className="absolute inset-0 bg-black/50 rounded-lg flex items-center justify-center">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white"></div>
        </div>
      )}
    </div>
  );
};

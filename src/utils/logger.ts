import { invoke } from '@tauri-apps/api/core';

type LogLevel = 'info' | 'warn' | 'error' | 'debug';

class FrontendLogger {
  private static instance: FrontendLogger;
  
  private constructor() {
    this.setupGlobalErrorHandlers();
  }
  
  static getInstance(): FrontendLogger {
    if (!FrontendLogger.instance) {
      FrontendLogger.instance = new FrontendLogger();
    }
    return FrontendLogger.instance;
  }
  
  /**
   * Configura los manejadores globales de errores
   */
  private setupGlobalErrorHandlers() {
    // Capturar errores no manejados
    window.addEventListener('error', (event) => {
      const errorObject = event.error ?? { message: event.message };
      void this.error('Unhandled Error', errorObject, 'window.error', {
        stack: event.error?.stack,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      });
    });
    
    // Capturar promesas rechazadas
    window.addEventListener('unhandledrejection', (event) => {
      void this.error('Unhandled Promise Rejection', event.reason, 'unhandledrejection', {
        reason: event.reason,
      });
    });
    
    // Capturar errores de React (si se usa con un Error Boundary)
    // El Error Boundary debe llamar a logger.error() manualmente
  }
  
  /**
   * Registra un mensaje en los logs
   */
  private async log(level: LogLevel, message: string, context?: string, data?: any) {
    // Log en consola (solo en desarrollo)
    const isDev = import.meta.env.DEV;
    if (isDev) {
      const logMethod = level === 'error' ? console.error : level === 'warn' ? console.warn : console.log;
      logMethod(`[${level.toUpperCase()}]${context ? ` [${context}]` : ''} ${message}`, data || '');
    }
    
    // Construir el mensaje completo con datos adicionales
    let fullMessage = message;
    if (data) {
      try {
        fullMessage += '\n' + JSON.stringify(data, null, 2);
      } catch (e) {
        fullMessage += '\n[Data serialization failed]';
      }
    }
    
    // Enviar al backend para guardar en archivo
    try {
      await invoke('log_frontend_error', {
        level,
        message: fullMessage,
        context: context || undefined,
      });
    } catch (error) {
      // Si falla el logging, solo mostrar en consola
      console.error('Failed to log to backend:', error);
    }
  }
  
  /**
   * Registra un mensaje de información
   */
  async info(message: string, context?: string, data?: any) {
    return this.log('info', message, context, data);
  }
  
  /**
   * Registra una advertencia
   */
  async warn(message: string, context?: string, data?: any) {
    return this.log('warn', message, context, data);
  }
  
  /**
   * Registra un error
   */
  async error(message: string, error?: any, context?: string, extraData?: any) {
    let errorData: Record<string, any> | undefined;

    if (error instanceof Error) {
      const errorAny = error as Record<string, unknown>;
      errorData = {
        message: error.message,
        stack: error.stack,
      };

      // Incluir propiedades adicionales del error si existen
      const enumerableProps: Record<string, unknown> = {};
      for (const key of Object.getOwnPropertyNames(errorAny)) {
        if (key !== 'message' && key !== 'stack') {
          enumerableProps[key] = errorAny[key];
        }
      }
      if (Object.keys(enumerableProps).length > 0) {
        errorData = { ...errorData, ...enumerableProps };
      }
    } else if (error !== undefined) {
      if (typeof error === 'object') {
        errorData = { ...error };
      } else {
        errorData = { value: error };
      }
    }

    if (extraData) {
      errorData = { ...(errorData || {}), ...extraData };
    }

    return this.log('error', message, context, errorData);
  }
  
  /**
   * Registra un mensaje de debug
   */
  async debug(message: string, context?: string, data?: any) {
    return this.log('debug', message, context, data);
  }
  
  /**
   * Obtiene todos los logs del frontend
   */
  async getLogs(): Promise<string> {
    try {
      return await invoke<string>('get_frontend_logs');
    } catch (error) {
      console.error('Failed to get frontend logs:', error);
      return 'Failed to retrieve logs';
    }
  }
  
  /**
   * Limpia todos los logs del frontend
   */
  async clearLogs(): Promise<void> {
    try {
      await invoke('clear_frontend_logs');
    } catch (error) {
      console.error('Failed to clear frontend logs:', error);
    }
  }
  
  /**
   * Abre la carpeta de logs en el explorador de archivos
   */
  async openLogFolder(): Promise<void> {
    try {
      await invoke('open_frontend_log_folder');
    } catch (error) {
      console.error('Failed to open log folder:', error);
    }
  }
}

// Exportar instancia singleton
export const logger = FrontendLogger.getInstance();

// Exportar también la clase para casos especiales
export default FrontendLogger;


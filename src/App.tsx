import { useState, useEffect, useRef } from "react";
import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import Loader from "@/components/Loader";
import ToastContainer from "@/components/ToastContainer";
import Sidebar from "@/components/Sidebar";
import UserProfile from "@/components/UserProfile";
import SettingsView from "@/components/SettingsView";
import InstanceView from "@/components/InstanceView";
import DownloadProgressToast from "@/components/DownloadProgressToast";
import UpdateReadyToast from "@/components/UpdateReadyToast";
import { SkinManager } from "@/components/skin/SkinManager";
import { getCurrentWindow, ProgressBarStatus } from "@tauri-apps/api/window";
import { sendNotification } from "@tauri-apps/plugin-notification";
import { UpdaterService } from "@/services/updater";
import { WhitelistService } from "@/services/whitelist";
import { SessionService } from "@/services/sessions";
import { AdminService } from "@/services/admins";
import NoAccessScreen from "@/components/NoAccessScreen";
import CreateLocalInstanceModal from "@/components/CreateLocalInstanceModal";
import type { LocalInstance } from "@/types/local-instances";
import kindlyklanLogo from "@/assets/kindlyklan.png";
import microsoftIcon from "@/assets/icons/microsoft.svg";
import { logger } from "@/utils/logger";
type AssetDownloadProgress = {
  current: number;
  total: number;
  percentage: number;
  current_file: string;
  status: string;
};


const getRequiredJavaVersion = (minecraftVersion: string): string => {
  const version = minecraftVersion.split('.')[1]; 

  if (parseInt(version) >= 21) return '21';
  if (parseInt(version) >= 20) return '17';
  if (parseInt(version) >= 18) return '17';
  if (parseInt(version) >= 17) return '16';
  if (parseInt(version) >= 8) return '8';

  return '8'; 
};


const checkJavaInstalled = async (javaVersion: string): Promise<boolean> => {
  try {
    const result = await invoke<string>('check_java_version', { version: javaVersion });
    return result === 'installed';
  } catch (error) {
    console.error('Error checking Java version:', error);
    return false;
  }
};


const ensureJavaInstalled = async (
  minecraftVersion: string,
  setJavaProgress?: (progress: number) => void
): Promise<string> => {
  const javaVersion = getRequiredJavaVersion(minecraftVersion);

  const isInstalled = await checkJavaInstalled(javaVersion);
  if (isInstalled) {
    return javaVersion;
  }

  try {
    // Escuchar eventos de progreso de Java
    const unlistenProgress = await listen('java-download-progress', (e: any) => {
      const data = e.payload as { percentage: number; status: string };
      const progress = data.percentage / 100;
      // Actualizar barra de progreso en la barra de tareas
      getCurrentWindow().setProgressBar({ 
        status: ProgressBarStatus.Normal, 
        progress: progress 
      }).catch(() => {});
      if (setJavaProgress) {
        setJavaProgress(data.percentage);
      }
    });
    
    const unlistenCompleted = await listen('java-download-completed', async (e: any) => {
      try {
        await getCurrentWindow().setProgressBar({ status: ProgressBarStatus.None });
        // Mostrar notificación
        sendNotification({
          title: 'Java instalado',
          body: `Java ${e.payload.version || ''} se ha instalado correctamente`,
        });
      } catch {}
      unlistenProgress();
      unlistenCompleted();
    });
    
    await invoke<string>('download_java', { version: javaVersion });
    return javaVersion;
  } catch (error) {
    console.error('Error downloading Java:', error);
    try { await getCurrentWindow().setProgressBar({ status: ProgressBarStatus.None }); } catch {}
    throw error;
  }
};

const launchInstance = async (
  instance: any,
  currentAccount: Account | null,
  addToast: (message: string, type?: 'success' | 'error' | 'info', duration?: number) => void,
  onComplete?: () => void,
  setIsDownloadingAssets?: (downloading: boolean) => void,
  setDownloadProgress?: Dispatch<SetStateAction<AssetDownloadProgress | null>>,
  onAuthError?: () => void,
  baseUrl?: string,
  instanceUrl?: string
): Promise<void> => {
  let javaVersion = '';

  try {
    javaVersion = await ensureJavaInstalled(instance.minecraft_version);

    if (onComplete) {
      onComplete();
    }

    const javaPath = await invoke<string>('get_java_path', { version: javaVersion });

    await invoke<string>('create_instance_directory', {
      instanceId: instance.id,
      javaVersion: javaVersion
    });

    if (setIsDownloadingAssets && setDownloadProgress) {
      setIsDownloadingAssets(true);
      setDownloadProgress(null);

      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unlistenProgress = await listen('asset-download-progress', (e: any) => {
          const data = e.payload as AssetDownloadProgress;
          setDownloadProgress(data);
          // Actualizar barra de progreso en la barra de tareas
          const progress = data.percentage / 100;
          getCurrentWindow().setProgressBar({ 
            status: ProgressBarStatus.Normal, 
            progress: progress 
          }).catch(() => {});
        });
        const unlistenCompleted = await listen('asset-download-completed', async () => {
          setDownloadProgress({ current: 100, total: 100, percentage: 100, current_file: '', status: 'Completed' });
          try {
            await getCurrentWindow().setProgressBar({ status: ProgressBarStatus.None });
            sendNotification({
              title: 'Instancia lista',
              body: `La instancia "${instance.name}" se ha descargado correctamente`,
            });
          } catch {}
          unlistenProgress();
          unlistenCompleted();
        });

        await invoke<string>('download_instance_assets', {
          instanceId: instance.id,
          minecraftVersion: instance.minecraft_version,
          baseUrl: baseUrl,
          instanceUrl: instanceUrl
        });

        unlistenProgress();
        unlistenCompleted();
      } catch (error) {
        console.error('Error downloading assets:', error);
        try { await getCurrentWindow().setProgressBar({ status: ProgressBarStatus.None }); } catch {}
        addToast('Error descargando assets de la instancia', 'error');
        throw error;
      }
    }

    // Validar y refrescar sesión antes de lanzar
    let accessToken = currentAccount?.user.access_token || '';
    if (currentAccount?.user.username) {
      try {
        const sessionResponse = await invoke<EnsureSessionResponse>('ensure_valid_session', {
          username: currentAccount.user.username
        });
        
        if (sessionResponse.status === 'Ok' && sessionResponse.data?.session) {
          accessToken = sessionResponse.data.session.access_token;
          if (sessionResponse.data.refreshed) {
            addToast('Sesión renovada automáticamente', 'info', 2000);
          }
        } else if (sessionResponse.status === 'Err') {
          // Error de sesión, pedir login
          addToast('Sesión expirada. Por favor, inicia sesión nuevamente.', 'error');
          if (onAuthError) {
            onAuthError();
          }
          return;
        }
      } catch (error) {
        console.error('Error validating session:', error);
        // Continuar con el token actual si hay error de red
      }
    }

    // Load saved RAM configuration
    const [minRam, maxRam] = await invoke<[number, number]>('load_ram_config');
    
    await invoke<string>('launch_minecraft_with_java', {
      appHandle: undefined,
      instanceId: instance.id,
      javaPath: javaPath,
      minecraftVersion: instance.minecraft_version,
      javaVersion: javaVersion,
      accessToken: accessToken,
      minRamGb: minRam,
      maxRamGb: maxRam
    });

    if (setIsDownloadingAssets) setIsDownloadingAssets(false);
    if (setDownloadProgress) setDownloadProgress(null);
    addToast(`Instancia "${instance.name}" lanzada correctamente`, 'success');
  } catch (error) {
    console.error('Error launching instance:', error);
    if (onComplete) {
      onComplete();
    }

    if (setIsDownloadingAssets) {
      setIsDownloadingAssets(false);
    }
    if (setDownloadProgress) {
      setDownloadProgress(null);
    }

    // Handle authentication errors using structured codes
    if (error && typeof error === 'string') {
      try {
        const errorData = JSON.parse(error);
        if (errorData.status === 'Err' && ['NO_SESSION', 'NO_REFRESH', 'REFRESH_FAILED', 'PROFILE_401'].includes(errorData.code)) {
          addToast('Sesión expirada. Por favor, inicia sesión nuevamente.', 'error');
          if (onAuthError) {
            onAuthError();
          }
          return;
        }
      } catch {
        // Si no es JSON, ignorar y continuar con el error genérico
      }
    }

    addToast(`Error lanzando ${instance.name}`, 'error');
    throw error;
  }
};


if (typeof window !== 'undefined') {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    return false;
  });

  document.addEventListener('keydown', (e) => {

    if (
      e.key === 'F12' ||
      (e.ctrlKey && e.shiftKey && e.key === 'I') ||
      (e.ctrlKey && e.key === 'U') ||
      (e.key === 'F5' && e.ctrlKey) 
    ) {
      e.preventDefault();
      return false;
    }
  });
}

interface AuthSession {
  access_token: string;
  username: string;
  uuid: string;
  user_type: string;
  expires_at?: number;
  refresh_token?: string;
}

interface EnsureSessionResponse {
  status: 'Ok' | 'Err';
  data?: {
    session?: {
      id: string;
      username: string;
      access_token: string;
      refresh_token: string | null;
      expires_at: number;
      created_at: number;
      updated_at: number;
    };
    refreshed?: boolean;
    code?: string;
    message?: string;
  };
}

interface Account {
  id: string;
  user: AuthSession;
  isActive: boolean;
}


interface ToastItem {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
  duration?: number;
}

interface DistributionManifest {
  distribution: {
    name: string;
    version: string;
    description: string;
    base_url: string;
    last_updated: string;
  };
  instances: Array<{
    id: string;
    name: string;
    description: string;
    version: string;
    minecraft_version: string;
    icon?: string;
    background?: string;
    background_video?: string;
    last_updated?: string;
    instance_url: string;
    mod_loader?: {
      type: string;
      version: string;
    };
  }>;
}


function App() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [currentAccount, setCurrentAccount] = useState<Account | null>(null);
  const [distribution, setDistribution] = useState<DistributionManifest | null>(null);
  const [selectedInstance, setSelectedInstance] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [showLoader, setShowLoader] = useState(false);
  const [loaderText, setLoaderText] = useState("Iniciando sesión...");
  const [isTransitioning, setIsTransitioning] = useState(false);
  const [isLoginVisible, setIsLoginVisible] = useState(true);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [distributionLoaded, setDistributionLoaded] = useState(false);
  const [skinViewOpen, setSkinViewOpen] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<AssetDownloadProgress | null>(null);
  const [logoVisible, setLogoVisible] = useState(false);
  const [isDownloadingAssets, setIsDownloadingAssets] = useState(false);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateDialogState, setUpdateDialogState] = useState<{ isDownloadReady: boolean; hasUpdateAvailable: boolean; version: string | null } | null>(null);
  const [showNoAccessScreen, setShowNoAccessScreen] = useState(false);
  const [filteredInstances, setFilteredInstances] = useState<any[]>([]);
  const initialized = useRef(false);
  
  // Estados para toasts de actualización
  const [updateDownloadProgress, setUpdateDownloadProgress] = useState<number | null>(null);
  const [updateDownloadVersion, setUpdateDownloadVersion] = useState<string | null>(null);
  const [updateReadyVersion, setUpdateReadyVersion] = useState<string | null>(null);
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);

  // Estados para instancias locales
  const [localInstances, setLocalInstances] = useState<LocalInstance[]>([]);
  const [isAdmin, setIsAdmin] = useState(false);
  const [createLocalModalOpen, setCreateLocalModalOpen] = useState(false);
  const [creatingInstanceId, setCreatingInstanceId] = useState<string | null>(null);
  const [syncModsModalOpen, setSyncModsModalOpen] = useState(false);
  const [syncingLocalId, setSyncingLocalId] = useState<string | null>(null);

  useEffect(() => {
    void logger.info('Aplicación iniciada', 'APP');
  }, []);
  
  useEffect(() => {}, [distributionLoaded]);
  
  // Manejar evento de cierre durante descarga
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen('close-requested-during-download', () => {
      setCloseDialogOpen(true);
    }).then((fn) => { unlisten = fn; }).catch(() => {});
    return () => { if (unlisten) { try { unlisten(); } catch {} } };
  }, []);
  
  useEffect(() => {
    if (!selectedInstance && !settingsOpen && !skinViewOpen && currentAccount) {
      setLogoVisible(false);
      const timer = setTimeout(() => setLogoVisible(true), 50);
      return () => clearTimeout(timer);
    }
  }, [selectedInstance, settingsOpen, skinViewOpen, currentAccount]);
  
  const DISTRIBUTION_URL = 'http://files.kindlyklan.com:26500/dist/manifest.json';

  // Check for updates on startup - verificar si ya estamos en la nueva versión y limpiar estado si es así
  const checkForUpdatesOnStartup = async () => {
    try {
      const state = await UpdaterService.getUpdateState();
      const currentVersion = state.current_version;
      
      // Si la versión actual coincide con la disponible, significa que ya se instaló
      // Limpiar el estado automáticamente para evitar mostrar "necesita instalar"
      if (state.available_version && state.available_version === currentVersion) {
        console.log('La versión actual coincide con la disponible, limpiando estado...');
        await invoke('clear_update_state');
        // Continuar con la verificación después de limpiar para buscar nuevas actualizaciones
      }
      
      // Si hay una actualización descargada y lista, instalar automáticamente al reiniciar
      if (state.download_ready) {
        console.log('Actualización descargada encontrada al iniciar, instalando automáticamente...');
        try {
          const result = await UpdaterService.installUpdate();
          if (result.success) {
            addToast('Actualización instalada. La aplicación se reiniciará.', 'success');
            // La aplicación se reiniciará automáticamente después de la instalación
            return;
          } else {
            console.error('Error instalando actualización automática:', result.message);
            // Si falla, mostrar toast de actualización lista para que el usuario pueda instalar manualmente
            if (state.available_version) {
              setUpdateReadyVersion(state.available_version);
            }
            return;
          }
        } catch (error) {
          console.error('Error instalando actualización automática:', error);
          // Si falla, mostrar toast de actualización lista para que el usuario pueda instalar manualmente
          if (state.available_version) {
            setUpdateReadyVersion(state.available_version);
          }
          return;
        }
      }

      // SIEMPRE verificar si hay nuevas actualizaciones al iniciar (no solo cada 30 minutos)
      // Esto asegura que siempre se verifique al iniciar el launcher
      console.log('Verificando actualizaciones al iniciar...');
      const result = await UpdaterService.checkForUpdates();
      
      if (result.available) {
        // Obtener el estado actualizado después de verificar
        const newState = await UpdaterService.getUpdateState();
        if (newState.available_version && !newState.downloaded) {
          console.log('Nueva actualización disponible, iniciando descarga automática...');
          // Mostrar toast de descarga con progreso ANTES de iniciar la descarga
          setUpdateDownloadVersion(newState.available_version);
          setUpdateDownloadProgress(0);
          
          // Descargar automáticamente
          const downloadResult = await UpdaterService.downloadUpdateSilent(false);
          if (!downloadResult.success) {
            // Si falla la descarga, ocultar toast de descarga y mostrar error
            setUpdateDownloadProgress(null);
            setUpdateDownloadVersion(null);
            addToast('Error al descargar la actualización', 'error');
            const finalState = await UpdaterService.getUpdateState();
            if (finalState.available_version) {
              setUpdateDialogState({ isDownloadReady: false, hasUpdateAvailable: true, version: finalState.available_version });
              setUpdateDialogOpen(true);
            }
          }
          // Si tiene éxito, el toast de "lista para instalar" se mostrará cuando se complete
          // a través del evento update-download-complete
        }
      } else {
        console.log('No hay actualizaciones disponibles');
      }
    } catch (error) {
      console.error('Error checking for updates on startup:', error);
    }
  };

  // Verificar actualizaciones periódicamente (cada 30 minutos)
  const checkForUpdatesPeriodic = async () => {
    try {
      const shouldCheck = await UpdaterService.shouldCheckForUpdates();
      if (!shouldCheck) {
        console.log('Aún no es momento de verificar actualizaciones (cada 30 minutos)');
        return;
      }

      console.log('Verificando actualizaciones periódicamente...');
      const result = await UpdaterService.checkForUpdates();
      if (result.available) {
        // Obtener el estado actualizado después de verificar
        const state = await UpdaterService.getUpdateState();
        if (state.available_version && !state.downloaded) {
          console.log('Nueva actualización disponible en verificación periódica, iniciando descarga...');
          // Mostrar toast de descarga con progreso ANTES de iniciar la descarga
          setUpdateDownloadVersion(state.available_version);
          setUpdateDownloadProgress(0);
          
          // Descargar automáticamente
          const downloadResult = await UpdaterService.downloadUpdateSilent(false);
          if (!downloadResult.success) {
            // Si falla la descarga, ocultar toast de descarga y mostrar error
            setUpdateDownloadProgress(null);
            setUpdateDownloadVersion(null);
            addToast('Error al descargar la actualización', 'error');
            const newState = await UpdaterService.getUpdateState();
            if (newState.available_version && !newState.download_ready) {
              setUpdateDialogState({ isDownloadReady: false, hasUpdateAvailable: true, version: newState.available_version });
              setUpdateDialogOpen(true);
            }
          }
          // Si tiene éxito, el toast de "lista para instalar" se mostrará cuando se complete
          // a través del evento update-download-complete
        }
      } else {
        console.log('No hay actualizaciones disponibles en verificación periódica');
      }
    } catch (error) {
      console.error('Error checking for updates periodically:', error);
    }
  };

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    const hideInitialLoader = () => {
      const loaderElement = document.querySelector('.initial-loader') as HTMLElement;
      if (loaderElement) {
        loaderElement.classList.add('hidden');
      }
    };
    setTimeout(hideInitialLoader, 100);

    loadDistribution();
    checkExistingSession().catch(console.error);
    
    // Check for updates after a short delay to not interfere with startup
    setTimeout(() => {
      checkForUpdatesOnStartup();
    }, 2000);

    // Verificar actualizaciones periódicamente cada 30 minutos
    const updateCheckInterval = setInterval(() => {
      checkForUpdatesPeriodic();
    }, 30 * 60 * 1000); // 30 minutos

    // Escuchar eventos de actualización
    let unlistenUpdateStart: (() => void) | null = null;
    let unlistenUpdateProgress: (() => void) | null = null;
    let unlistenUpdateComplete: (() => void) | null = null;
    
    (async () => {
      try {
        // Listener para cuando inicia la descarga - mostrar toast inmediatamente
        unlistenUpdateStart = await listen('update-download-start', async () => {
          console.log('Descarga de actualización iniciada');
          // Obtener el estado para asegurar que tenemos la versión correcta
          try {
            const state = await UpdaterService.getUpdateState();
            if (state.available_version) {
              setUpdateDownloadVersion(state.available_version);
              setUpdateDownloadProgress(0);
            }
          } catch (error) {
            console.error('Error obteniendo estado de actualización:', error);
            // Aún así, establecer el progreso a 0 para mostrar el toast
            setUpdateDownloadProgress(0);
          }
        });
        
        unlistenUpdateProgress = await listen<number>('update-download-progress', (event) => {
          setUpdateDownloadProgress(event.payload);
        });
        
        unlistenUpdateComplete = await listen('update-download-complete', async () => {
          console.log('Descarga de actualización completada');
          setUpdateDownloadProgress(100);
          // Esperar un poco para que se actualice el estado
          setTimeout(async () => {
            const state = await UpdaterService.getUpdateState();
            // Verificar que la actualización no se haya instalado ya
            if (state.available_version && state.available_version === state.current_version) {
              // Ya está instalada, limpiar estado
              await invoke('clear_update_state');
              setUpdateDownloadProgress(null);
              setUpdateDownloadVersion(null);
              return;
            }
            if (state.download_ready && state.available_version) {
              setUpdateDownloadProgress(null);
              setUpdateDownloadVersion(null);
              setUpdateReadyVersion(state.available_version);
            }
          }, 500);
        });
      } catch (error) {
        console.error('Error setting up update event listeners:', error);
      }
    })();

    if (accounts.length === 0 && !isLoginVisible) {
      const timer = setTimeout(() => {
        setIsLoginVisible(true);
      }, 100);
      return () => {
        clearTimeout(timer);
        clearInterval(updateCheckInterval);
        if (unlistenUpdateStart) unlistenUpdateStart();
        if (unlistenUpdateProgress) unlistenUpdateProgress();
        if (unlistenUpdateComplete) unlistenUpdateComplete();
      };
    }
    
    return () => {
      clearInterval(updateCheckInterval);
      if (unlistenUpdateStart) unlistenUpdateStart();
      if (unlistenUpdateProgress) unlistenUpdateProgress();
      if (unlistenUpdateComplete) unlistenUpdateComplete();
    };
  }, [accounts.length, isLoginVisible]);

  useEffect(() => {
    if (accounts.length === 0) return;

    const validateAllTokens = async () => {
      const validAccounts: Account[] = [];

      for (const account of accounts) {
        const isValid = await validateAccountToken(account);
        if (isValid) {
          validAccounts.push(account);
        } else {
          console.warn(`Token inválido para cuenta ${account.user.username}, eliminando...`);
        }
      }

      if (validAccounts.length !== accounts.length) {
        setAccounts(validAccounts);
        // DB es fuente de verdad, no sincronizar a localStorage


        if (currentAccount && !validAccounts.find(acc => acc.id === currentAccount.id)) {
          if (validAccounts.length > 0) {
            setCurrentAccount(validAccounts[0]);
            addToast(`Cuenta activa cambiada a: ${validAccounts[0].user.username}`, 'info');
          } else {
            setCurrentAccount(null);
            setIsLoginVisible(true);
            addToast('Todas las cuentas han expirado. Vuelve a iniciar sesión.', 'info');
          }
        }
      }
    };
    validateAllTokens();
    const interval = setInterval(validateAllTokens, 5 * 60 * 1000); 

    return () => clearInterval(interval);
  }, [accounts, currentAccount]);


  /* TOASTS */
  const addToast = (message: string, type: 'success' | 'error' | 'info' = 'info', duration = 5000) => {
    const id = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    setToasts(prev => [...prev, { id, message, type, duration }]);
  };

  const removeToast = (id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  };

  const handleInstanceSelect = (instanceId: string) => {
    setSkinViewOpen(false);
    setSettingsOpen(false);
    setSelectedInstance(instanceId);
  };

  const handleAddAccount = async () => {
    setCurrentAccount(null);
    setSelectedInstance(null);
    setSkinViewOpen(false);
    setSettingsOpen(false);
    setIsLoginVisible(true);

    addToast('Logéate para añadir una nueva cuenta', 'info');
  };

  const handleSwitchAccount = (account: Account) => {
    validateAccountToken(account).then(isValid => {
      if (!isValid) {
        addToast(`Token de ${account.user.username} ha expirado. Por favor, inicia sesión nuevamente.`, 'error');
        handleLogoutAccount(account.id);
        return;
      }

      setCurrentAccount(account);

      const updatedAccounts = accounts.map(acc => ({
        ...acc,
        isActive: acc.id === account.id
      }));
      setAccounts(updatedAccounts);
      // DB es fuente de verdad, no sincronizar a localStorage

      addToast(`Cambiado a cuenta: ${account.user.username}`, 'success');
    }).catch(error => {
      console.error('Error switching account:', error);
      addToast('Error al cambiar de cuenta', 'error');
    });
  };

  const validateAccountToken = async (account: Account): Promise<boolean> => {
    try {
      const refreshed = await SessionService.validateAndRefreshToken(account.user.username);
      // Actualizar en memoria si el backend renovó el token
      if (refreshed && refreshed.access_token && refreshed.username === account.user.username) {
        account.user.access_token = refreshed.access_token;
        account.user.expires_at = refreshed.expires_at;
      }
      return true;
    } catch (error) {
      console.error(`Token validation/refresh failed for account ${account.user.username}:`, error);
      return false;
    }
  };

  const handleLogoutAccount = async (accountId: string) => {
    const updatedAccounts = accounts.filter(acc => acc.id !== accountId);

    if (updatedAccounts.length === 0) {
      // Si no quedan cuentas, limpiar todo
      try {
        await SessionService.clearAllSessions();
      } catch (error) {
        console.error('Error clearing all sessions from database:', error);
      }

      setAccounts([]);
      setCurrentAccount(null);
      setIsLoginVisible(true);
      addToast('Todas las cuentas cerradas. Vuelve a iniciar sesión.', 'info');
    } else {
      // Si quedan cuentas, establecer la primera como activa
      const newActiveAccount = updatedAccounts[0];
      setCurrentAccount(newActiveAccount);

      // También limpiar sesión de la base de datos para la cuenta cerrada
      try {
        const accountToRemove = accounts.find(acc => acc.id === accountId);
        if (accountToRemove) {
          await SessionService.deleteSession(accountToRemove.user.username);
        }
      } catch (error) {
        console.error('Error deleting session from database:', error);
      }

      setAccounts(updatedAccounts);
      // DB es fuente de verdad, no sincronizar a localStorage

      addToast(`Sesión cerrada.`, 'info');
    }

    setSelectedInstance(null);
    setSkinViewOpen(false);
    setSettingsOpen(false);
  };

  const handleSettingsToggle = () => {
    if (!settingsOpen) {
      setSkinViewOpen(false);
      setSelectedInstance(null);
      setSettingsOpen(true);
    } else {
      setSettingsOpen(false);
      setSelectedInstance(null);
    }
  };

  const handleSkinToggle = () => {
    if (!skinViewOpen) {
      setSettingsOpen(false);
      setSelectedInstance(null);
      setSkinViewOpen(true);
    } else {
      setSkinViewOpen(false);
      setSelectedInstance(null);
    }
  };


  const loadDistribution = async () => {
    if (distributionLoaded) return; 
    try {
      const manifest = await invoke<DistributionManifest>('load_distribution_manifest', {
        url: DISTRIBUTION_URL
      });
      setDistribution(manifest);
      setDistributionLoaded(true);
      
      // Filtrar instancias según permisos del usuario actual
      if (currentAccount) {
        const accessibleInstances = await WhitelistService.getAccessibleInstances(
          currentAccount.user.username,
          manifest.instances
        );
        setFilteredInstances(accessibleInstances);
      } else {
        // Si no hay usuario logueado, mostrar todas las instancias
        setFilteredInstances(manifest.instances);
      }
      
      addToast(`¡Instancias cargadas correctamente!`, 'success');
    } catch (error) {
      addToast('Error al cargar la distribución', 'error');
    }
  };

  // Check if current user is admin
  const checkAdminStatus = async () => {
    if (!currentAccount) {
      setIsAdmin(false);
      return;
    }

    try {
      const admin = await AdminService.checkIsAdmin(currentAccount.user.username);
      setIsAdmin(admin);
      console.log(`Admin status for ${currentAccount.user.username}: ${admin}`);
    } catch (error) {
      console.error('Error checking admin status:', error);
      setIsAdmin(false);
    }
  };

  // Load local instances (only for admins)
  const loadLocalInstances = async () => {
    if (!isAdmin) {
      setLocalInstances([]);
      return;
    }

    try {
      const instances = await invoke<LocalInstance[]>('get_local_instances');
      setLocalInstances(instances);
      console.log('Local instances loaded:', instances.length);
    } catch (error) {
      console.error('Error loading local instances:', error);
      addToast('Error al cargar instancias locales', 'error');
    }
  };

  // Handle creating a local instance
  const handleCreateLocalInstance = (instance: LocalInstance) => {
    setLocalInstances([...localInstances, instance]);
    setCreatingInstanceId(instance.id);    
    // Remove creating state after animation
    setTimeout(() => {
      setCreatingInstanceId(null);
    }, 2000);
  };

  // Handle sync mods from remote
  const handleSyncMods = (localId: string) => {
    setSyncingLocalId(localId);
    setSyncModsModalOpen(true);
  };

  const handleSyncModsConfirm = async (remoteId: string) => {
    if (!syncingLocalId || !distribution) return;

    try {
      setShowLoader(true);
      setLoaderText('Sincronizando mods...');
      
      await invoke('sync_mods_from_remote', {
        localInstanceId: syncingLocalId,
        remoteInstanceId: remoteId,
        distributionUrl: distribution.distribution.base_url,
      });
      setSyncModsModalOpen(false);
      setSyncingLocalId(null);
    } catch (error) {
      console.error('Error syncing mods:', error);
      addToast(`Error al sincronizar mods: ${error}`, 'error');
    } finally {
      setShowLoader(false);
      setLoaderText('Iniciando sesión...');
    }
  };

  // Handle open instance folder
  const handleOpenFolder = async (instanceId: string) => {
    try {
      await invoke('open_instance_folder', { instanceId });
    } catch (error) {
      console.error('Error opening folder:', error);
      addToast('Error al abrir carpeta de la instancia', 'error');
    }
  };

  // Handle local instance deleted
  const handleLocalInstanceDeleted = (instanceId: string) => {
    setLocalInstances(localInstances.filter(li => li.id !== instanceId));
    if (selectedInstance === instanceId) {
      setSelectedInstance(null);
    }
    addToast('Instancia local eliminada', 'success');
  };

  // Effect to check admin status when user changes
  useEffect(() => {
    checkAdminStatus();
  }, [currentAccount]);

  // Effect to load local instances when admin status changes
  useEffect(() => {
    loadLocalInstances();
  }, [isAdmin]);

  // Listen to local instance creation progress
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    listen('local-instance-progress', (event: any) => {
      const progress = event.payload;
      console.log('Local instance progress:', progress);
      
      if (progress.stage === 'completed') {
        addToast(progress.message, 'success');
        loadLocalInstances(); // Reload local instances
      }
    }).then((fn) => { unlisten = fn; }).catch(() => {});
    
    return () => { if (unlisten) { try { unlisten(); } catch {} } };
  }, []);

  // Listen to mod sync progress
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    listen('mod-sync-progress', (event: any) => {
      const progress = event.payload;
      console.log('Mod sync progress:', progress);
      
      if (progress.stage === 'completed') {
        addToast(progress.message, 'success');
      }
    }).then((fn) => { unlisten = fn; }).catch(() => {});
    
    return () => { if (unlisten) { try { unlisten(); } catch {} } };
  }, []);

  const handleLogout = async () => {
    try {
      // Limpiar sesiones de la base de datos si hay cuenta activa
      if (currentAccount) {
        await SessionService.deleteSession(currentAccount.user.username);
      }
      // También limpiar todas las sesiones por seguridad
      await SessionService.clearAllSessions();
    } catch (error) {
      console.error('Error clearing sessions from database:', error);
    }

    setAccounts([]);
    setCurrentAccount(null);
    setShowNoAccessScreen(false);
    setIsLoginVisible(true);
    addToast('Sesión cerrada correctamente', 'info');
  };

  const checkExistingSession = async () => {
    try {
      // Primero intentar cargar sesión activa desde la base de datos (el backend intentará refrescar si es posible)
      const activeSession = await SessionService.getActiveSession();

      if (activeSession) {
        console.log('Found active session for user:', activeSession.username);
        console.log('Session expires at:', new Date(activeSession.expires_at * 1000));
        console.log('Current time:', new Date());
        console.log('Is expired:', SessionService.isSessionExpired(activeSession));

        // Si expirada, eliminar; si expira pronto, intentar refresh
        if (SessionService.isSessionExpired(activeSession)) {
          console.log('Session expired, removing...');
          await SessionService.deleteSession(activeSession.username);
          setShowNoAccessScreen(true);
          return;
        }

        // Si expira pronto, intentar refresh de forma transparente
        if (SessionService.isSessionExpiringSoon(activeSession, 10)) {
          try {
            const refreshed = await SessionService.refreshActiveSession(activeSession.username);
            console.log('Session refreshed until:', new Date(refreshed.expires_at * 1000));
          } catch (refreshError) {
            console.error('Session refresh failed:', refreshError);
            // si falla, permanecer con la sesión actual hasta que realmente expire
          }
        }

        // Crear cuenta desde la sesión con id=username para evitar duplicados
        const account: Account = {
          id: activeSession.username,
          user: {
            access_token: activeSession.access_token,
            username: activeSession.username,
            uuid: activeSession.uuid, // UUID real de Minecraft para la skin
            user_type: 'microsoft',
            expires_at: activeSession.expires_at
          },
          isActive: true
        };

        setAccounts([account]);
        setCurrentAccount(account);

        try {
          const accessCheck = await WhitelistService.checkAccess(account.user.username);
          if (!accessCheck.has_access) {
            setAccounts([]);
            setCurrentAccount(null);
            setShowNoAccessScreen(true);
            await SessionService.deleteSession(activeSession.username);
            return;
          }
        } catch (whitelistError) {
          console.error('Error checking whitelist for existing session:', whitelistError);
          // No eliminar la sesión por error de whitelist, solo mostrar advertencia
          addToast('Advertencia: No se pudo verificar el acceso. Contacta a un administrador si hay problemas.', 'info');
        }
      }
      // NO borrar localStorage aquí - la DB es la fuente de verdad
    } catch (error) {
      console.error('Error checking existing session:', error);
      // Si hay error con la base de datos, intentar fallback a localStorage
      const savedAccounts = localStorage.getItem('kkk_accounts');
      const activeAccountId = localStorage.getItem('kkk_active_account');

      if (savedAccounts && activeAccountId) {
        try {
          const accountsData = JSON.parse(savedAccounts);
          setAccounts(accountsData);

          const activeAccount = accountsData.find((acc: Account) => acc.id === activeAccountId);
          if (activeAccount) {
            setCurrentAccount(activeAccount);
          }
        } catch (parseError) {
          console.error('Error parsing saved accounts:', parseError);
          // NO borrar localStorage por errores de parsing - puede ser temporal
        }
      }
    }
  };

  const handleMicrosoftAuth = async () => {
    setIsLoading(true);
    setLoaderText("Iniciando sesión...");
    setShowLoader(true);

    try {
      const userSession = await invoke<AuthSession>('start_microsoft_auth');

      const newAccount: Account = {
        id: userSession.username, // Usar username como id desde el inicio para evitar duplicados
        user: userSession,
        isActive: true
      };

      // Guardar sesión en la base de datos con tokens reales
      // Backend devuelve expires_at en SEGUNDOS (timestamp UNIX), NO dividir por 1000
      const expiresAt = userSession.expires_at || Math.floor(Date.now() / 1000) + 3600;
      console.log('Saving session for user:', userSession.username);
      console.log('Expires at:', new Date(expiresAt * 1000));
      console.log('Refresh token available:', !!userSession.refresh_token);
      console.log('Raw expires_at from backend:', userSession.expires_at);

      try {
        await SessionService.saveSession(
          userSession.username,
          userSession.uuid,
          userSession.access_token,
          userSession.refresh_token || null, // Convertir undefined a null
          expiresAt
        );
        console.log('✅ Session saved successfully to database with UUID:', userSession.uuid);
      } catch (sessionError) {
        console.error('❌ CRITICAL: Error saving session to database:', sessionError);
        addToast('Error crítico: No se pudo guardar la sesión. Contacta a soporte.', 'error', 10000);
        setIsLoading(false);
        setShowLoader(false);
        throw sessionError; // NO continuar si no se puede guardar
      }

      // Evitar duplicados: filtrar por username y agregar la nueva cuenta
      const updatedAccounts = [...accounts.filter(a => a.user.username !== newAccount.user.username), newAccount];
      setCurrentAccount(newAccount);
      setAccounts(updatedAccounts);
      // DB es fuente de verdad, no sincronizar a localStorage

      // Verificar whitelist después de autenticación exitosa
      setLoaderText("Verificando acceso...");
      try {
        // Añadimos timeout a la verificación de whitelist para evitar bloqueo
        const whitelistPromise = WhitelistService.checkAccess(userSession.username);
        const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('Whitelist timeout')), 8000));
        const accessCheck = await Promise.race([whitelistPromise, timeoutPromise]) as any;

        if (!accessCheck.has_access) {
          // Eliminar sesión de la base de datos si no tiene acceso
          await SessionService.deleteSession(userSession.username);
          setShowNoAccessScreen(true);
          setIsLoading(false);
          setShowLoader(false);
          return;
        }

        // Si tiene acceso, continuar con el flujo normal
        addToast('Autenticación exitosa.', 'success');
        setIsTransitioning(true);

        setTimeout(() => {
          setIsLoading(false);
          setShowLoader(false);
          setLoaderText("Iniciando sesión...");

          setTimeout(() => {
            setIsTransitioning(false);
          }, 500);
        }, 1000);
        
      } catch (whitelistError) {
        console.error('Whitelist check error:', whitelistError);
        addToast('Error verificando acceso. Inténtalo de nuevo.', 'error');
        setIsLoading(false);
        setShowLoader(false);
      }
      
    } catch (error) {
      console.error('Microsoft auth error:', error);
      addToast('Error en autenticación: ' + error, 'error');
      setIsLoading(false);
      setShowLoader(false);
      setLoaderText("Iniciando sesión...");
      setIsTransitioning(false);
    }
  };




  return (
    <div className="min-h-screen bg-gradient-to-br from-black via-[#0a0a0a] to-black flex relative overflow-hidden">
      {/* No Access Screen - Full screen overlay */}
      {showNoAccessScreen ? (
        <NoAccessScreen 
          onLogout={handleLogout}
          username={currentAccount?.user.username}
        />
      ) : (
        <>
          {currentAccount && (
               <Sidebar
                 instances={filteredInstances.length > 0 ? filteredInstances : (distribution?.instances || [])}
                 localInstances={localInstances}
                 selectedInstance={selectedInstance}
                 onInstanceSelect={handleInstanceSelect}
                 handleSettingsToggle={handleSettingsToggle}
                 handleSkinToggle={handleSkinToggle}
                 distributionBaseUrl={distribution?.distribution.base_url || ''}
                 currentUser={currentAccount.user}
                 settingsOpen={settingsOpen}
                 isAdmin={isAdmin}
                 onCreateLocalInstance={() => setCreateLocalModalOpen(true)}
                 creatingInstanceId={creatingInstanceId}
                 onLocalInstanceDeleted={handleLocalInstanceDeleted}
                 addToast={addToast}
               />
          )}

          <div className={`flex-1 flex flex-col ${currentAccount ? 'ml-20' : ''}`}>
            {accounts.length > 0 && (
              <div className="absolute top-4 right-4 z-50">
                <UserProfile
                  accounts={accounts}
                  currentAccount={currentAccount}
                  onSwitchAccount={handleSwitchAccount}
                  onLogoutAccount={handleLogoutAccount}
                  onAddAccount={handleAddAccount}
                />
              </div>
            )}

                 <main className={`flex-1 relative transition-all duration-500 ease-out ${isTransitioning ? 'opacity-0 scale-95' : 'opacity-100 scale-100'}`}>
                  {!currentAccount ? (
                <div className={`flex items-center justify-center h-full transition-all duration-500 ease-out ${isLoginVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}`}>
                  <div className="text-center group animate-fade-in-up">
                    <div className={`mb-10 transition-all duration-500 delay-200 ${isLoginVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95'}`}>
                      <div className="p-12 inline-block">
                        <img
                          src={kindlyklanLogo}
                          alt="KindlyKlan"
                          className="w-48 h-48 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_40px_rgba(0,255,255,0.4)] group-hover:scale-105 select-none"
                        />
                      </div>
                    </div>
                    <div className={`transition-all duration-500 delay-400 ${isLoginVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
                <Button
                        onClick={handleMicrosoftAuth}
                        disabled={isLoading}
                        className="relative glass-light hover:bg-white/10 text-white border-2 border-white/20 hover:border-[#00ffff]/50 
                                 rounded-2xl px-16 py-6 text-2xl font-semibold transition-all duration-300 ease-out 
                                 shadow-2xl hover:shadow-[0_0_30px_rgba(0,255,255,0.3)] group overflow-hidden min-w-[380px] 
                                 cursor-pointer hover:scale-105 neon-glow-cyan-hover"
                      >
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-[#00ffff]/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>
                        <img src={microsoftIcon} alt="Microsoft" className="w-8 h-8 mr-3 relative z-10" />
                        <span className="relative z-10">Iniciar Sesión</span>
                </Button>
              </div>
            </div>
                </div>
               ) : skinViewOpen ? (
                 <SkinManager
                   currentUser={currentAccount?.user}
                   onClose={() => setSkinViewOpen(false)}
                 />
               ) : settingsOpen ? (
                 <SettingsView
                   addToast={addToast}
                 />
               ) : !distribution ? (
                <div className="flex items-center justify-center h-full">
                  <Loader text="Cargando distribución..." variant="orbital" showReloadAfter={30} />
            </div>
               ) : !selectedInstance ? (
                 <div className="relative h-full w-full overflow-hidden">
                   
                   {/* Background - More subtle gradient */}
                   <div className="absolute inset-0 z-0">
                     <div
                       className="w-full h-full"
                       style={{
                         background: 'linear-gradient(135deg, #000000 0%, #0a0a0a 50%, #000000 100%)'
                       }}
                     />
                   </div>

                   {/* Subtle neon accents in background */}
                   <div className="absolute inset-0 z-5 opacity-10">
                     <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-[#00ffff] rounded-full blur-3xl"></div>
                     <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-[#ff00ff] rounded-full blur-3xl"></div>
                   </div>

                   {/* Overlay */}
                   <div className="absolute inset-0 bg-black/60 z-10" />

                   {/* Content */}
                   <div className="relative z-20 h-full flex flex-col">
                     <div className={`flex-1 flex items-center justify-center p-8 transition-all duration-500 ease-out delay-200 ${logoVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`}>
                       <div className="text-center group animate-scale-in">
                         <div className="p-16 inline-block">
                           <img 
                             src={kindlyklanLogo} 
                             alt="KindlyKlan" 
                             className="w-64 h-64 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_50px_rgba(0,255,255,0.5)] group-hover:scale-105"
                           />
                         </div>
                       </div>
                     </div>
                   </div>
                 </div>
               ) : (
                 <div className="h-full">
                   <InstanceView
                     instanceId={selectedInstance}
                     distribution={distribution}
                     distributionBaseUrl={distribution.distribution.base_url}
                     isJavaInstalling={showLoader || isDownloadingAssets}
                     localInstance={localInstances.find(li => li.id === selectedInstance)}
                     isLocal={localInstances.some(li => li.id === selectedInstance)}
                     onSyncMods={handleSyncMods}
                     onOpenFolder={handleOpenFolder}
                     onLaunch={async (instance) => {
                       if (isDownloadingAssets) {
                         setLoaderText("Descargando assets de instancia...");
                       } else {
                         setLoaderText("Descargando Java...");
                       }
                       setShowLoader(true);

                       // Check if it's a local instance
                       const isLocalInstance = localInstances.some(li => li.id === selectedInstance);
                       
                       if (isLocalInstance) {
                         // Launch local instance
                         try {
                           const localInst = localInstances.find(li => li.id === selectedInstance);
                           if (!localInst) throw new Error('Local instance not found');
                           
                           await invoke('launch_local_instance', {
                             instanceId: localInst.id,
                             accessToken: currentAccount.user.access_token,
                             username: currentAccount.user.username,
                             uuid: currentAccount.user.uuid,
                             minRamGb: 4.0,
                             maxRamGb: 8.0,
                           });
                           
                           setShowLoader(false);
                           setLoaderText("Iniciando sesión...");
                           addToast('Minecraft iniciado exitosamente', 'success');
                         } catch (error) {
                           console.error('Error launching local instance:', error);
                           addToast(`Error al iniciar instancia: ${error}`, 'error');
                           setShowLoader(false);
                           setLoaderText("Iniciando sesión...");
                         }
                       } else {
                         // Launch remote instance
                       await launchInstance(
                         instance,
                         currentAccount,
                         addToast,
                         () => {
                           setShowLoader(false);
                           setLoaderText("Iniciando sesión...");
                         },
                         setIsDownloadingAssets,
                         setDownloadProgress,
                         () => {
                           // Auth error callback - clear account and show login
                           setCurrentAccount(null);
                           setIsLoginVisible(true);
                        },
                        distribution?.distribution.base_url,
                        instance.instance_url
                       );
                       }
                     }}
                   />
          </div>
        )}
             </main>
      </div>


      {showLoader && (
        <div className={`blur-overlay transition-all duration-500 ${isTransitioning ? 'opacity-0 scale-110' : 'opacity-100 scale-100'}`}>
          <Loader text={loaderText} />
        </div>
      )}

      <ToastContainer toasts={toasts} onRemove={removeToast}>
        {downloadProgress && (
          <DownloadProgressToast
            message={downloadProgress.status === 'Completed' ? 'Assets descargados' : 'Descargando assets de instancia'}
            percentage={downloadProgress.percentage}
            onClose={() => setDownloadProgress(null)}
          />
        )}
        {updateDownloadProgress !== null && updateDownloadVersion && (
          <DownloadProgressToast
            message="Descargando nueva actualización"
            percentage={updateDownloadProgress}
            onClose={() => {
              setUpdateDownloadProgress(null);
              setUpdateDownloadVersion(null);
            }}
          />
        )}
        {updateReadyVersion && (
          <UpdateReadyToast
            message="Nueva actualización lista para instalar"
            version={updateReadyVersion}
            onClose={() => setUpdateReadyVersion(null)}
            onClick={() => {
              setSettingsOpen(true);
              setSelectedInstance(null);
              setSkinViewOpen(false);
            }}
          />
        )}
      </ToastContainer>

      {/* Update Dialog */}
      {updateDialogOpen && updateDialogState && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div 
            className="rounded-2xl border-2 border-green-400/60 p-8 max-w-md w-full mx-4 shadow-2xl"
            style={{
              background: updateDialogState.isDownloadReady
                ? 'linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(0, 0, 0, 0.7) 100%)'
                : 'linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(0, 0, 0, 0.7) 100%)',
              backdropFilter: 'blur(20px)',
              WebkitBackdropFilter: 'blur(20px)',
              boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.6)'
            }}
          >
            <div className="text-center">
              <div className="w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4"
                style={{
                  background: updateDialogState.isDownloadReady
                    ? 'linear-gradient(135deg, rgba(34, 197, 94, 0.2) 0%, rgba(0, 0, 0, 0.4) 100%)'
                    : 'linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(0, 0, 0, 0.4) 100%)',
                  backdropFilter: 'blur(10px)',
                  WebkitBackdropFilter: 'blur(10px)',
                  border: updateDialogState.isDownloadReady
                    ? '2px solid rgba(34, 197, 94, 0.4)'
                    : '2px solid rgba(59, 130, 246, 0.4)'
                }}
              >
                <svg 
                  className={`w-8 h-8 ${updateDialogState.isDownloadReady ? 'text-green-300' : 'text-blue-300'}`} 
                  fill="none" 
                  stroke="currentColor" 
                  viewBox="0 0 24 24"
                >
                  {updateDialogState.isDownloadReady ? (
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  ) : (
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  )}
                </svg>
              </div>
              
              {updateDialogState.isDownloadReady ? (
                <>
                  <h3 className="text-2xl font-bold text-white mb-2">Actualización Lista</h3>
                  <p className="text-white/80 mb-6">
                    Hay una actualización descargada y lista para instalar. La aplicación se reiniciará después de la instalación.
                  </p>
                  
                  <div className="flex gap-3 justify-center">
                    <button
                      onClick={async () => {
                        setUpdateDialogOpen(false);
                        try {
                          const result = await UpdaterService.installUpdate();
                          if (result.success) {
                            addToast('Actualización instalada. La aplicación se reiniciará.', 'success');
                          } else {
                            addToast('Error al instalar la actualización', 'error');
                          }
                        } catch (error) {
                          addToast('Error al instalar la actualización', 'error');
                        }
                      }}
                      className="px-6 py-3 rounded-xl border-2 border-green-400/60 text-green-200 transition-all duration-200 font-medium"
                      style={{
                        background: 'linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
                        backdropFilter: 'blur(20px)',
                        WebkitBackdropFilter: 'blur(20px)',
                        boxShadow: '0 4px 16px 0 rgba(0, 0, 0, 0.4)'
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(34, 197, 94, 0.25) 0%, rgba(0, 0, 0, 0.6) 100%)';
                        e.currentTarget.style.boxShadow = '0 6px 20px 0 rgba(34, 197, 94, 0.3)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)';
                        e.currentTarget.style.boxShadow = '0 4px 16px 0 rgba(0, 0, 0, 0.4)';
                      }}
                    >
                      Instalar Ahora
                    </button>
                  </div>
                </>
              ) : updateDialogState.hasUpdateAvailable ? (
                <>
                  <h3 className="text-2xl font-bold text-white mb-2">Actualización Disponible</h3>
                  <p className="text-white/80 mb-6">
                    Hay una nueva versión disponible ({updateDialogState.version}). ¿Quieres descargarla ahora?
                  </p>
                  
                  <div className="flex gap-3 justify-center">
                    <button
                      onClick={async () => {
                        setUpdateDialogOpen(false);
                        try {
                          // Descarga automática (desde diálogo), NO manual
                          const result = await UpdaterService.downloadUpdateSilent(false);
                          if (result.success) {
                            addToast('Actualización descargada correctamente', 'success');
                            // Verificar el estado después de descargar
                            const newState = await UpdaterService.getUpdateState();
                            if (newState.download_ready) {
                              // Mostrar diálogo de instalación
                              setUpdateDialogState({ isDownloadReady: true, hasUpdateAvailable: false, version: newState.available_version });
                              setTimeout(() => setUpdateDialogOpen(true), 500);
                            }
                          } else {
                            addToast('Error al descargar la actualización', 'error');
                          }
                        } catch (error) {
                          addToast('Error al descargar la actualización', 'error');
                        }
                      }}
                      className="px-6 py-3 rounded-xl border-2 border-blue-400/60 text-blue-200 transition-all duration-200 font-medium"
                      style={{
                        background: 'linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
                        backdropFilter: 'blur(20px)',
                        WebkitBackdropFilter: 'blur(20px)',
                        boxShadow: '0 4px 16px 0 rgba(0, 0, 0, 0.4)'
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(59, 130, 246, 0.25) 0%, rgba(0, 0, 0, 0.6) 100%)';
                        e.currentTarget.style.boxShadow = '0 6px 20px 0 rgba(59, 130, 246, 0.3)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)';
                        e.currentTarget.style.boxShadow = '0 4px 16px 0 rgba(0, 0, 0, 0.4)';
                      }}
                    >
                      Descargar
                    </button>
                    
                    <button
                      onClick={() => {
                        setUpdateDialogOpen(false);
                      }}
                      className="px-6 py-3 rounded-xl border-2 border-gray-400/60 text-gray-200 transition-all duration-200 font-medium"
                      style={{
                        background: 'linear-gradient(135deg, rgba(156, 163, 175, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
                        backdropFilter: 'blur(20px)',
                        WebkitBackdropFilter: 'blur(20px)',
                        boxShadow: '0 4px 16px 0 rgba(0, 0, 0, 0.4)'
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(156, 163, 175, 0.25) 0%, rgba(0, 0, 0, 0.6) 100%)';
                        e.currentTarget.style.boxShadow = '0 6px 20px 0 rgba(156, 163, 175, 0.3)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'linear-gradient(135deg, rgba(156, 163, 175, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)';
                        e.currentTarget.style.boxShadow = '0 4px 16px 0 rgba(0, 0, 0, 0.4)';
                      }}
                    >
                      Más Tarde
                    </button>
                  </div>
                </>
              ) : null}
            </div>
          </div>
        </div>
      )}

      {/* Diálogo de confirmación de cierre durante descarga */}
      {closeDialogOpen && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div 
            className="rounded-2xl border-2 border-orange-400/60 p-8 max-w-md w-full mx-4 shadow-2xl"
            style={{
              background: 'linear-gradient(135deg, rgba(234, 88, 12, 0.15) 0%, rgba(0, 0, 0, 0.7) 100%)',
              backdropFilter: 'blur(20px)',
              WebkitBackdropFilter: 'blur(20px)',
              boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.6)'
            }}
          >
            <div className="text-center">
              <div className="w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4"
                style={{
                  background: 'linear-gradient(135deg, rgba(234, 88, 12, 0.2) 0%, rgba(0, 0, 0, 0.4) 100%)',
                  backdropFilter: 'blur(10px)',
                  WebkitBackdropFilter: 'blur(10px)',
                  border: '2px solid rgba(234, 88, 12, 0.4)'
                }}
              >
                <svg className="w-8 h-8 text-orange-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
              </div>
              
              <h3 className="text-2xl font-bold text-white mb-2">Descarga en progreso</h3>
              <p className="text-white/80 mb-6">
                Hay una descarga en progreso. Si cierras la aplicación ahora, la descarga se cancelará. ¿Estás seguro de que quieres cerrar?
              </p>
              
              <div className="flex gap-3 justify-center">
                <button
                  onClick={async () => {
                    setCloseDialogOpen(false);
                    // Permitir cierre forzado
                    await invoke('set_downloading_state', { isDownloading: false });
                    const { getCurrentWindow } = await import('@tauri-apps/api/window');
                    await getCurrentWindow().close();
                  }}
                  className="px-6 py-3 rounded-xl border-2 border-red-400/60 text-red-200 transition-all duration-200 font-medium"
                  style={{
                    background: 'linear-gradient(135deg, rgba(239, 68, 68, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
                    backdropFilter: 'blur(20px)',
                    WebkitBackdropFilter: 'blur(20px)',
                    boxShadow: '0 4px 16px 0 rgba(0, 0, 0, 0.4)'
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(239, 68, 68, 0.25) 0%, rgba(0, 0, 0, 0.6) 100%)';
                    e.currentTarget.style.boxShadow = '0 6px 20px 0 rgba(239, 68, 68, 0.3)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(239, 68, 68, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)';
                    e.currentTarget.style.boxShadow = '0 4px 16px 0 rgba(0, 0, 0, 0.4)';
                  }}
                >
                  Cerrar de todas formas
                </button>
                
                <button
                  onClick={() => setCloseDialogOpen(false)}
                  className="px-6 py-3 rounded-xl border-2 border-gray-400/60 text-gray-200 transition-all duration-200 font-medium"
                  style={{
                    background: 'linear-gradient(135deg, rgba(156, 163, 175, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)',
                    backdropFilter: 'blur(20px)',
                    WebkitBackdropFilter: 'blur(20px)',
                    boxShadow: '0 4px 16px 0 rgba(0, 0, 0, 0.4)'
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(156, 163, 175, 0.25) 0%, rgba(0, 0, 0, 0.6) 100%)';
                    e.currentTarget.style.boxShadow = '0 6px 20px 0 rgba(156, 163, 175, 0.3)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(156, 163, 175, 0.15) 0%, rgba(0, 0, 0, 0.5) 100%)';
                    e.currentTarget.style.boxShadow = '0 4px 16px 0 rgba(0, 0, 0, 0.4)';
                  }}
                >
                  Cancelar
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal de creación de instancia local */}
      <CreateLocalInstanceModal
        isOpen={createLocalModalOpen}
        onClose={() => setCreateLocalModalOpen(false)}
        onInstanceCreated={handleCreateLocalInstance}
      />

      {/* Modal de sincronización de mods */}
      {syncModsModalOpen && distribution && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div 
            className="glass-card rounded-3xl border border-white/10 p-8 max-w-2xl w-full shadow-2xl"
            style={{
              background: 'rgba(10, 10, 10, 0.95)',
              backdropFilter: 'blur(24px)',
              WebkitBackdropFilter: 'blur(24px)',
            }}
          >
            <h2 className="text-3xl font-bold text-white mb-4">
              Sincronizar Mods
            </h2>
            <p className="text-white/60 mb-6">
              Selecciona una instancia remota para copiar sus mods a esta instancia local
            </p>

            <div className="space-y-3 max-h-[400px] overflow-y-auto custom-scrollbar mb-6">
              {filteredInstances.map((instance) => (
                <button
                  key={instance.id}
                  onClick={() => handleSyncModsConfirm(instance.id)}
                  className="w-full p-4 rounded-xl bg-white/5 border border-white/10 text-left hover:bg-white/10 hover:border-[#00ffff]/30 transition-all duration-200 group"
                >
                  <div className="flex items-center gap-4">
                    {instance.icon && (
                      <img
                        src={`${distribution.distribution.base_url}/${instance.icon}`}
                        alt={instance.name}
                        className="w-12 h-12 rounded-lg"
                      />
                    )}
                    <div className="flex-1">
                      <h3 className="text-white font-bold group-hover:text-[#00ffff] transition-colors">
                        {instance.name}
                      </h3>
                      <p className="text-white/60 text-sm">
                        Minecraft {instance.minecraft_version} • {instance.mod_loader?.type} {instance.mod_loader?.version}
                      </p>
                    </div>
                    <svg 
                      className="w-6 h-6 text-white/40 group-hover:text-[#00ffff] transition-colors" 
                      fill="none" 
                      stroke="currentColor" 
                      viewBox="0 0 24 24"
                    >
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                    </svg>
                  </div>
                </button>
              ))}
            </div>

            <div className="flex justify-end">
              <button
                onClick={() => {
                  setSyncModsModalOpen(false);
                  setSyncingLocalId(null);
                }}
                className="px-6 py-3 rounded-xl bg-white/5 border border-white/10 text-white hover:bg-white/10 transition-all duration-200"
              >
                Cancelar
              </button>
            </div>
          </div>
        </div>
      )}
        </>
      )}
    </div>
  );
}

export default App;

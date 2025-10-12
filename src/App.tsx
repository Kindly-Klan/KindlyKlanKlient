import { useState, useEffect, useRef } from "react";
import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import Loader from "@/components/Loader";
import ToastContainer from "@/components/ToastContainer";
import Sidebar from "@/components/Sidebar";
import UserProfile from "@/components/UserProfile";
import SettingsView from "@/components/SettingsView";
import InstanceView from "@/components/InstanceView";
import DownloadProgressToast from "@/components/DownloadProgressToast";
import { SkinManager } from "@/components/skin/SkinManager";
import { UpdaterService } from "@/services/updater";
import { WhitelistService } from "@/services/whitelist";
import NoAccessScreen from "@/components/NoAccessScreen";
import kindlyklanLogo from "@/assets/kindlyklan.png";
import microsoftIcon from "@/assets/icons/microsoft.svg";
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


const ensureJavaInstalled = async (minecraftVersion: string): Promise<string> => {
  const javaVersion = getRequiredJavaVersion(minecraftVersion);

  const isInstalled = await checkJavaInstalled(javaVersion);
  if (isInstalled) {
    return javaVersion;
  }


  try {
    await invoke<string>('download_java', { version: javaVersion });
    return javaVersion;
  } catch (error) {
    console.error('Error downloading Java:', error);
    throw error;
  }
};

const launchInstance = async (
  instance: any,
  currentAccount: Account | null,
  addToast: (message: string, type?: 'success' | 'error' | 'info', duration?: number) => void,
  onComplete?: () => void,
  setIsDownloadingAssets?: (downloading: boolean) => void,
  setDownloadProgress?: Dispatch<SetStateAction<AssetDownloadProgress | null>>
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
        });
        const unlistenCompleted = await listen('asset-download-completed', () => {
          setDownloadProgress({ current: 100, total: 100, percentage: 100, current_file: '', status: 'Completed' });
        });

        await invoke<string>('download_instance_assets', {
          appHandle: undefined,
          instanceId: instance.id,
          distributionUrl: 'http://files.kindlyklan.com:26500/dist'
        });

        unlistenProgress();
        unlistenCompleted();

        addToast('Assets descargados correctamente', 'success');

      } catch (error) {
        console.error('Error downloading assets:', error);
        addToast('Error descargando assets de la instancia', 'error');
        throw error;
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
      accessToken: currentAccount?.user.access_token || '',
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
    last_updated?: string;
    instance_url: string;
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
  const [showNoAccessScreen, setShowNoAccessScreen] = useState(false);
  const [filteredInstances, setFilteredInstances] = useState<any[]>([]);
  const initialized = useRef(false);

  useEffect(() => {}, [distributionLoaded]);
  
  useEffect(() => {
    if (!selectedInstance && !settingsOpen && !skinViewOpen && currentAccount) {
      setLogoVisible(false);
      const timer = setTimeout(() => setLogoVisible(true), 50);
      return () => clearTimeout(timer);
    }
  }, [selectedInstance, settingsOpen, skinViewOpen, currentAccount]);
  
  const DISTRIBUTION_URL = 'http://files.kindlyklan.com:26500/dist/manifest.json';

  // Check for updates on startup
  const checkForUpdatesOnStartup = async () => {
    try {
      // Check if we should check for updates (every 6 hours)
      const shouldCheck = await UpdaterService.shouldCheckForUpdates();
      if (!shouldCheck) return;

      // Check if there's already a downloaded update ready
      const state = await UpdaterService.getUpdateState();
      if (state.download_ready) {
        setUpdateDialogOpen(true);
        return;
      }

      // Check for new updates in background
      const result = await UpdaterService.checkForUpdates();
      if (result.available) {
        // Download the update silently
        await UpdaterService.downloadUpdateSilent();
        
        // Show toast notification
        addToast('Actualización descargada. Puedes instalarla desde Configuración.', 'info', 5000);
      }
    } catch (error) {
      console.error('Error checking for updates on startup:', error);
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

    if (accounts.length === 0 && !isLoginVisible) {
      const timer = setTimeout(() => {
        setIsLoginVisible(true);
      }, 100);
      return () => clearTimeout(timer);
    }
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
        localStorage.setItem('kkk_accounts', JSON.stringify(validAccounts));


        if (currentAccount && !validAccounts.find(acc => acc.id === currentAccount.id)) {
          if (validAccounts.length > 0) {
            setCurrentAccount(validAccounts[0]);
            localStorage.setItem('kkk_active_account', validAccounts[0].id);
            addToast(`Cuenta activa cambiada a: ${validAccounts[0].user.username}`, 'info');
          } else {
            setCurrentAccount(null);
            setIsLoginVisible(true);
            localStorage.removeItem('kkk_active_account');
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
    localStorage.removeItem('kkk_active_account');

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
      localStorage.setItem('kkk_active_account', account.id);

      const updatedAccounts = accounts.map(acc => ({
        ...acc,
        isActive: acc.id === account.id
      }));
      setAccounts(updatedAccounts);
      localStorage.setItem('kkk_accounts', JSON.stringify(updatedAccounts));

      addToast(`Cambiado a cuenta: ${account.user.username}`, 'success');
    }).catch(error => {
      console.error('Error switching account:', error);
      addToast('Error al cambiar de cuenta', 'error');
    });
  };

  const validateAccountToken = async (account: Account): Promise<boolean> => {
    try {
      await invoke<string>('get_minecraft_profile', { accessToken: account.user.access_token });
      return true;
    } catch (error) {
      console.error(`Token validation failed for account ${account.user.username}:`, error);
      return false;
    }
  };

  const handleLogoutAccount = (accountId: string) => {
    const updatedAccounts = accounts.filter(acc => acc.id !== accountId);

    if (updatedAccounts.length === 0) {
      setAccounts([]);
      setCurrentAccount(null);
      localStorage.removeItem('kkk_accounts');
      localStorage.removeItem('kkk_active_account');
      setIsLoginVisible(true);
      addToast('Todas las cuentas cerradas. Vuelve a iniciar sesión.', 'info');
    } else {
      const newActiveAccount = updatedAccounts[0];
      setCurrentAccount(newActiveAccount);
      localStorage.setItem('kkk_active_account', newActiveAccount.id);

      setAccounts(updatedAccounts);
      localStorage.setItem('kkk_accounts', JSON.stringify(updatedAccounts));

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

  const handleLogout = () => {
    setAccounts([]);
    setCurrentAccount(null);
    setShowNoAccessScreen(false);
    setIsLoginVisible(true);
    localStorage.removeItem('kkk_accounts');
    localStorage.removeItem('kkk_active_account');
    addToast('Sesión cerrada correctamente', 'info');
  };

  const checkExistingSession = async () => {
    const savedAccounts = localStorage.getItem('kkk_accounts');
    const activeAccountId = localStorage.getItem('kkk_active_account');

    if (savedAccounts) {
      try {
        const accountsData = JSON.parse(savedAccounts);
        setAccounts(accountsData);

        let activeAccount = null;
        if (activeAccountId) {
          activeAccount = accountsData.find((acc: Account) => acc.id === activeAccountId);
        } else if (accountsData.length > 0) {
          activeAccount = accountsData[0];
          localStorage.setItem('kkk_active_account', accountsData[0].id);
        }

        if (activeAccount) {
          // Verificar whitelist para la sesión existente
          try {
            const accessCheck = await WhitelistService.checkAccess(activeAccount.user.username);
            
            if (!accessCheck.has_access) {
              // Usuario sin acceso - limpiar sesión y mostrar pantalla de acceso denegado
              setAccounts([]);
              setCurrentAccount(null);
              setShowNoAccessScreen(true);
              localStorage.removeItem('kkk_accounts');
              localStorage.removeItem('kkk_active_account');
              return;
            }

            // Usuario con acceso - establecer cuenta activa
            setCurrentAccount(activeAccount);
          } catch (whitelistError) {
            console.error('Error checking whitelist for existing session:', whitelistError);
            // En caso de error, permitir acceso pero mostrar advertencia
            setCurrentAccount(activeAccount);
            addToast('Advertencia: No se pudo verificar el acceso. Contacta a un administrador si hay problemas.', 'info');
          }
        }
      } catch (error) {
        console.error('Error parsing saved accounts:', error);
        localStorage.removeItem('kkk_accounts');
        localStorage.removeItem('kkk_active_account');
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
        id: `account_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        user: userSession,
        isActive: true
      };

      const updatedAccounts = [...accounts, newAccount];

      if (accounts.length === 0) {
        setCurrentAccount(newAccount);
        localStorage.setItem('kkk_active_account', newAccount.id);
      }

      setAccounts(updatedAccounts);
      localStorage.setItem('kkk_accounts', JSON.stringify(updatedAccounts));
      
      // Verificar whitelist después de autenticación exitosa
      setLoaderText("Verificando acceso...");
      try {
        const accessCheck = await WhitelistService.checkAccess(userSession.username);
        
        if (!accessCheck.has_access) {
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
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-slate-900 to-black flex relative overflow-hidden">
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
                 selectedInstance={selectedInstance}
                 onInstanceSelect={handleInstanceSelect}
                 handleSettingsToggle={handleSettingsToggle}
                 handleSkinToggle={handleSkinToggle}
                 distributionBaseUrl={distribution?.distribution.base_url || ''}
                 currentUser={currentAccount.user}
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

                 <main className={`flex-1 relative transition-all duration-700 ${isTransitioning ? 'opacity-0 scale-95' : 'opacity-100 scale-100'}`}>
                  {!currentAccount ? (
                <div className={`flex items-center justify-center h-full transition-all duration-700 ${isLoginVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}`}>
                  <div className="text-center group">
                    <div className={`mb-8 transition-all duration-500 delay-200 ${isLoginVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95'}`}>
                      <img
                        src={kindlyklanLogo}
                        alt="KindlyKlan"
                        className="w-48 h-48 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_30px_rgba(255,255,255,0.3)] select-none"
                      />
                    </div>
                    <div className={`transition-all duration-500 delay-400 ${isLoginVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
                <Button
                        onClick={handleMicrosoftAuth}
                        disabled={isLoading}
                        className="relative bg-black hover:bg-gray-900 text-white border-2 border-gray-600 hover:border-gray-400 rounded-2xl px-16 py-6 text-2xl font-semibold transition-all duration-300 shadow-2xl hover:shadow-white/20 group overflow-hidden min-w-[380px] cursor-pointer"
                      >
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>
                        <img src={microsoftIcon} alt="Microsoft" className="w-8 h-8 mr-3" />
                        Iniciar Sesión
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
                   onClose={() => setSettingsOpen(false)}
                 />
               ) : !distribution ? (
                <div className="flex items-center justify-center h-full">
                  <Loader text="Cargando distribución..." variant="orbital" showReloadAfter={30} />
            </div>
               ) : !selectedInstance ? (
                 <div className="relative h-full w-full overflow-hidden">
                   
                   {/* Background */}
                   <div className="absolute inset-0 z-0">
                     <div
                       className="w-full h-full"
                       style={{
                         background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)'
                       }}
                     />
                   </div>

                   {/* Overlay */}
                   <div className="absolute inset-0 bg-black/40 z-10" />

                   {/* Content */}
                   <div className="relative z-20 h-full flex flex-col">
                     <div className={`flex-1 flex items-center justify-center p-8 transition-all duration-700 delay-200 ${logoVisible ? 'opacity-100 translate-y-0 scale-100' : 'opacity-0 translate-y-8 scale-95'}`}>
                       <div className="text-center group transition-all duration-500 hover:scale-110 hover:drop-shadow-2xl">
                         <img 
                           src={kindlyklanLogo} 
                           alt="KindlyKlan" 
                           className="w-64 h-64 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_30px_rgba(255,255,255,0.3)]"
                         />
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
                     onLaunch={async (instance) => {
                       if (isDownloadingAssets) {
                         setLoaderText("Descargando assets de instancia...");
                       } else {
                         setLoaderText("Descargando Java...");
                       }
                       setShowLoader(true);

                       await launchInstance(
                         instance,
                         currentAccount,
                         addToast,
                         () => {
                           setShowLoader(false);
                           setLoaderText("Iniciando sesión...");
                         },
                         setIsDownloadingAssets,
                         setDownloadProgress
                       );
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
      </ToastContainer>

      {/* Update Dialog */}
      {updateDialogOpen && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="bg-gray-900/95 backdrop-blur-md rounded-2xl border border-white/10 p-8 max-w-md w-full mx-4 shadow-2xl">
            <div className="text-center">
              <div className="w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
                <svg className="w-8 h-8 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              
              <h3 className="text-2xl font-bold text-white mb-2">Actualización Lista</h3>
              <p className="text-white/80 mb-6">
                Hay una actualización descargada y lista para instalar. ¿Quieres instalarla ahora?
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
                  className="px-6 py-3 bg-green-500/20 hover:bg-green-500/30 text-green-300 border border-green-500/30 rounded-lg transition-all duration-200 font-medium"
                >
                  Instalar Ahora
                </button>
                
                <button
                  onClick={() => setUpdateDialogOpen(false)}
                  className="px-6 py-3 bg-gray-500/20 hover:bg-gray-500/30 text-gray-300 border border-gray-500/30 rounded-lg transition-all duration-200 font-medium"
                >
                  Más Tarde
                </button>
              </div>
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

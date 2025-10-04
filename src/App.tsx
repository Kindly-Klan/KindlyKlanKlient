import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import Loader from "@/components/Loader";
import ToastContainer from "@/components/ToastContainer";
import Sidebar from "@/components/Sidebar";
import UserProfile from "@/components/UserProfile";
import SettingsPanel from "@/components/SettingsPanel";
import InstanceView from "@/components/InstanceView";
import { SkinManager } from "@/components/skin/SkinManager";


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

const launchInstance = async (instance: any, addToast: (message: string, type?: 'success' | 'error' | 'info', duration?: number) => void, onComplete?: () => void): Promise<void> => {
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

    await invoke<string>('launch_minecraft_with_java', {
      instanceId: instance.id,
      javaPath: javaPath,
      minecraftVersion: instance.minecraft_version,
      javaVersion: javaVersion
    });

    addToast(`Instancia "${instance.name}" lanzada correctamente`, 'success');
  } catch (error) {
    console.error('Error launching instance:', error);
    if (onComplete) {
      onComplete();
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
  const initialized = useRef(false);


  useEffect(() => {
    console.log('distributionLoaded changed to:', distributionLoaded);
  }, [distributionLoaded]);
  const DISTRIBUTION_URL = 'http://files.kindlyklan.com:26500/dist/manifest.json';

  useEffect(() => {
    if (initialized.current) {
      console.log('App useEffect triggered but already initialized');
      return;
    }
    console.log('App useEffect triggered - initializing');
    initialized.current = true;
    loadDistribution();
    checkExistingSession();

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
    if (skinViewOpen) setSkinViewOpen(false);
    setSelectedInstance(instanceId);
  };

  const handleAddAccount = async () => {
    setCurrentAccount(null);
    setSelectedInstance(null);
    setSkinViewOpen(false);
    setSettingsOpen(false);
    setIsLoginVisible(true);
    localStorage.removeItem('kkk_active_account');

    addToast('Añade una nueva cuenta desde la página de login', 'info');
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
    setSettingsOpen(!settingsOpen);
  };

  const handleSkinToggle = () => {
    setSkinViewOpen(!skinViewOpen);
    if (!skinViewOpen) {
      setSelectedInstance(null);
    }
  };


  const loadDistribution = async () => {
    console.log('loadDistribution called, distributionLoaded:', distributionLoaded);
    if (distributionLoaded) {
      console.log('Distribution already loaded, skipping');
      return; 
    }

    console.log('Loading distribution from:', DISTRIBUTION_URL);
    try {
      const manifest = await invoke<DistributionManifest>('load_distribution_manifest', {
        url: DISTRIBUTION_URL
      });
      setDistribution(manifest);
      setDistributionLoaded(true);
      console.log('Distribution loaded successfully:', manifest.distribution.name);
      addToast(`¡Instancias cargadas correctamente!`, 'success');
    } catch (error) {
      console.error('Error loading distribution:', error);
      addToast('Error al cargar la distribución', 'error');
    }
  };

  const checkExistingSession = () => {
    const savedAccounts = localStorage.getItem('kkk_accounts');
    const activeAccountId = localStorage.getItem('kkk_active_account');

    if (savedAccounts) {
      try {
        const accountsData = JSON.parse(savedAccounts);
        setAccounts(accountsData);

        if (activeAccountId) {
          const activeAccount = accountsData.find((acc: Account) => acc.id === activeAccountId);
          if (activeAccount) {
            setCurrentAccount(activeAccount);
          }
        } else if (accountsData.length > 0) {
          setCurrentAccount(accountsData[0]);
          localStorage.setItem('kkk_active_account', accountsData[0].id);
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
      {currentAccount && (
           <Sidebar
             instances={distribution?.instances || []}
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
                        src="/src/assets/kindlyklan.png"
                        alt="KindlyKlan"
                        className="w-48 h-48 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_30px_rgba(255,255,255,0.3)]"
                      />
                    </div>
                    <div className={`transition-all duration-500 delay-400 ${isLoginVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}`}>
                <Button
                        onClick={handleMicrosoftAuth}
                        disabled={isLoading}
                        className="relative bg-black hover:bg-gray-900 text-white border-2 border-gray-600 hover:border-gray-400 rounded-2xl px-16 py-6 text-2xl font-semibold transition-all duration-300 shadow-2xl hover:shadow-white/20 group overflow-hidden min-w-[380px]"
                      >
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>
                        <img src="/src/assets/icons/microsoft.svg" alt="Microsoft" className="w-8 h-8 mr-3" />
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
               ) : !distribution ? (
                <div className="flex items-center justify-center h-full">
                  <Loader text="Cargando distribución..." variant="orbital" showReloadAfter={30} />
            </div>
               ) : !selectedInstance ? (
                 <div className="flex items-center justify-center h-full">
                   <div className="text-center group cursor-pointer transition-all duration-500 hover:scale-110 hover:drop-shadow-2xl">
                     <img 
                       src="/src/assets/kindlyklan.png" 
                       alt="KindlyKlan" 
                       className="w-64 h-64 mx-auto transition-all duration-500 group-hover:brightness-110 group-hover:contrast-110 group-hover:drop-shadow-[0_0_30px_rgba(255,255,255,0.3)]"
                     />
          </div>
          </div>
               ) : (
                 <div className="h-full">
                   <InstanceView
                     instanceId={selectedInstance}
                     distribution={distribution}
                     distributionBaseUrl={distribution.distribution.base_url}
                     isJavaInstalling={showLoader}
                     onLaunch={async (instance) => {
                       setLoaderText("Descargando Java...");
                       setShowLoader(true);

                       await launchInstance(
                         instance,
                         addToast,
                         () => {
                           setShowLoader(false);
                           setLoaderText("Iniciando sesión...");
                         }
                       );
                     }}
                   />
          </div>
        )}
             </main>
      </div>

      <SettingsPanel
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        distributionUrl={DISTRIBUTION_URL}
        onReloadDistribution={loadDistribution}
      />

      {showLoader && (
        <div className={`blur-overlay transition-all duration-500 ${isTransitioning ? 'opacity-0 scale-110' : 'opacity-100 scale-100'}`}>
          <Loader text={loaderText} />
        </div>
      )}

      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </div>
  );
}

export default App;

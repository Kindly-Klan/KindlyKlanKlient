import React from 'react';
import { Button } from '@/components/ui/button';
import Tooltip from '@/components/ui/Tooltip';

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

interface UserProfileProps {
  accounts: Account[];
  currentAccount: Account | null;
  onSwitchAccount: (account: Account) => void;
  onLogoutAccount: (accountId: string) => void;
  onAddAccount: () => void;
}

const UserProfile: React.FC<UserProfileProps> = ({
  accounts,
  currentAccount,
  onSwitchAccount,
  onLogoutAccount,
  onAddAccount
}) => {
  const handleImageError = (e: React.SyntheticEvent<HTMLImageElement, Event>, account: Account) => {
    
    e.currentTarget.src = `data:image/svg+xml;base64,${btoa(`
      <svg width="40" height="40" viewBox="0 0 40 40" xmlns="http://www.w3.org/2000/svg">
        <rect width="40" height="40" rx="8" fill="#4A90E2"/>
        <text x="20" y="26" font-family="Arial, sans-serif" font-size="16" font-weight="bold" text-anchor="middle" fill="white">
          ${account.user.username.charAt(0).toUpperCase()}
        </text>
      </svg>
    `)}`;
  };

  return (
    <div className="flex items-center space-x-2 bg-black/30 backdrop-blur-md rounded-full px-3 py-2 border border-white/10">
      
      <div className="flex items-center space-x-1">
        {accounts.map((account) => (
          <div key={account.id} className="relative">
            <Tooltip content={account.user.username} side="bottom">
              <img
                src={`https://crafatar.com/avatars/${account.user.uuid}?size=32&overlay=true`}
                className={`w-6 h-6 rounded-lg border-2 cursor-pointer transition-all duration-200 ${
                  account.id === currentAccount?.id
                    ? 'border-blue-400 shadow-lg shadow-blue-400/50'
                    : 'border-white/20 hover:border-white/40'
                }`}
                onError={(e) => handleImageError(e, account)}
                onClick={() => onSwitchAccount(account)}
              />
            </Tooltip>
            
            {account.id === currentAccount?.id && (
              <div className="absolute -bottom-1 -right-1 w-2 h-2 bg-green-400 rounded-full border border-black"></div>
            )}
          </div>
        ))}
      </div>

      
      <Tooltip content="Añadir cuenta" side="bottom">
        <Button
          onClick={onAddAccount}
          size="sm"
          variant="ghost"
          className="w-6 h-6 p-0 text-white/60 hover:text-white hover:bg-white/10 rounded-full"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
        </Button>
      </Tooltip>

      
      <Tooltip content="Cerrar sesión" side="bottom">
        <Button
          onClick={() => currentAccount && onLogoutAccount(currentAccount.id)}
          size="sm"
          variant="ghost"
          className="w-6 h-6 p-0 text-white/60 hover:text-red-400 hover:bg-red-500/10 rounded-full"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
          </svg>
        </Button>
      </Tooltip>
    </div>
  );
};

export default UserProfile;

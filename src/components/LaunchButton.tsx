import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';

interface LaunchButtonProps {
	onLaunch: () => Promise<void>;
	disabled?: boolean;
	className?: string;
	isJavaInstalling?: boolean;
	instanceId?: string;
}

type LaunchState = 'idle' | 'launching' | 'playing';

// Caché global para estados de lanzamiento por instancia
const launchStateCache = new Map<string, { state: LaunchState; startTime: number }>();

const LaunchButton: React.FC<LaunchButtonProps> = ({
	onLaunch,
	disabled = false,
	className = '',
	isJavaInstalling = false,
	instanceId = 'default'
}) => {
	// Inicializar con estado en caché si existe
	const getInitialState = () => {
		const cachedState = launchStateCache.get(instanceId);
		return cachedState?.state || 'idle';
	};
	
	const [state, setState] = useState<LaunchState>(getInitialState());
	const [playTime, setPlayTime] = useState(0);
	const startTimeRef = useRef<number>(Date.now());
	const [isHovered, setIsHovered] = useState(false);
	
	// Actualizar estado cuando cambia instanceId
	useEffect(() => {
		const cachedState = launchStateCache.get(instanceId);
		if (cachedState) {
			setState(cachedState.state);
			if (cachedState.state === 'playing' || cachedState.state === 'launching') {
				startTimeRef.current = cachedState.startTime;
				// Calcular tiempo transcurrido
				const elapsed = Math.floor((Date.now() - cachedState.startTime) / 1000);
				setPlayTime(elapsed);
			} else {
				setPlayTime(0);
				startTimeRef.current = Date.now();
			}
		} else {
			setState('idle');
			setPlayTime(0);
			startTimeRef.current = Date.now();
		}
	}, [instanceId]);

	// Intervalo para incrementar el tiempo cuando está playing o launching
	useEffect(() => {
		let interval: NodeJS.Timeout | null = null;
		if (state === 'playing' || state === 'launching') {
			interval = setInterval(() => {
				const cachedState = launchStateCache.get(instanceId);
				if (cachedState && (cachedState.state === 'playing' || cachedState.state === 'launching')) {
					// Calcular tiempo transcurrido desde el startTime
					const elapsed = Math.floor((Date.now() - cachedState.startTime) / 1000);
					setPlayTime(elapsed);
				}
			}, 1000);
		} else {
			setPlayTime(0);
		}
		return () => { if (interval) clearInterval(interval); };
	}, [state, instanceId]);

	// Reset when the Minecraft process exits (only for this instance)
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		listen('minecraft_exited', (event: any) => {
			const data = event.payload;
			// Solo procesar si el evento es para esta instancia
			if (data.instance_id !== instanceId) {
				return;
			}
			
			// Guardar tiempo total jugado antes de resetear
			const cached = launchStateCache.get(instanceId);
			if (cached && cached.state === 'playing') {
				const elapsed = Math.floor((Date.now() - cached.startTime) / 1000);
				const hours = elapsed / 3600;
				const saved = localStorage.getItem(`playtime_${instanceId}`);
				const previousHours = saved ? parseFloat(saved) || 0 : 0;
				const totalHours = previousHours + hours;
				localStorage.setItem(`playtime_${instanceId}`, totalHours.toString());
			}
			
			setState('idle');
			setPlayTime(0);
			// Limpiar caché cuando el juego termina
			launchStateCache.delete(instanceId);
			startTimeRef.current = Date.now();
		}).then((fn) => { unlisten = fn; }).catch(() => {});
		return () => { if (unlisten) { try { unlisten(); } catch {} } };
	}, [instanceId]);

	const handleClick = async () => {
		// Verificar el estado actual del caché para esta instancia específica
		const currentCachedState = launchStateCache.get(instanceId);
		const currentState = currentCachedState?.state || state;
		
		if (disabled || currentState !== 'idle' || isJavaInstalling) {
			return;
		}
		
		setState('launching');
		const launchStartTime = Date.now();
		startTimeRef.current = launchStartTime;
		launchStateCache.set(instanceId, { state: 'launching', startTime: launchStartTime });
		setPlayTime(0);
		
		try {
			await onLaunch();
			const playStartTime = Date.now();
			startTimeRef.current = playStartTime;
			setState('playing');
			setPlayTime(0);
			launchStateCache.set(instanceId, { state: 'playing', startTime: playStartTime });
		} catch (error) {
			console.error(`[LaunchButton ${instanceId}] Error during launch:`, error);
			setState('idle');
			setPlayTime(0);
			launchStateCache.delete(instanceId);
		}
	};

	const formatTime = (seconds: number): string => {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		const secs = seconds % 60;
		if (hours > 0) return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
		return `${minutes}:${secs.toString().padStart(2, '0')}`;
	};

	const formatTimeForMarquee = (seconds: number): string => {
		const timeStr = formatTime(seconds);
		return timeStr.padEnd(10, ' ');
	};

	const getButtonContent = () => {
		switch (state) {
			case 'launching': {
				const timeText = formatTimeForMarquee(playTime);
				return (
					<div className="marquee-container">
						<div className="marquee-text">
							{timeText.split('').map((letter, index) => (
								<span
									key={index}
									className="marquee-letter"
									style={{
										animationDelay: `${(2.5 / timeText.length) * (timeText.length - 1 - index) * -1}s`
									}}
								>
									{letter === ' ' ? '\u00A0' : letter}
								</span>
							))}
						</div>
					</div>
				);
			}
			case 'playing': {
				const timeText = formatTimeForMarquee(playTime);
				return (
					<div className="marquee-container">
						<div className="marquee-text">
							{timeText.split('').map((letter, index) => (
								<span
									key={index}
									className="marquee-letter"
									style={{
										animationDelay: `${(2.5 / timeText.length) * (timeText.length - 1 - index) * -1}s`
									}}
								>
									{letter === ' ' ? '\u00A0' : letter}
								</span>
							))}
						</div>
					</div>
				);
			}
			default:
				return 'JUGAR';
		}
	};

	const getButtonClass = () => {
		const baseClass = "text-white font-bold text-xl px-16 py-8 rounded-2xl shadow-2xl transform transition-all duration-500 ease-out text-center relative overflow-hidden min-w-[16rem] hover:scale-105";
		const currentCachedState = launchStateCache.get(instanceId);
		const currentState = currentCachedState?.state || state;
		if (currentState === 'playing' || currentState === 'launching') {
			return `${baseClass} bg-gradient-to-r from-[#00ffff]/20 via-[#00d4ff]/20 to-[#00ffff]/20 hover:from-[#00ffff]/30 hover:via-[#00d4ff]/30 hover:to-[#00ffff]/30 neon-glow-cyan`;
		}
		return `${baseClass} bg-gradient-to-r from-[#00ffff]/10 via-[#ff00ff]/10 to-[#00ffff]/10 hover:from-[#00ffff]/20 hover:via-[#ff00ff]/20 hover:to-[#00ffff]/20 neon-glow-cyan-hover`;
	};

	const getButtonStyle = (isHovered: boolean = false) => {
		const currentCachedState = launchStateCache.get(instanceId);
		const currentState = currentCachedState?.state || state;
		if (currentState === 'playing' || currentState === 'launching') {
			return {
				background: 'rgba(0, 0, 0, 0.6)',
				backdropFilter: 'blur(24px)',
				WebkitBackdropFilter: 'blur(24px)',
				boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.7)',
				border: '1px solid',
				borderColor: isHovered ? 'rgba(0, 255, 255, 0.8)' : 'rgba(0, 255, 255, 0.5)',
				transition: 'all 0.5s cubic-bezier(0.4, 0, 0.2, 1)'
			};
		}
		return {
			background: 'rgba(0, 0, 0, 0.6)',
			backdropFilter: 'blur(24px)',
			WebkitBackdropFilter: 'blur(24px)',
			boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.7)',
			border: '1px solid',
			borderColor: isHovered ? 'rgba(0, 255, 255, 0.7)' : 'rgba(0, 255, 255, 0.4)',
			transition: 'all 0.5s cubic-bezier(0.4, 0, 0.2, 1)'
		};
	};

	// Obtener el estado actual del caché para esta instancia específica
	const currentCachedState = launchStateCache.get(instanceId);
	const displayState = currentCachedState?.state || state;
	// El botón solo debe estar deshabilitado si ESTA instancia específica no está en 'idle'
	const isButtonDisabled = disabled || displayState !== 'idle' || isJavaInstalling;
	
	return (
		<div className="relative">
			<Button
				onClick={handleClick}
				disabled={isButtonDisabled}
				className={`${getButtonClass()} ${className}`}
				style={getButtonStyle(isHovered)}
				onMouseEnter={() => setIsHovered(true)}
				onMouseLeave={() => setIsHovered(false)}
			>
				{(displayState === 'launching' || displayState === 'playing') && (
					<div className="absolute inset-0 flex items-center justify-center">
						<div className="marquee-container transition-all duration-500">
							<div className="marquee-text">
								{formatTimeForMarquee(playTime).split('').map((letter, index) => (
									<span
										key={index}
										className="marquee-letter transition-all duration-500"
										style={{
											animationDelay: `${(2.5 / formatTimeForMarquee(playTime).length) * (formatTimeForMarquee(playTime).length - 1 - index) * -1}s`
										}}
									>
										{letter === ' ' ? '\u00A0' : letter}
									</span>
								))}
							</div>
						</div>
					</div>
				)}

				<span className="relative z-10">
					{displayState === 'launching' || displayState === 'playing' ? '' : getButtonContent()}
				</span>
			</Button>
		</div>
	);
};

export default LaunchButton;

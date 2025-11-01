import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';

interface LaunchButtonProps {
	onLaunch: () => Promise<void>;
	disabled?: boolean;
	className?: string;
	isJavaInstalling?: boolean;
	instanceId?: string; // Añadir instanceId para identificar la instancia
}

type LaunchState = 'idle' | 'launching' | 'playing';

// Caché global para estados de lanzamiento por instancia
const launchStateCache = new Map<string, { state: LaunchState; playTime: number; startTime: number }>();

const LaunchButton: React.FC<LaunchButtonProps> = ({
	onLaunch,
	disabled = false,
	className = '',
	isJavaInstalling = false,
	instanceId = 'default'
}) => {
	// Inicializar con estado en caché si existe
	const cachedState = launchStateCache.get(instanceId);
	const [state, setState] = useState<LaunchState>(cachedState?.state || 'idle');
	const [playTime, setPlayTime] = useState(() => {
		if (cachedState && cachedState.state === 'playing') {
			// Calcular tiempo transcurrido desde el inicio
			const elapsed = Math.floor((Date.now() - cachedState.startTime) / 1000);
			return cachedState.playTime + elapsed;
		}
		return cachedState?.playTime || 0;
	});
	const startTimeRef = useRef<number>(cachedState?.startTime || Date.now());

	useEffect(() => {
		let interval: NodeJS.Timeout | null = null;
		if (state === 'playing') {
			interval = setInterval(() => {
				setPlayTime(prev => {
					const newTime = prev + 1;
					// Actualizar caché
					if (launchStateCache.has(instanceId)) {
						const cached = launchStateCache.get(instanceId)!;
						launchStateCache.set(instanceId, { 
							...cached, 
							playTime: newTime,
							startTime: startTimeRef.current
						});
					}
					return newTime;
				});
			}, 1000);
		}
		return () => { if (interval) clearInterval(interval); };
	}, [state, instanceId]);

	// Reset when the Minecraft process exits
	useEffect(() => {
		let unlisten: (() => void) | undefined;
		listen('minecraft_exited', () => {
			setState('idle');
			setPlayTime(0);
			// Limpiar caché cuando el juego termina
			launchStateCache.delete(instanceId);
			startTimeRef.current = Date.now();
		}).then((fn) => { unlisten = fn; }).catch(() => {});
		return () => { if (unlisten) { try { unlisten(); } catch {} } };
	}, [instanceId]);

	const handleClick = async () => {
		if (disabled || state !== 'idle' || isJavaInstalling) return;
		setState('launching');
		launchStateCache.set(instanceId, { state: 'launching', playTime: 0, startTime: Date.now() });
		try {
			await onLaunch();
			startTimeRef.current = Date.now();
			setState('playing');
			setPlayTime(0);
			launchStateCache.set(instanceId, { state: 'playing', playTime: 0, startTime: startTimeRef.current });
		} catch (error) {
			console.error('Error during launch:', error);
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
		const baseClass = "bg-gradient-to-r from-blue-600 via-blue-700 to-indigo-800 hover:from-blue-700 hover:via-blue-800 hover:to-indigo-900 text-white font-bold text-xl px-16 py-8 rounded-full shadow-2xl transform transition-all duration-500 border-2 text-center relative overflow-hidden min-w-[16rem]";
		if (state === 'playing' || state === 'launching') {
			return `${baseClass} border-green-400/50 hover:border-green-300/70 bg-gradient-to-r from-green-600 via-green-700 to-emerald-800 hover:from-green-700 hover:via-green-800 hover:to-emerald-900`;
		}
		return `${baseClass} border-blue-400/30 hover:border-blue-300/50 ${state === 'idle' ? 'hover:scale-105' : ''}`;
	};

	return (
		<div className="relative">
			<Button
				onClick={handleClick}
				disabled={disabled || state !== 'idle' || isJavaInstalling}
				className={`${getButtonClass()} ${className}`}
			>
				{(state === 'launching' || state === 'playing') && (
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
					{state === 'launching' || state === 'playing' ? '' : getButtonContent()}
				</span>
			</Button>
		</div>
	);
};

export default LaunchButton;
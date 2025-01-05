// src/lib/utils/logger.ts

/**
 * Configuration type for logger
 */
type LogConfig = {
	level: 'debug' | 'info' | 'warn' | 'error';
	enabled: boolean;
};

/**
 * Logger configuration based on environment
 * In SvelteKit, we can use import.meta.env.DEV to check if we're in development
 */
const config: LogConfig = {
	level: 'debug',
	enabled: import.meta.env.DEV
};

/**
 * Main logger function that handles all log types
 */
function createLogger(type: 'log' | 'info' | 'warn' | 'error' | 'debug') {
	return (...args: any[]) => {
		if (!config.enabled) return;

		const timestamp = new Date().toISOString();
		const prefix = `[${timestamp}]`;

		switch (type) {
			case 'debug':
				console.debug(prefix, ...args);
				break;
			case 'info':
				console.info(prefix, ...args);
				break;
			case 'warn':
				console.warn(prefix, ...args);
				break;
			case 'error':
				console.error(prefix, ...args);
				break;
			default:
				console.log(prefix, ...args);
		}
	};
}

/**
 * Export individual logging functions
 */
export const log = createLogger('log');
export const info = createLogger('info');
export const warn = createLogger('warn');
export const error = createLogger('error');
export const debug = createLogger('debug');

/**
 * Enable/disable logging programmatically
 */
export function enableLogging(enabled: boolean = true) {
	config.enabled = enabled;
}

/**
 * Set logging level
 */
export function setLogLevel(level: LogConfig['level']) {
	config.level = level;
}

// Additional utility for group logging
export const group = (name: string, fn: () => void) => {
	if (!config.enabled) return;
	console.group(name);
	fn();
	console.groupEnd();
};

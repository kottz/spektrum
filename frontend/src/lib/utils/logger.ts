/** Development-only console logging; silent in production builds. */
const enabled = import.meta.env.DEV;

function createLogger(write: (...args: unknown[]) => void) {
	return (...args: unknown[]) => {
		if (!enabled) return;
		write(`[${new Date().toISOString()}]`, ...args);
	};
}

export const info = createLogger((...args) => console.info(...args));
export const warn = createLogger((...args) => console.warn(...args));

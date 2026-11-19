import { defineEnvVars } from '@sveltejs/kit/env';

export const variables = defineEnvVars({
	PUBLIC_SPEKTRUM_SERVER_URL: {
		public: true,
		static: true,
		description: 'Base URL of the Spektrum server HTTP API'
	},
	PUBLIC_SPEKTRUM_CDN_URL: {
		public: true,
		static: true,
		description: 'Base URL for media files; empty to serve media from the server'
	},
	PUBLIC_DEV_ADMIN_PASSWORD: {
		public: true,
		// Inlined at build time so the dev-only usage is stripped from production bundles
		static: true,
		schema: (value) => value?.trim() || undefined,
		description: 'Auto-login password for local development; ignored in production builds'
	}
});

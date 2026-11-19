import { defineEnvVars } from '@sveltejs/kit/env';

/** Treats unset and blank values alike, so optional features can be switched off with `VAR=` */
const optional = (value: string | undefined) => value?.trim() || undefined;

export const variables = defineEnvVars({
	PUBLIC_SPEKTRUM_SERVER_URL: {
		public: true,
		static: true,
		description: 'Base URL of the Spektrum server HTTP API'
	},
	PUBLIC_SPEKTRUM_WS_SERVER_URL: {
		public: true,
		static: true,
		description: 'WebSocket endpoint of the Spektrum server'
	},
	PUBLIC_SPEKTRUM_CDN_URL: {
		public: true,
		static: true,
		description: 'Base URL for media files; empty to serve media from the server'
	},
	PUBLIC_TITLE: {
		public: true,
		static: true,
		description: 'Application title displayed in the UI'
	},
	PUBLIC_UMAMI_WEBSITE_ID: {
		public: true,
		schema: optional,
		description: 'Umami analytics website ID (optional)'
	},
	PUBLIC_SUPPORT_EMAIL: {
		public: true,
		schema: optional,
		description: 'Support email shown in the footer (optional)'
	}
});

import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import { defineConfig } from 'eslint/config';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import { fileURLToPath } from 'node:url';
import ts from 'typescript-eslint';
const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));

export default defineConfig(
	includeIgnoreFile(gitignorePath),
	js.configs.recommended,
	ts.configs.recommended,
	svelte.configs.recommended,
	prettier,
	svelte.configs.prettier,
	{
		languageOptions: {
			globals: {
				...globals.browser,
				...globals.node,
				YT: 'readonly'
			}
		}
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts'],

		languageOptions: {
			parserOptions: {
				parser: ts.parser
			}
		}
	},
	{
		rules: {
			'@typescript-eslint/no-unused-vars': [
				'error',
				{ argsIgnorePattern: '^_', varsIgnorePattern: '^_' }
			]
		}
	},
	{
		files: ['src/lib/components/ui/**/*.svelte'],
		rules: {
			// UI wrapper components use ...restProps by design; they are not custom elements
			'svelte/valid-compile': ['error', { ignoreWarnings: true }],
			// UI wrappers forward an `href` their callers have already resolved
			'svelte/no-navigation-without-resolve': 'off'
		}
	}
);

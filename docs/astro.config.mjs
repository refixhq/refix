// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://refix.dev',
	integrations: [
		starlight({
			title: 'ReFIX',
			logo: {
				light: './src/assets/mark-light.svg',
				dark: './src/assets/mark-dark.svg',
			},
			customCss: ['./src/styles/custom.css'],
			expressiveCode: { styleOverrides: { borderRadius: '0.5rem' } },
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/refixhq/refix' }],
			sidebar: [
				{ label: 'Quick start', slug: 'quick-start' },
				{
					label: 'Guides',
					items: [
						{ label: 'Reading logs', slug: 'guides/reading-logs' },
						{ label: 'Raw messages', slug: 'guides/raw-messages' },
						{ label: 'Typed messages', slug: 'guides/typed-messages' },
					],
				},
			],
		}),
	],
});

// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://refix.dev',
	integrations: [
		starlight({
			title: 'ReFIX',
			logo: {
				light: './src/assets/logo-light.svg',
				dark: './src/assets/logo-dark.svg',
				replacesTitle: true,
			},
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/refixhq/refix' }],
			sidebar: [{ label: 'Quick start', slug: 'quick-start' }],
		}),
	],
});

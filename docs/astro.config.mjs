// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://refix.dev',
	integrations: [
		starlight({
			title: 'ReFIX',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/refixhq/refix' }],
		}),
	],
});

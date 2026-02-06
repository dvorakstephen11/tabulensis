import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config';

export default defineWorkersConfig({
	test: {
		poolOptions: {
			workers: {
				// Keep tests self-contained: don't require Cloudflare login or remote proxy sessions.
				remoteBindings: false,
				wrangler: { configPath: './wrangler.jsonc' },
			},
		},
	},
});

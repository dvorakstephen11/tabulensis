import { env, createExecutionContext, waitOnExecutionContext } from 'cloudflare:test';
import { describe, it, expect } from 'vitest';
import worker from '../src/index';

// For now, you'll need to do something like this to get a correctly-typed
// `Request` to pass to `worker.fetch()`.
const IncomingRequest = Request<unknown, IncomingRequestCfProperties>;

describe('tabulensis-api licensing worker', () => {
	const SEED_B64 = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=';

	function testEnv() {
		return {
			...env,
			LICENSE_MOCK_STRIPE: '1',
			LICENSE_SIGNING_KEY_B64: SEED_B64,
			LICENSE_MAX_DEVICES: '2',
			STRIPE_TRIAL_DAYS: '30',
			STRIPE_SUCCESS_URL: 'https://tabulensis.com/download/success',
			STRIPE_CANCEL_URL: 'https://tabulensis.com/download',
			STRIPE_PORTAL_RETURN_URL: 'https://tabulensis.com/support/billing',
		};
	}

	it('health is ok', async () => {
		const request = new IncomingRequest('http://example.com/health', { method: 'GET' });
		const ctx = createExecutionContext();
		const response = await worker.fetch(request, testEnv(), ctx);
		await waitOnExecutionContext(ctx);
		expect(response.status).toBe(200);
		expect(await response.json()).toEqual({ status: 'ok' });
	});

	it('d1 supports DDL in tests', async () => {
		const e = testEnv() as any;
		await e.DB.prepare('CREATE TABLE IF NOT EXISTS ddl_test (id INTEGER PRIMARY KEY)').run();
		await e.DB.prepare('INSERT INTO ddl_test (id) VALUES (1)').run();
		const row = await e.DB.prepare('SELECT COUNT(*) as count FROM ddl_test').first();
		expect(Number(row?.count ?? 0)).toBeGreaterThanOrEqual(1);
	});

	it('mock checkout issues a license key and allows activation', async () => {
		const ctx = createExecutionContext();
		const startReq = new IncomingRequest('http://example.com/api/checkout/start', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ email: 'user@example.com' }),
		});
		const startRes = await worker.fetch(startReq, testEnv(), ctx);
		expect(startRes.status).toBe(200);
		const startData = (await startRes.json()) as any;
		expect(startData.license_key).toMatch(/^TABU-[A-Z2-7]{4}-[A-Z2-7]{4}-[A-Z2-7]{4}$/);
		expect(startData.session_id).toMatch(/^mock_/);
		expect(startData.checkout_url).toContain('license_key=');

		const statusReq = new IncomingRequest(
			`http://example.com/api/checkout/status?session_id=${encodeURIComponent(startData.session_id)}`,
			{ method: 'GET' },
		);
		const statusRes = await worker.fetch(statusReq, testEnv(), ctx);
		expect(statusRes.status).toBe(200);
		const statusData = (await statusRes.json()) as any;
		expect(statusData.license_key).toBe(startData.license_key);
		expect(statusData.status).toBe('trialing');

		const pkReq = new IncomingRequest('http://example.com/public_key', { method: 'GET' });
		const pkRes = await worker.fetch(pkReq, testEnv(), ctx);
		expect(pkRes.status).toBe(200);
		const pkData = (await pkRes.json()) as any;
		expect(pkData.public_key_b64).toMatch(/^[A-Za-z0-9+/]+=*$/);

		const activateReq = new IncomingRequest('http://example.com/license/activate', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				license_key: startData.license_key,
				device_id: 'device-hash-1',
				device_label: 'test-device',
			}),
		});
		const activateRes = await worker.fetch(activateReq, testEnv(), ctx);
		expect(activateRes.status).toBe(200);
		const activateData = (await activateRes.json()) as any;
		expect(activateData.token?.signature).toMatch(/^[A-Za-z0-9+/]+=*$/);
		expect(activateData.status?.status).toBe('trialing');
		expect(activateData.status?.activations?.length).toBe(1);

		await waitOnExecutionContext(ctx);
	});

	it('enforces device limit and allows reuse after deactivation', async () => {
		const ctx = createExecutionContext();
		const startReq = new IncomingRequest('http://example.com/api/checkout/start', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ email: 'user2@example.com' }),
		});
		const startRes = await worker.fetch(startReq, testEnv(), ctx);
		const startData = (await startRes.json()) as any;

		for (const device of ['device-a', 'device-b']) {
			const res = await worker.fetch(
				new IncomingRequest('http://example.com/license/activate', {
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ license_key: startData.license_key, device_id: device }),
				}),
				testEnv(),
				ctx,
			);
			expect(res.status).toBe(200);
		}

		const third = await worker.fetch(
			new IncomingRequest('http://example.com/license/activate', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ license_key: startData.license_key, device_id: 'device-c' }),
			}),
			testEnv(),
			ctx,
		);
		expect(third.status).toBe(403);

		const deact = await worker.fetch(
			new IncomingRequest('http://example.com/license/deactivate', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ license_key: startData.license_key, device_id: 'device-a' }),
			}),
			testEnv(),
			ctx,
		);
		expect(deact.status).toBe(200);

		const after = await worker.fetch(
			new IncomingRequest('http://example.com/license/activate', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ license_key: startData.license_key, device_id: 'device-c' }),
			}),
			testEnv(),
			ctx,
		);
		expect(after.status).toBe(200);

		await waitOnExecutionContext(ctx);
	});
});

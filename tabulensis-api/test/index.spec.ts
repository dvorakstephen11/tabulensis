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
			DOWNLOAD_BUCKET: undefined,
			LICENSE_MOCK_STRIPE: '1',
			LICENSE_SIGNING_KEY_B64: SEED_B64,
			LICENSE_MAX_DEVICES: '2',
			RESEND_API_KEY: 're_test_key_123',
			RESEND_FROM: 'Tabulensis <licenses@mail.tabulensis.com>',
			RESEND_REPLY_TO: 'support@tabulensis.com',
			STRIPE_TRIAL_DAYS: '30',
			STRIPE_SUCCESS_URL: 'https://tabulensis.com/download/success',
			STRIPE_CANCEL_URL: 'https://tabulensis.com/download',
			STRIPE_PORTAL_RETURN_URL: 'https://tabulensis.com/support/billing',
			DOWNLOAD_ORIGIN_BASE_URL: 'https://downloads.example.test/releases/latest/download/',
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

		it('license resend sends email (by license_key)', async () => {
			const realFetch = globalThis.fetch;
			const calls: Array<{ url: string; init?: RequestInit }> = [];
			(globalThis as any).fetch = async (input: any, init?: any) => {
				const url = typeof input === 'string' ? input : String(input?.url ?? '');
				if (url === 'https://api.resend.com/emails') {
					calls.push({ url, init });
					return new Response(JSON.stringify({ id: 'email_test_123' }), {
						status: 200,
						headers: { 'content-type': 'application/json' },
					});
				}
				return realFetch(input, init);
			};

			try {
				const ctx = createExecutionContext();
				const startReq = new IncomingRequest('http://example.com/api/checkout/start', {
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ email: 'user3@example.com' }),
				});
				const startRes = await worker.fetch(startReq, testEnv(), ctx);
				expect(startRes.status).toBe(200);
				const startData = (await startRes.json()) as any;

				const resendReq = new IncomingRequest('http://example.com/license/resend', {
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ license_key: startData.license_key }),
				});
				const resendRes = await worker.fetch(resendReq, testEnv(), ctx);
				expect(resendRes.status).toBe(200);
				const resendData = (await resendRes.json()) as any;
				expect(resendData.status).toBe('sent');
				expect(resendData.id).toBe('email_test_123');

				await waitOnExecutionContext(ctx);
				expect(calls.length).toBe(1);

				const sent = calls[0];
				const headers = new Headers(sent.init?.headers);
				expect(headers.get('Authorization')).toBe('Bearer re_test_key_123');
				expect(headers.get('Idempotency-Key')).toBe(null);

				const payload = JSON.parse(String(sent.init?.body ?? '')) as any;
				expect(payload.to).toBe('user3@example.com');
				expect(payload.from).toBe('Tabulensis <licenses@mail.tabulensis.com>');
				expect(payload.reply_to).toBe('support@tabulensis.com');
				expect(payload.subject).toBe('Your Tabulensis license key');
				expect(payload.text).toContain(String(startData.license_key));
				expect(payload.html).toContain(String(startData.license_key));
			} finally {
				(globalThis as any).fetch = realFetch;
			}
		});

		it('checkout.session.completed webhook triggers a license email with an idempotency key', async () => {
			const realFetch = globalThis.fetch;
			const calls: Array<{ url: string; init?: RequestInit }> = [];
			(globalThis as any).fetch = async (input: any, init?: any) => {
				const url = typeof input === 'string' ? input : String(input?.url ?? '');
				if (url === 'https://api.resend.com/emails') {
					calls.push({ url, init });
					return new Response(JSON.stringify({ id: 'email_webhook_1' }), {
						status: 200,
						headers: { 'content-type': 'application/json' },
					});
				}
				return realFetch(input, init);
			};

			try {
				const ctx = createExecutionContext();
				const eventId = 'evt_test_123';
				const licenseKey = 'TABU-AAAA-BBBB-CCCC';
				const webhookReq = new IncomingRequest('http://example.com/stripe/webhook', {
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						id: eventId,
						type: 'checkout.session.completed',
						created: 123,
						data: {
							object: {
								id: 'cs_test_123',
								customer: 'cus_test_123',
								subscription: 'sub_test_123',
								customer_details: { email: 'buyer@example.com' },
								metadata: { license_key: licenseKey },
							},
						},
					}),
				});

				const webhookRes = await worker.fetch(webhookReq, testEnv(), ctx);
				expect(webhookRes.status).toBe(200);
				await waitOnExecutionContext(ctx);

				expect(calls.length).toBe(1);
				const headers = new Headers(calls[0].init?.headers);
				expect(headers.get('Idempotency-Key')).toBe(`license-email/stripe-event/${eventId}`);

				const payload = JSON.parse(String(calls[0].init?.body ?? '')) as any;
				expect(payload.to).toBe('buyer@example.com');
				expect(payload.text).toContain(licenseKey);
			} finally {
				(globalThis as any).fetch = realFetch;
			}
		});

		it('download proxy serves allowed assets', async () => {
			const realFetch = globalThis.fetch;
			(globalThis as any).fetch = async (input: any, init?: any) => {
				const url = typeof input === 'string' ? input : String(input?.url ?? '');
				if (
					url ===
					'https://downloads.example.test/releases/latest/download/tabulensis-latest-windows-x86_64.exe'
				) {
					return new Response('binary-data', {
						status: 200,
						headers: {
							'content-type': 'application/octet-stream',
							'content-disposition': 'attachment; filename="tabulensis-latest-windows-x86_64.exe"',
						},
					});
				}
				return realFetch(input, init);
			};

			try {
				const ctx = createExecutionContext();
				const req = new IncomingRequest(
					'http://example.com/download/tabulensis-latest-windows-x86_64.exe',
					{ method: 'GET' },
				);
				const res = await worker.fetch(req, testEnv(), ctx);
				expect(res.status).toBe(200);
				expect(res.headers.get('content-type')).toBe('application/octet-stream');
				expect(res.headers.get('cache-control')).toBe('public, max-age=300');
				expect(await res.text()).toBe('binary-data');
				await waitOnExecutionContext(ctx);
			} finally {
				(globalThis as any).fetch = realFetch;
			}
		});

		it('download proxy rejects unknown assets', async () => {
			const ctx = createExecutionContext();
			const req = new IncomingRequest('http://example.com/download/nope.exe', { method: 'GET' });
			const res = await worker.fetch(req, testEnv(), ctx);
			expect(res.status).toBe(404);
			await waitOnExecutionContext(ctx);
		});

		it('download proxy masks upstream 404s (no GitHub HTML leakage)', async () => {
			const realFetch = globalThis.fetch;
			(globalThis as any).fetch = async (input: any, init?: any) => {
				const url = typeof input === 'string' ? input : String(input?.url ?? '');
				if (
					url ===
					'https://downloads.example.test/releases/latest/download/tabulensis-latest-windows-x86_64.exe'
				) {
					return new Response('<html>github 404</html>', {
						status: 404,
						headers: {
							'content-type': 'text/html; charset=utf-8',
							'x-github-request-id': 'test',
						},
					});
				}
				return realFetch(input, init);
			};

			try {
				const ctx = createExecutionContext();
				const req = new IncomingRequest(
					'http://example.com/download/tabulensis-latest-windows-x86_64.exe',
					{ method: 'GET' },
				);
				const res = await worker.fetch(req, testEnv(), ctx);
				expect(res.status).toBe(404);
				expect(res.headers.get('cache-control')).toBe('no-store');
				expect(res.headers.get('x-github-request-id')).toBe(null);
				expect(await res.text()).toBe('Not Found');
				await waitOnExecutionContext(ctx);
			} finally {
				(globalThis as any).fetch = realFetch;
			}
		});
	});

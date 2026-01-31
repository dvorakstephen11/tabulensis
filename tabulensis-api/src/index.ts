import {
	createHash,
	createHmac,
	createPrivateKey,
	randomBytes,
	randomUUID,
	sign as cryptoSign,
	timingSafeEqual,
} from 'node:crypto';

type LicenseStatus = 'trialing' | 'active' | 'past_due' | 'canceled' | 'revoked';

type Env = {
	DB: D1Database;
	STRIPE_SECRET_KEY: string;
	STRIPE_WEBHOOK_SECRET: string;
	STRIPE_PRICE_ID_YEARLY: string;
	APP_ORIGIN: string;
	LICENSE_SIGNING_PRIVATE_KEY: string;
	LICENSE_SIGNING_KEY_ID?: string;
	LICENSE_TOKEN_TTL_SECONDS?: string;
};

const JSON_HEADERS = {
	'content-type': 'application/json',
};

const STRIPE_API_BASE = 'https://api.stripe.com/v1';
const WEBHOOK_TOLERANCE_SECONDS = 300;

function jsonResponse(data: unknown, status = 200): Response {
	return new Response(JSON.stringify(data), { status, headers: JSON_HEADERS });
}

function errorResponse(message: string, status = 400): Response {
	return jsonResponse({ error: message }, status);
}

async function readJson<T>(request: Request): Promise<T | null> {
	try {
		return (await request.json()) as T;
	} catch {
		return null;
	}
}

function base64UrlEncode(data: Uint8Array): string {
	return Buffer.from(data)
		.toString('base64')
		.replace(/=/g, '')
		.replace(/\+/g, '-')
		.replace(/\//g, '_');
}

function sha256Hex(value: string): string {
	return createHash('sha256').update(value).digest('hex');
}

function generateLicenseKey(): string {
	const raw = randomBytes(16).toString('hex');
	return `TAB-${raw}`;
}

function parseStripeSignature(signatureHeader: string | null): { timestamp: number; signatures: string[] } | null {
	if (!signatureHeader) return null;
	const parts = signatureHeader.split(',').map((part) => part.trim());
	let timestamp: number | null = null;
	const signatures: string[] = [];
	for (const part of parts) {
		const [key, value] = part.split('=');
		if (!key || !value) continue;
		if (key === 't') {
			const parsed = Number(value);
			if (!Number.isFinite(parsed)) return null;
			timestamp = parsed;
		}
		if (key === 'v1') signatures.push(value);
	}
	if (!timestamp || signatures.length === 0) return null;
	return { timestamp, signatures };
}

function verifyStripeWebhook(rawBody: string, signatureHeader: string | null, secret: string): boolean {
	const parsed = parseStripeSignature(signatureHeader);
	if (!parsed) return false;
	const nowSeconds = Math.floor(Date.now() / 1000);
	if (Math.abs(nowSeconds - parsed.timestamp) > WEBHOOK_TOLERANCE_SECONDS) return false;
	const signedPayload = `${parsed.timestamp}.${rawBody}`;
	const expected = createHmac('sha256', secret).update(signedPayload, 'utf8').digest('hex');
	const expectedBuf = Buffer.from(expected, 'hex');
	for (const signature of parsed.signatures) {
		const signatureBuf = Buffer.from(signature, 'hex');
		if (signatureBuf.length === expectedBuf.length && timingSafeEqual(signatureBuf, expectedBuf)) {
			return true;
		}
	}
	return false;
}

async function stripeRequest(
	env: Env,
	method: 'GET' | 'POST',
	path: string,
	body?: URLSearchParams,
): Promise<any> {
	const url = `${STRIPE_API_BASE}${path}`;
	const response = await fetch(url, {
		method,
		headers: {
			Authorization: `Bearer ${env.STRIPE_SECRET_KEY}`,
			...(body ? { 'content-type': 'application/x-www-form-urlencoded' } : {}),
		},
		body: body ? body.toString() : undefined,
	});
	const data = await response.json();
	if (!response.ok) {
		const message = data?.error?.message ?? 'Stripe request failed';
		throw new Error(message);
	}
	return data;
}

async function createCheckoutSession(env: Env, email?: string): Promise<any> {
	const params = new URLSearchParams({
		mode: 'subscription',
		'line_items[0][price]': env.STRIPE_PRICE_ID_YEARLY,
		'line_items[0][quantity]': '1',
		'subscription_data[trial_period_days]': '30',
		payment_method_collection: 'always',
		success_url: `${env.APP_ORIGIN}/success?session_id={CHECKOUT_SESSION_ID}`,
		cancel_url: `${env.APP_ORIGIN}/download?canceled=1`,
	});
	if (email) params.set('customer_email', email);
	return stripeRequest(env, 'POST', '/checkout/sessions', params);
}

async function getCheckoutSession(env: Env, sessionId: string): Promise<any> {
	return stripeRequest(env, 'GET', `/checkout/sessions/${encodeURIComponent(sessionId)}`);
}

async function createPortalSession(env: Env, customerId: string): Promise<any> {
	const params = new URLSearchParams({
		customer: customerId,
		return_url: `${env.APP_ORIGIN}/account`,
	});
	return stripeRequest(env, 'POST', '/billing_portal/sessions', params);
}

async function getSubscription(env: Env, subscriptionId: string): Promise<any> {
	return stripeRequest(env, 'GET', `/subscriptions/${encodeURIComponent(subscriptionId)}`);
}

function issueToken(env: Env, payload: Record<string, unknown>): string {
	const key = createPrivateKey(env.LICENSE_SIGNING_PRIVATE_KEY);
	const alg = key.asymmetricKeyType === 'ed25519' || key.asymmetricKeyType === 'ed448' ? 'EdDSA' : 'RS256';
	const header: Record<string, string> = { alg, typ: 'JWT' };
	if (env.LICENSE_SIGNING_KEY_ID) header.kid = env.LICENSE_SIGNING_KEY_ID;
	const headerB64 = base64UrlEncode(Buffer.from(JSON.stringify(header)));
	const payloadB64 = base64UrlEncode(Buffer.from(JSON.stringify(payload)));
	const signingInput = `${headerB64}.${payloadB64}`;
	const signature = cryptoSign(alg === 'EdDSA' ? null : 'sha256', Buffer.from(signingInput), key);
	const signatureB64 = base64UrlEncode(signature);
	return `${signingInput}.${signatureB64}`;
}

async function handleCheckoutStart(request: Request, env: Env): Promise<Response> {
	const body = await readJson<{ email?: string }>(request);
	try {
		const session = await createCheckoutSession(env, body?.email);
		return jsonResponse({ id: session.id, url: session.url });
	} catch (err) {
		return errorResponse((err as Error).message, 502);
	}
}

async function handleCheckoutSession(request: Request, env: Env): Promise<Response> {
	const url = new URL(request.url);
	const sessionId = url.searchParams.get('session_id');
	if (!sessionId) return errorResponse('session_id is required');
	try {
		const session = await getCheckoutSession(env, sessionId);
		return jsonResponse({
			id: session.id,
			status: session.status,
			customer: session.customer,
			subscription: session.subscription,
			customer_email: session.customer_details?.email ?? session.customer_email,
		});
	} catch (err) {
		return errorResponse((err as Error).message, 502);
	}
}

async function handlePortalSession(request: Request, env: Env): Promise<Response> {
	const body = await readJson<{ customer_id?: string; license_key?: string }>(request);
	if (!body?.customer_id && !body?.license_key) return errorResponse('customer_id or license_key is required');
	let customerId = body.customer_id;
	if (!customerId && body.license_key) {
		const keyHash = sha256Hex(body.license_key);
		const row = await env.DB.prepare(
			`SELECT l.stripe_customer_id as stripe_customer_id
       FROM licenses l
       JOIN license_keys lk ON lk.license_id = l.id
       WHERE lk.key_hash = ?`,
		)
			.bind(keyHash)
			.first();
		customerId = row?.stripe_customer_id ?? null;
	}
	if (!customerId) return errorResponse('customer not found', 404);
	try {
		const session = await createPortalSession(env, customerId);
		return jsonResponse({ url: session.url });
	} catch (err) {
		return errorResponse((err as Error).message, 502);
	}
}

async function markEventProcessed(env: Env, event: any): Promise<boolean> {
	const now = Math.floor(Date.now() / 1000);
	try {
		await env.DB.prepare(
			'INSERT INTO stripe_events (id, type, created_at, processed_at) VALUES (?, ?, ?, ?)',
		)
			.bind(event.id, event.type, event.created ?? now, now)
			.run();
		return true;
	} catch {
		return false;
	}
}

async function handleCheckoutCompleted(env: Env, session: any): Promise<void> {
	const subscriptionId = session.subscription as string | undefined;
	if (!subscriptionId) return;
	const existing = await env.DB.prepare('SELECT id FROM licenses WHERE stripe_subscription_id = ?')
		.bind(subscriptionId)
		.first();
	if (existing?.id) return;

	const now = Math.floor(Date.now() / 1000);
	const licenseId = randomUUID();
	let trialEnd: number | null = null;
	let currentPeriodEnd: number | null = null;
	try {
		const subscription = await getSubscription(env, subscriptionId);
		trialEnd = subscription.trial_end ?? null;
		currentPeriodEnd = subscription.current_period_end ?? null;
	} catch {
		trialEnd = null;
		currentPeriodEnd = null;
	}

	await env.DB.prepare(
		`INSERT INTO licenses (
        id, created_at, updated_at, status, stripe_customer_id, stripe_subscription_id, trial_end, current_period_end, max_devices
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`
	)
		.bind(
			licenseId,
			now,
			now,
			'trialing',
			session.customer ?? null,
			subscriptionId,
			trialEnd,
			currentPeriodEnd,
			2,
		)
		.run();

	const licenseKey = generateLicenseKey();
	const keyHash = sha256Hex(licenseKey);
	await env.DB.prepare('INSERT INTO license_keys (license_id, key_hash, created_at) VALUES (?, ?, ?)')
		.bind(licenseId, keyHash, now)
		.run();
}

async function updateLicenseStatus(
	env: Env,
	subscriptionId: string,
	status: LicenseStatus,
	trialEnd?: number | null,
	currentPeriodEnd?: number | null,
): Promise<void> {
	const now = Math.floor(Date.now() / 1000);
	await env.DB.prepare(
		`UPDATE licenses
       SET status = ?, updated_at = ?, trial_end = COALESCE(?, trial_end), current_period_end = COALESCE(?, current_period_end)
       WHERE stripe_subscription_id = ?`,
	)
		.bind(status, now, trialEnd ?? null, currentPeriodEnd ?? null, subscriptionId)
		.run();
}

async function handleWebhook(request: Request, env: Env): Promise<Response> {
	const rawBody = await request.text();
	const signature = request.headers.get('Stripe-Signature');
	if (!verifyStripeWebhook(rawBody, signature, env.STRIPE_WEBHOOK_SECRET)) {
		return errorResponse('invalid signature', 400);
	}

	let event: any;
	try {
		event = JSON.parse(rawBody);
	} catch {
		return errorResponse('invalid payload', 400);
	}

	const inserted = await markEventProcessed(env, event);
	if (!inserted) return jsonResponse({ received: true });

	const data = event.data?.object;
	if (!data) return jsonResponse({ received: true });

	switch (event.type) {
		case 'checkout.session.completed':
			await handleCheckoutCompleted(env, data);
			break;
		case 'invoice.paid': {
			const subscriptionId = data.subscription as string | undefined;
			const periodEnd = data.lines?.data?.[0]?.period?.end ?? null;
			if (subscriptionId) await updateLicenseStatus(env, subscriptionId, 'active', null, periodEnd);
			break;
		}
		case 'invoice.payment_failed': {
			const subscriptionId = data.subscription as string | undefined;
			if (subscriptionId) await updateLicenseStatus(env, subscriptionId, 'past_due');
			break;
		}
		case 'customer.subscription.updated': {
			const subscriptionId = data.id as string | undefined;
			if (subscriptionId)
				await updateLicenseStatus(
					env,
					subscriptionId,
					data.status as LicenseStatus,
					data.trial_end ?? null,
					data.current_period_end ?? null,
				);
			break;
		}
		case 'customer.subscription.deleted': {
			const subscriptionId = data.id as string | undefined;
			if (subscriptionId) await updateLicenseStatus(env, subscriptionId, 'canceled');
			break;
		}
		default:
			break;
	}

	return jsonResponse({ received: true });
}

async function handleActivate(request: Request, env: Env): Promise<Response> {
	const body = await readJson<{ license_key?: string; device_id?: string }>(request);
	if (!body?.license_key || !body.device_id) return errorResponse('license_key and device_id are required');

	const keyHash = sha256Hex(body.license_key);
	const deviceHash = sha256Hex(body.device_id);
	const license = await env.DB.prepare(
		`SELECT l.id as id, l.status as status, l.max_devices as max_devices
       FROM licenses l
       JOIN license_keys lk ON lk.license_id = l.id
       WHERE lk.key_hash = ?`,
	)
		.bind(keyHash)
		.first();
	if (!license) return errorResponse('license not found', 404);
	if (license.status !== 'trialing' && license.status !== 'active') {
		return errorResponse(`license status is ${license.status}`, 403);
	}

	const now = Math.floor(Date.now() / 1000);
	const existing = await env.DB.prepare(
		`SELECT id FROM activations WHERE license_id = ? AND device_id_hash = ? AND revoked_at IS NULL`,
	)
		.bind(license.id, deviceHash)
		.first();
	if (existing?.id) {
		await env.DB.prepare(`UPDATE activations SET last_seen_at = ? WHERE id = ?`)
			.bind(now, existing.id)
			.run();
	} else {
		const countRow = await env.DB.prepare(
			`SELECT COUNT(*) as count FROM activations WHERE license_id = ? AND revoked_at IS NULL`,
		)
			.bind(license.id)
			.first();
		const count = Number(countRow?.count ?? 0);
		if (count >= Number(license.max_devices)) {
			return errorResponse('device limit reached', 409);
		}
		const activationId = randomUUID();
		await env.DB.prepare(
			`INSERT INTO activations (id, license_id, device_id_hash, activated_at, last_seen_at)
         VALUES (?, ?, ?, ?, ?)`,
		)
			.bind(activationId, license.id, deviceHash, now, now)
			.run();
	}

	const ttl = Number(env.LICENSE_TOKEN_TTL_SECONDS ?? 1209600);
	const token = issueToken(env, {
		license_id: license.id,
		device_id_hash: deviceHash,
		status: license.status,
		iat: now,
		exp: now + ttl,
	});

	return jsonResponse({ token, expires_in: ttl });
}

async function handleDeactivate(request: Request, env: Env): Promise<Response> {
	const body = await readJson<{ license_key?: string; device_id?: string }>(request);
	if (!body?.license_key || !body.device_id) return errorResponse('license_key and device_id are required');

	const keyHash = sha256Hex(body.license_key);
	const deviceHash = sha256Hex(body.device_id);
	const license = await env.DB.prepare(
		`SELECT l.id as id
       FROM licenses l
       JOIN license_keys lk ON lk.license_id = l.id
       WHERE lk.key_hash = ?`,
	)
		.bind(keyHash)
		.first();
	if (!license) return errorResponse('license not found', 404);

	const now = Math.floor(Date.now() / 1000);
	await env.DB.prepare(
		`UPDATE activations SET revoked_at = ? WHERE license_id = ? AND device_id_hash = ? AND revoked_at IS NULL`,
	)
		.bind(now, license.id, deviceHash)
		.run();

	return jsonResponse({ ok: true });
}

export default {
	async fetch(request, env): Promise<Response> {
		const url = new URL(request.url);
		const { pathname } = url;

		if (pathname === '/api/stripe/checkout/start') {
			if (request.method !== 'POST') return errorResponse('method not allowed', 405);
			return handleCheckoutStart(request, env as Env);
		}

		if (pathname === '/api/stripe/checkout/session') {
			if (request.method !== 'GET') return errorResponse('method not allowed', 405);
			return handleCheckoutSession(request, env as Env);
		}

		if (pathname === '/api/stripe/customer-portal/session') {
			if (request.method !== 'POST') return errorResponse('method not allowed', 405);
			return handlePortalSession(request, env as Env);
		}

		if (pathname === '/api/stripe/webhook') {
			if (request.method !== 'POST') return errorResponse('method not allowed', 405);
			return handleWebhook(request, env as Env);
		}

		if (pathname === '/api/license/activate') {
			if (request.method !== 'POST') return errorResponse('method not allowed', 405);
			return handleActivate(request, env as Env);
		}

		if (pathname === '/api/license/deactivate') {
			if (request.method !== 'POST') return errorResponse('method not allowed', 405);
			return handleDeactivate(request, env as Env);
		}

		return new Response('Not Found', { status: 404 });
	},
} satisfies ExportedHandler<Env>;

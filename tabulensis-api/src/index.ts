import { createHmac, randomBytes, randomUUID, timingSafeEqual } from 'node:crypto';
import nacl from 'tweetnacl';
import { Resend } from 'resend';

type LicenseStatus = 'pending' | 'trialing' | 'active' | 'past_due' | 'canceled' | 'revoked';

type Env = {
	DB: D1Database;

	// Stripe
	STRIPE_SECRET_KEY?: string;
	STRIPE_WEBHOOK_SECRET?: string;
	STRIPE_PRICE_ID?: string;
	STRIPE_SUCCESS_URL?: string;
	STRIPE_CANCEL_URL?: string;
	STRIPE_PORTAL_RETURN_URL?: string;
	STRIPE_TRIAL_DAYS?: string;

	// Licensing
	LICENSE_SIGNING_KEY_B64?: string; // 32-byte Ed25519 seed, base64
	LICENSE_TOKEN_TTL_DAYS?: string; // default 14
	LICENSE_PAST_DUE_GRACE_DAYS?: string; // default 3
	LICENSE_MAX_DEVICES?: string; // default 2
	LICENSE_ADMIN_TOKEN?: string;
	LICENSE_MOCK_STRIPE?: string;

	// Resend (transactional email)
	RESEND_API_KEY?: string; // secret
	RESEND_FROM?: string; // var
	RESEND_REPLY_TO?: string; // var

	// Downloads
	// - Prefer `DOWNLOAD_BUCKET` (R2) to serve binaries from your own storage.
	// - Or set `DOWNLOAD_ORIGIN_BASE_URL` to proxy from an HTTP origin (GitHub Releases, S3, etc).
	DOWNLOAD_ORIGIN_BASE_URL?: string;
	DOWNLOAD_BUCKET?: R2Bucket; // preferred (keeps origin private), optional binding

	// CORS allowlist for browser requests (CORS never applies to CLI/desktop).
	APP_ORIGIN?: string;
};

type CheckoutStartRequest = {
	email?: string;
};

type CheckoutStartResponse = {
	checkout_url: string;
	session_id: string;
	license_key: string;
};

type CheckoutStatusResponse = {
	session_id: string;
	license_key: string | null;
	status: string | null;
};

type LicenseActivateRequest = {
	license_key: string;
	device_id: string;
	device_label?: string;
};

type LicenseDeactivateRequest = {
	license_key: string;
	device_id: string;
};

type LicenseStatusRequest = {
	license_key: string;
	device_id?: string;
};

type LicenseResendRequest = {
	email?: string;
	license_key?: string;
};

type LicenseResetRequest = {
	license_key: string;
};

type PortalSessionRequest = {
	license_key?: string;
	email?: string;
};

type PortalSessionResponse = {
	url: string;
};

type ActivationTokenPayload = {
	license_key: string;
	device_id: string;
	status: string;
	issued_at: number;
	expires_at: number;
	grace_until: number | null;
	period_end: number | null;
};

type ActivationToken = {
	payload: ActivationTokenPayload;
	signature: string;
};

type ActivationInfo = {
	device_id: string;
	device_label: string | null;
	activated_at: number;
	last_seen_at: number | null;
};

type LicenseStatusResponse = {
	license_key: string;
	status: string;
	max_devices: number;
	trial_end: number | null;
	period_end: number | null;
	activations: ActivationInfo[];
};

type ActivateResult = {
	token: ActivationToken;
	status: LicenseStatusResponse;
};

const STRIPE_API_BASE = 'https://api.stripe.com/v1';
const WEBHOOK_TOLERANCE_SECONDS = 300;

const JSON_HEADERS = {
	'content-type': 'application/json',
};

const TABULENSIS_DOWNLOAD_URL = 'https://tabulensis.com/download';
const TABULENSIS_BILLING_URL = 'https://tabulensis.com/support/billing';
const TABULENSIS_SUPPORT_EMAIL = 'support@tabulensis.com';
const LICENSE_EMAIL_SUBJECT = 'Your Tabulensis license key';

const ALLOWED_DOWNLOAD_ASSETS = new Set<string>([
	'tabulensis-latest-windows-x86_64.exe',
	'tabulensis-latest-windows-x86_64.zip',
	'tabulensis-latest-windows-x86_64.exe.sha256',
	'tabulensis-latest-windows-x86_64.zip.sha256',
	'tabulensis-latest-macos-universal.tar.gz',
	'tabulensis-latest-macos-universal.tar.gz.sha256',
	'tabulensis-latest-linux-x86_64.tar.gz',
	'tabulensis-latest-linux-x86_64.tar.gz.sha256',
]);

let schemaEnsured = false;
let schemaEnsuring: Promise<void> | null = null;

function envFlag(value: string | undefined): boolean {
	return value === '1' || value?.toLowerCase() === 'true';
}

function envInt(value: string | undefined, fallback: number): number {
	const parsed = Number(value);
	return Number.isFinite(parsed) ? parsed : fallback;
}

function nowSeconds(): number {
	return Math.floor(Date.now() / 1000);
}

function daysToSeconds(days: number): number {
	return Math.max(0, Math.trunc(days)) * 86400;
}

function resendFrom(env: Env): string {
	return env.RESEND_FROM?.trim() || 'Tabulensis <licenses@mail.tabulensis.com>';
}

function resendReplyTo(env: Env): string | undefined {
	const value = env.RESEND_REPLY_TO?.trim();
	return value ? value : undefined;
}

function escapeHtml(value: string): string {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&#39;');
}

function licenseEmailText(licenseKey: string): string {
	return [
		'Your Tabulensis license key',
		'',
		'License key:',
		licenseKey,
		'',
		'Activate:',
		`tabulensis license activate ${licenseKey}`,
		'',
		'Download:',
		TABULENSIS_DOWNLOAD_URL,
		'',
		'Billing:',
		TABULENSIS_BILLING_URL,
		'',
		'Support:',
		TABULENSIS_SUPPORT_EMAIL,
		'',
	].join('\n');
}

function licenseEmailHtml(licenseKey: string): string {
	// Keep this dependency-free and deterministic.
	const key = escapeHtml(licenseKey);
	return `
<div style="font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Arial, sans-serif; line-height: 1.4">
  <h1 style="margin: 0 0 16px 0; font-size: 20px;">Your Tabulensis license key</h1>
  <p style="margin: 0 0 12px 0;"><strong>License key:</strong> <code style="font-size: 14px;">${key}</code></p>
  <p style="margin: 0 0 12px 0;"><strong>Activate:</strong> <code style="font-size: 14px;">tabulensis license activate ${key}</code></p>
  <p style="margin: 0 0 12px 0;"><strong>Download:</strong> <a href="${TABULENSIS_DOWNLOAD_URL}">${TABULENSIS_DOWNLOAD_URL}</a></p>
  <p style="margin: 0 0 12px 0;"><strong>Billing:</strong> <a href="${TABULENSIS_BILLING_URL}">${TABULENSIS_BILLING_URL}</a></p>
  <p style="margin: 0;"><strong>Support:</strong> <a href="mailto:${TABULENSIS_SUPPORT_EMAIL}">${TABULENSIS_SUPPORT_EMAIL}</a></p>
</div>
`.trim();
}

type SendLicenseEmailParams = {
	to: string;
	licenseKey: string;
	idempotencyKey?: string;
};

async function sendLicenseEmail(env: Env, params: SendLicenseEmailParams): Promise<string> {
	const apiKey = env.RESEND_API_KEY?.trim();
	if (!apiKey) throw new Error('RESEND_API_KEY not set');

	const resend = new Resend(apiKey);
	const { data, error } = await resend.emails.send(
		{
			from: resendFrom(env),
			to: params.to.trim(),
			replyTo: resendReplyTo(env),
			subject: LICENSE_EMAIL_SUBJECT,
			text: licenseEmailText(params.licenseKey.trim()),
			html: licenseEmailHtml(params.licenseKey.trim()),
		},
		params.idempotencyKey ? { idempotencyKey: params.idempotencyKey } : undefined,
	);

	if (error) {
		const message = (error as any)?.message ? String((error as any).message) : JSON.stringify(error);
		throw new Error(`Resend send failed: ${message}`);
	}

	const id = (data as any)?.id != null ? String((data as any).id) : '';
	if (!id) throw new Error('Resend send failed: response missing id');
	return id;
}

function base32NoPad(bytes: Uint8Array): string {
	const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
	let output = '';
	let buffer = 0;
	let bitsLeft = 0;
	for (const byte of bytes) {
		buffer = (buffer << 8) | byte;
		bitsLeft += 8;
		while (bitsLeft >= 5) {
			const index = (buffer >> (bitsLeft - 5)) & 0x1f;
			output += alphabet[index] ?? '';
			bitsLeft -= 5;
		}
	}
	return output;
}

function generateLicenseKey(): string {
	const bytes = randomBytes(12);
	const encoded = base32NoPad(bytes);
	return `TABU-${encoded.slice(0, 4)}-${encoded.slice(4, 8)}-${encoded.slice(8, 12)}`;
}

function generateShortId(): string {
	return randomBytes(8).toString('hex');
}

function serializeActivationPayload(payload: ActivationTokenPayload): string {
	// Keep key order stable to match Rust `serde_json` struct serialization.
	return JSON.stringify({
		license_key: payload.license_key,
		device_id: payload.device_id,
		status: payload.status,
		issued_at: payload.issued_at,
		expires_at: payload.expires_at,
		grace_until: payload.grace_until,
		period_end: payload.period_end,
	});
}

function signingKeySeed(env: Env): Uint8Array {
	const keyB64 = env.LICENSE_SIGNING_KEY_B64?.trim();
	if (!keyB64) {
		throw new Error('LICENSE_SIGNING_KEY_B64 not set');
	}
	const seed = Buffer.from(keyB64, 'base64');
	if (seed.length !== 32) {
		throw new Error('LICENSE_SIGNING_KEY_B64 must be base64 for exactly 32 bytes');
	}
	return seed;
}

function publicKeyB64(env: Env): string {
	const seed = signingKeySeed(env);
	const kp = nacl.sign.keyPair.fromSeed(seed);
	return Buffer.from(kp.publicKey).toString('base64');
}

function signActivationToken(env: Env, payload: ActivationTokenPayload): ActivationToken {
	const seed = signingKeySeed(env);
	const kp = nacl.sign.keyPair.fromSeed(seed);
	const payloadJson = serializeActivationPayload(payload);
	const sig = nacl.sign.detached(Buffer.from(payloadJson, 'utf8'), kp.secretKey);
	return { payload, signature: Buffer.from(sig).toString('base64') };
}

function corsOrigin(request: Request, env: Env): string {
	const origin = request.headers.get('Origin');
	if (!origin) return '*';
	if (env.APP_ORIGIN && origin === env.APP_ORIGIN) return origin;
	return '*';
}

function withCors(request: Request, env: Env, response: Response): Response {
	const headers = new Headers(response.headers);
	headers.set('Access-Control-Allow-Origin', corsOrigin(request, env));
	headers.set('Access-Control-Allow-Methods', 'GET,POST,OPTIONS');
	headers.set('Access-Control-Allow-Headers', 'content-type,stripe-signature,x-admin-token');
	headers.set('Access-Control-Max-Age', '86400');
	return new Response(response.body, { status: response.status, headers });
}

function jsonResponse(request: Request, env: Env, data: unknown, status = 200): Response {
	return withCors(request, env, new Response(JSON.stringify(data), { status, headers: JSON_HEADERS }));
}

function errorResponse(request: Request, env: Env, message: string, status = 400): Response {
	return jsonResponse(request, env, { error: message }, status);
}

async function readJson<T>(request: Request): Promise<T | null> {
	try {
		return (await request.json()) as T;
	} catch {
		return null;
	}
}

async function ensureSchema(env: Env): Promise<void> {
	const db = env.DB;

	if (schemaEnsured) {
		const row = await db
			.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='licenses'")
			.first();
		if (row?.name) return;
		schemaEnsured = false;
	}

	if (schemaEnsuring) {
		await schemaEnsuring;
		return;
	}

	schemaEnsuring = (async () => {
	// Base schema (mirrors migrations; safe to run multiple times).
	await db.prepare('PRAGMA foreign_keys = ON').run();

	await db
		.prepare(
			`CREATE TABLE IF NOT EXISTS licenses (
          id TEXT PRIMARY KEY,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          status TEXT NOT NULL,
          stripe_customer_id TEXT,
          stripe_subscription_id TEXT UNIQUE,
          trial_end INTEGER,
          current_period_end INTEGER,
          max_devices INTEGER NOT NULL DEFAULT 2,
          license_key TEXT,
          email TEXT
        )`,
		)
		.run();

	await db
		.prepare(
			`CREATE TABLE IF NOT EXISTS activations (
          id TEXT PRIMARY KEY,
          license_id TEXT NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
          device_id_hash TEXT NOT NULL,
          device_label TEXT,
          activated_at INTEGER NOT NULL,
          last_seen_at INTEGER NOT NULL,
          revoked_at INTEGER,
          UNIQUE(license_id, device_id_hash)
        )`,
		)
		.run();

	await db
		.prepare(
			`CREATE TABLE IF NOT EXISTS checkout_sessions (
          session_id TEXT PRIMARY KEY,
          license_id TEXT,
          license_key TEXT NOT NULL,
          email TEXT,
          created_at INTEGER NOT NULL
        )`,
		)
		.run();

	await db
		.prepare(
			`CREATE TABLE IF NOT EXISTS stripe_events (
          id TEXT PRIMARY KEY,
          type TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          processed_at INTEGER NOT NULL
        )`,
		)
		.run();

	// Legacy table retained for now (older worker versions used it).
	await db
		.prepare(
			`CREATE TABLE IF NOT EXISTS license_keys (
          license_id TEXT PRIMARY KEY REFERENCES licenses(id) ON DELETE CASCADE,
          key_hash TEXT NOT NULL UNIQUE,
          created_at INTEGER NOT NULL
        )`,
		)
		.run();

	// Schema extensions for older DBs: ignore "duplicate column name" errors.
	const tryAddColumn = async (sql: string): Promise<void> => {
		try {
			await db.prepare(sql).run();
		} catch (err) {
			const msg = String(err);
			if (!msg.includes('duplicate column name')) {
				throw err;
			}
		}
	};
	await tryAddColumn('ALTER TABLE licenses ADD COLUMN license_key TEXT');
	await tryAddColumn('ALTER TABLE licenses ADD COLUMN email TEXT');
	await tryAddColumn('ALTER TABLE activations ADD COLUMN device_label TEXT');

	await db.prepare('CREATE UNIQUE INDEX IF NOT EXISTS idx_licenses_license_key ON licenses(license_key)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_licenses_email ON licenses(email)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_activations_license_id ON activations(license_id)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_activations_device_id_hash ON activations(device_id_hash)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_stripe_events_type ON stripe_events(type)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_checkout_sessions_license_key ON checkout_sessions(license_key)').run();
	await db.prepare('CREATE INDEX IF NOT EXISTS idx_checkout_sessions_email ON checkout_sessions(email)').run();

	// Sanity-check: schema must exist.
	const schemaCheck = await db
		.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='licenses'")
		.first();
	if (!schemaCheck?.name) {
		throw new Error('Schema initialization failed: licenses table missing');
	}
	schemaEnsured = true;
	})();

	try {
		await schemaEnsuring;
	} finally {
		schemaEnsuring = null;
	}
}

async function stripeRequest(
	env: Env,
	method: 'GET' | 'POST',
	path: string,
	body?: URLSearchParams,
): Promise<any> {
	const secretKey = env.STRIPE_SECRET_KEY?.trim();
	if (!secretKey) throw new Error('STRIPE_SECRET_KEY not set');

	const url = `${STRIPE_API_BASE}${path}`;
	const response = await fetch(url, {
		method,
		headers: {
			Authorization: `Bearer ${secretKey}`,
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

function checkoutSuccessUrl(env: Env, sessionIdPlaceholder: string): string {
	const base = env.STRIPE_SUCCESS_URL?.trim() || 'https://tabulensis.com/download/success';
	// IMPORTANT: Stripe only substitutes the literal `{CHECKOUT_SESSION_ID}` token.
	// If we add it via `URLSearchParams`, `{`/`}` get percent-encoded and Stripe will not replace it.
	const url = new URL(base);
	url.searchParams.delete('session_id');

	const prefix = `${url.origin}${url.pathname}`;
	const existing = url.searchParams.toString();
	const existingQuery = existing ? `?${existing}` : '';
	const sep = existing ? '&' : '?';
	return `${prefix}${existingQuery}${sep}session_id=${sessionIdPlaceholder}${url.hash ?? ''}`;
}

function checkoutCancelUrl(env: Env): string {
	return env.STRIPE_CANCEL_URL?.trim() || 'https://tabulensis.com/download';
}

function portalReturnUrl(env: Env): string {
	return env.STRIPE_PORTAL_RETURN_URL?.trim() || 'https://tabulensis.com/support/billing';
}

function safeDecodeURIComponent(value: string): string {
	try {
		return decodeURIComponent(value);
	} catch {
		return '';
	}
}

function downloadOriginBaseUrl(env: Env): string {
	const raw = env.DOWNLOAD_ORIGIN_BASE_URL?.trim();
	if (!raw) return '';
	return raw.endsWith('/') ? raw : `${raw}/`;
}

async function handleDownload(request: Request, env: Env): Promise<Response> {
	const url = new URL(request.url);
	const asset = url.pathname.startsWith('/download/')
		? url.pathname.slice('/download/'.length)
		: url.pathname.startsWith('/dl/')
			? url.pathname.slice('/dl/'.length)
			: '';
	const decoded = asset ? safeDecodeURIComponent(asset) : '';
	if (!decoded) return errorResponse(request, env, 'asset is required', 400);
	if (decoded.includes('/') || decoded.includes('\\') || decoded.includes('..')) {
		return errorResponse(request, env, 'invalid asset', 400);
	}
	if (!ALLOWED_DOWNLOAD_ASSETS.has(decoded)) {
		return withCors(request, env, new Response('Not Found', { status: 404 }));
	}

	if (env.DOWNLOAD_BUCKET) {
		const object = await env.DOWNLOAD_BUCKET.get(decoded);
		if (!object) {
			return withCors(
				request,
				env,
				new Response('Not Found', {
					status: 404,
					headers: { 'Cache-Control': 'no-store' },
				}),
			);
		}

		const headers = new Headers();
		headers.set('Cache-Control', 'public, max-age=300');

		const contentType = object.httpMetadata?.contentType ?? 'application/octet-stream';
		headers.set('Content-Type', contentType);

		// Force a download prompt; prevents the browser from attempting to render text/HTML.
		headers.set('Content-Disposition', `attachment; filename="${decoded}"`);

		const etag = object.httpEtag;
		if (etag) headers.set('ETag', etag);
		if (object.size !== undefined) headers.set('Content-Length', String(object.size));
		if (object.uploaded) headers.set('Last-Modified', object.uploaded.toUTCString());

		if (request.method === 'HEAD') {
			return withCors(request, env, new Response(null, { status: 200, headers }));
		}

		return withCors(request, env, new Response(object.body, { status: 200, headers }));
	}

	const originBase = downloadOriginBaseUrl(env);
	if (!originBase) {
		return withCors(
			request,
			env,
			new Response('Not Found', {
				status: 404,
				headers: { 'Cache-Control': 'no-store' },
			}),
		);
	}
	const originUrl = `${originBase}${decoded}`;

	const passthroughHeaders = new Headers();
	const range = request.headers.get('Range');
	if (range) passthroughHeaders.set('Range', range);
	const ifNoneMatch = request.headers.get('If-None-Match');
	if (ifNoneMatch) passthroughHeaders.set('If-None-Match', ifNoneMatch);
	const ifModifiedSince = request.headers.get('If-Modified-Since');
	if (ifModifiedSince) passthroughHeaders.set('If-Modified-Since', ifModifiedSince);

	const originRes = await fetch(originUrl, {
		method: request.method,
		headers: passthroughHeaders,
		redirect: 'follow',
	});

	// Avoid proxying GitHub HTML error pages (and their headers) back to the user.
	if (originRes.status === 404) {
		return withCors(
			request,
			env,
			new Response('Not Found', {
				status: 404,
				headers: { 'Cache-Control': 'no-store' },
			}),
		);
	}
	if (!originRes.ok && originRes.status !== 304 && originRes.status !== 416) {
		return withCors(
			request,
			env,
			new Response('Download unavailable', {
				status: 502,
				headers: { 'Cache-Control': 'no-store' },
			}),
		);
	}

	// Forward only the few headers that matter for downloads (avoid leaking GitHub-isms).
	const headers = new Headers();
	for (const name of [
		'content-type',
		'content-length',
		'content-range',
		'accept-ranges',
		'etag',
		'last-modified',
		'content-disposition',
		'content-encoding',
	]) {
		const value = originRes.headers.get(name);
		if (value) headers.set(name, value);
	}

	// Keep caching modest so "latest" can update shortly after a new release.
	headers.set('Cache-Control', 'public, max-age=300');

	return withCors(request, env, new Response(originRes.body, { status: originRes.status, headers }));
}

async function createCheckoutSession(env: Env, licenseKey: string, email?: string): Promise<any> {
	const priceId = env.STRIPE_PRICE_ID?.trim();
	if (!priceId) throw new Error('STRIPE_PRICE_ID not set');

	const trialDays = envInt(env.STRIPE_TRIAL_DAYS, 30);

	const params = new URLSearchParams({
		mode: 'subscription',
		'line_items[0][price]': priceId,
		'line_items[0][quantity]': '1',
		success_url: checkoutSuccessUrl(env, '{CHECKOUT_SESSION_ID}'),
		cancel_url: checkoutCancelUrl(env),
		'metadata[license_key]': licenseKey,
	});

	// Require card upfront even with trial.
	params.set('payment_method_collection', 'always');

	if (trialDays > 0) {
		params.set('subscription_data[trial_period_days]', String(trialDays));
	}
	if (email) {
		params.set('customer_email', email);
	}
	return stripeRequest(env, 'POST', '/checkout/sessions', params);
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
	const now = nowSeconds();
	if (Math.abs(now - parsed.timestamp) > WEBHOOK_TOLERANCE_SECONDS) return false;
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

async function markEventProcessed(env: Env, event: any): Promise<boolean> {
	const now = nowSeconds();
	try {
		await env.DB.prepare('INSERT INTO stripe_events (id, type, created_at, processed_at) VALUES (?, ?, ?, ?)')
			.bind(event.id, event.type, event.created ?? now, now)
			.run();
		return true;
	} catch {
		return false;
	}
}

function statusAllowsActivation(status: string): boolean {
	return status === 'trialing' || status === 'active' || status === 'past_due';
}

async function listActivations(env: Env, licenseId: string): Promise<ActivationInfo[]> {
	const rows = await env.DB.prepare(
		`SELECT device_id_hash as device_id, device_label as device_label, activated_at as activated_at, last_seen_at as last_seen_at
     FROM activations
     WHERE license_id = ? AND revoked_at IS NULL
     ORDER BY activated_at DESC`,
	)
		.bind(licenseId)
		.all();
	const results = (rows.results ?? []) as any[];
	return results.map((row) => ({
		device_id: String(row.device_id ?? ''),
		device_label: row.device_label !== undefined ? (row.device_label === null ? null : String(row.device_label)) : null,
		activated_at: Number(row.activated_at ?? 0),
		last_seen_at: row.last_seen_at !== undefined ? (row.last_seen_at === null ? null : Number(row.last_seen_at)) : null,
	}));
}

async function handleCheckoutStart(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<CheckoutStartRequest>(request);
	const email = body?.email?.trim() ? body.email.trim() : undefined;

	const licenseKey = generateLicenseKey();
	const licenseId = randomUUID();

	const maxDevices = envInt(env.LICENSE_MAX_DEVICES, 2);
	const trialDays = envInt(env.STRIPE_TRIAL_DAYS, 30);

	const now = nowSeconds();
	const initialStatus: LicenseStatus = envFlag(env.LICENSE_MOCK_STRIPE) ? 'trialing' : 'pending';
	const trialEnd = envFlag(env.LICENSE_MOCK_STRIPE) && trialDays > 0 ? now + daysToSeconds(trialDays) : null;
	const currentPeriodEnd = envFlag(env.LICENSE_MOCK_STRIPE) && trialDays > 0 ? now + daysToSeconds(trialDays) : null;

	await env.DB.prepare(
		`INSERT INTO licenses (
       id, created_at, updated_at, status, stripe_customer_id, stripe_subscription_id, trial_end, current_period_end, max_devices, license_key, email
     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
	)
		.bind(
			licenseId,
			now,
			now,
			initialStatus,
			null,
			null,
			trialEnd,
			currentPeriodEnd,
			maxDevices,
			licenseKey,
			email ?? null,
		)
		.run();

	if (envFlag(env.LICENSE_MOCK_STRIPE)) {
		const sessionId = `mock_${generateShortId()}`;
		await env.DB.prepare(
			`INSERT OR REPLACE INTO checkout_sessions (session_id, license_id, license_key, email, created_at)
       VALUES (?, ?, ?, ?, ?)`,
		)
			.bind(sessionId, licenseId, licenseKey, email ?? null, now)
			.run();

		const successBase = env.STRIPE_SUCCESS_URL?.trim() || 'https://tabulensis.com/download/success';
		const successUrl = new URL(successBase);
		successUrl.searchParams.set('license_key', licenseKey);
		const checkoutUrl = successUrl.toString();

		return jsonResponse(request, env, {
			checkout_url: checkoutUrl,
			session_id: sessionId,
			license_key: licenseKey,
		} satisfies CheckoutStartResponse);
	}

	try {
		const session = await createCheckoutSession(env, licenseKey, email);
		const sessionId = String(session.id ?? '');
		const checkoutUrl = String(session.url ?? '');
		if (!sessionId || !checkoutUrl) {
			return errorResponse(request, env, 'Stripe response missing session id or url', 502);
		}

		await env.DB.prepare(
			`INSERT OR REPLACE INTO checkout_sessions (session_id, license_id, license_key, email, created_at)
       VALUES (?, ?, ?, ?, ?)`,
		)
			.bind(sessionId, licenseId, licenseKey, email ?? null, now)
			.run();

		return jsonResponse(request, env, {
			checkout_url: checkoutUrl,
			session_id: sessionId,
			license_key: licenseKey,
		} satisfies CheckoutStartResponse);
	} catch (err) {
		return errorResponse(request, env, (err as Error).message, 502);
	}
}

async function handleCheckoutStatus(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const url = new URL(request.url);
	const sessionId = url.searchParams.get('session_id');
	if (!sessionId) return errorResponse(request, env, 'session_id is required');

	const sessionRow = await env.DB.prepare('SELECT license_key as license_key FROM checkout_sessions WHERE session_id = ?')
		.bind(sessionId)
		.first();
	const licenseKey = (sessionRow?.license_key as string | undefined) ?? null;

	let status: string | null = null;
	if (licenseKey) {
		const licenseRow = await env.DB.prepare('SELECT status as status FROM licenses WHERE license_key = ?')
			.bind(licenseKey)
			.first();
		status = (licenseRow?.status as string | undefined) ?? null;
	}

	return jsonResponse(request, env, {
		session_id: sessionId,
		license_key: licenseKey,
		status,
	} satisfies CheckoutStatusResponse);
}

async function handleWebhook(request: Request, env: Env, ctx?: ExecutionContext): Promise<Response> {
	await ensureSchema(env);

	const rawBody = await request.text();

	const secret = env.STRIPE_WEBHOOK_SECRET?.trim();
	if (secret) {
		const signature = request.headers.get('Stripe-Signature');
		if (!verifyStripeWebhook(rawBody, signature, secret)) {
			return errorResponse(request, env, 'invalid signature', 400);
		}
	}

	let event: any;
	try {
		event = JSON.parse(rawBody);
	} catch {
		return errorResponse(request, env, 'invalid payload', 400);
	}

	const inserted = await markEventProcessed(env, event);
	if (!inserted) return jsonResponse(request, env, { received: true });

	const data = event.data?.object;
	if (!data) return jsonResponse(request, env, { received: true });

	const eventType = String(event.type ?? '');

	if (eventType === 'checkout.session.completed') {
		const sessionId = String(data.id ?? '');
		const customerId = data.customer ? String(data.customer) : null;
		const subscriptionId = data.subscription ? String(data.subscription) : null;
		const email =
			data.customer_details?.email != null
				? String(data.customer_details.email)
				: data.customer_email != null
					? String(data.customer_email)
					: null;
		const metadataLicenseKey =
			data.metadata?.license_key != null ? String(data.metadata.license_key) : null;

		let licenseKey: string | null = metadataLicenseKey;
		if (!licenseKey && sessionId) {
			const row = await env.DB.prepare('SELECT license_key as license_key FROM checkout_sessions WHERE session_id = ?')
				.bind(sessionId)
				.first();
			licenseKey = (row?.license_key as string | undefined) ?? null;
		}
		if (!licenseKey) {
			licenseKey = generateLicenseKey();
		}

		const now = nowSeconds();
		const existing = await env.DB.prepare('SELECT id as id FROM licenses WHERE license_key = ?')
			.bind(licenseKey)
			.first();
		if (existing?.id) {
			await env.DB.prepare(
				`UPDATE licenses
         SET status = ?, updated_at = ?, stripe_customer_id = COALESCE(?, stripe_customer_id),
             stripe_subscription_id = COALESCE(?, stripe_subscription_id),
             email = COALESCE(?, email)
         WHERE license_key = ?`,
			)
				.bind('trialing', now, customerId, subscriptionId, email, licenseKey)
				.run();
			} else {
				const licenseId = randomUUID();
				await env.DB.prepare(
					`INSERT INTO licenses (
	           id, created_at, updated_at, status, stripe_customer_id, stripe_subscription_id, trial_end, current_period_end, max_devices, license_key, email
	         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
				)
					.bind(licenseId, now, now, 'trialing', customerId, subscriptionId, null, null, envInt(env.LICENSE_MAX_DEVICES, 2), licenseKey, email)
					.run();
			}

			// Best-effort email send: webhook should still succeed even if email delivery fails.
			const apiKey = env.RESEND_API_KEY?.trim();
			if (apiKey && email) {
				const idempotencyKey = event.id != null ? `license-email/stripe-event/${String(event.id)}` : undefined;
				const send = sendLicenseEmail(env, { to: email, licenseKey, idempotencyKey }).catch((err) => {
					console.error('license email send failed', err);
				});
				if (ctx) {
					ctx.waitUntil(send);
				} else {
					await send;
				}
			}
		}

	if (eventType === 'invoice.paid') {
		const subscriptionId = data.subscription ? String(data.subscription) : null;
		const customerId = data.customer ? String(data.customer) : null;
		const periodEnd = data.lines?.data?.[0]?.period?.end != null ? Number(data.lines.data[0].period.end) : null;
		const now = nowSeconds();
		if (subscriptionId) {
			await env.DB.prepare(
				`UPDATE licenses
         SET status = ?, updated_at = ?, current_period_end = COALESCE(?, current_period_end),
             stripe_customer_id = COALESCE(?, stripe_customer_id)
         WHERE stripe_subscription_id = ?`,
			)
				.bind('active', now, periodEnd, customerId, subscriptionId)
				.run();
		} else if (customerId) {
			await env.DB.prepare(
				`UPDATE licenses
         SET status = ?, updated_at = ?, current_period_end = COALESCE(?, current_period_end)
         WHERE stripe_customer_id = ?`,
			)
				.bind('active', now, periodEnd, customerId)
				.run();
		}
	}

	if (eventType === 'invoice.payment_failed') {
		const subscriptionId = data.subscription ? String(data.subscription) : null;
		const customerId = data.customer ? String(data.customer) : null;
		const now = nowSeconds();
		if (subscriptionId) {
			await env.DB.prepare('UPDATE licenses SET status = ?, updated_at = ? WHERE stripe_subscription_id = ?')
				.bind('past_due', now, subscriptionId)
				.run();
		} else if (customerId) {
			await env.DB.prepare('UPDATE licenses SET status = ?, updated_at = ? WHERE stripe_customer_id = ?')
				.bind('past_due', now, customerId)
				.run();
		}
	}

	if (eventType === 'customer.subscription.updated') {
		const subscriptionId = data.id != null ? String(data.id) : null;
		const customerId = data.customer != null ? String(data.customer) : null;
		const status = data.status != null ? String(data.status) : null;
		const trialEnd = data.trial_end != null ? Number(data.trial_end) : null;
		const periodEnd = data.current_period_end != null ? Number(data.current_period_end) : null;
		const now = nowSeconds();
		if (subscriptionId && status) {
			await env.DB.prepare(
				`UPDATE licenses
         SET status = ?, updated_at = ?, trial_end = COALESCE(?, trial_end), current_period_end = COALESCE(?, current_period_end),
             stripe_customer_id = COALESCE(?, stripe_customer_id)
         WHERE stripe_subscription_id = ?`,
			)
				.bind(status, now, trialEnd, periodEnd, customerId, subscriptionId)
				.run();
		}
	}

	if (eventType === 'customer.subscription.deleted') {
		const subscriptionId = data.id != null ? String(data.id) : null;
		const now = nowSeconds();
		if (subscriptionId) {
			await env.DB.prepare('UPDATE licenses SET status = ?, updated_at = ? WHERE stripe_subscription_id = ?')
				.bind('canceled', now, subscriptionId)
				.run();
		}
	}

	return jsonResponse(request, env, { received: true });
}

async function handleLicenseActivate(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<LicenseActivateRequest>(request);
	if (!body?.license_key || !body.device_id) {
		return errorResponse(request, env, 'license_key and device_id are required');
	}
	const licenseKey = body.license_key.trim();
	const deviceIdHash = body.device_id.trim();
	const deviceLabel = body.device_label?.trim() || null;

	const license = await env.DB.prepare(
		`SELECT id as id, status as status, max_devices as max_devices, trial_end as trial_end, current_period_end as period_end
     FROM licenses WHERE license_key = ?`,
	)
		.bind(licenseKey)
		.first();
	if (!license) return errorResponse(request, env, 'License not found', 404);

	const status = String(license.status ?? '');
	if (!statusAllowsActivation(status)) {
		return errorResponse(request, env, `License status does not allow activation: ${status}`, 403);
	}

	const licenseId = String(license.id ?? '');
	const maxDevices = Number(license.max_devices ?? envInt(env.LICENSE_MAX_DEVICES, 2));

	const existing = await env.DB.prepare(
		`SELECT id as id FROM activations WHERE license_id = ? AND device_id_hash = ? AND revoked_at IS NULL`,
	)
		.bind(licenseId, deviceIdHash)
		.first();

	const now = nowSeconds();
	const countRow = await env.DB.prepare(
		`SELECT COUNT(1) as count FROM activations WHERE license_id = ? AND revoked_at IS NULL`,
	)
		.bind(licenseId)
		.first();
	const activeCount = Number(countRow?.count ?? 0);

	if (!existing?.id && activeCount >= maxDevices) {
		return errorResponse(request, env, 'Device limit reached', 403);
	}

	if (existing?.id) {
		await env.DB.prepare(
			`UPDATE activations SET last_seen_at = ?, device_label = COALESCE(?, device_label) WHERE id = ?`,
		)
			.bind(now, deviceLabel, existing.id)
			.run();
	} else {
		const activationId = randomUUID();
		await env.DB.prepare(
			`INSERT INTO activations (id, license_id, device_id_hash, device_label, activated_at, last_seen_at)
       VALUES (?, ?, ?, ?, ?, ?)`,
		)
			.bind(activationId, licenseId, deviceIdHash, deviceLabel, now, now)
			.run();
	}

	const tokenTtlDays = envInt(env.LICENSE_TOKEN_TTL_DAYS, 14);
	const pastDueGraceDays = envInt(env.LICENSE_PAST_DUE_GRACE_DAYS, 3);
	const expiresAt = status === 'past_due' ? now + daysToSeconds(pastDueGraceDays) : now + daysToSeconds(tokenTtlDays);
	const periodEnd = license.period_end != null ? Number(license.period_end) : null;

	const payload: ActivationTokenPayload = {
		license_key: licenseKey,
		device_id: deviceIdHash,
		status,
		issued_at: now,
		expires_at: expiresAt,
		grace_until: expiresAt,
		period_end: periodEnd,
	};
	let token: ActivationToken;
	try {
		token = signActivationToken(env, payload);
	} catch (err) {
		return errorResponse(request, env, (err as Error).message, 500);
	}

	const activations = await listActivations(env, licenseId);
	const statusResponse: LicenseStatusResponse = {
		license_key: licenseKey,
		status,
		max_devices: maxDevices,
		trial_end: license.trial_end != null ? Number(license.trial_end) : null,
		period_end: periodEnd,
		activations,
	};

	return jsonResponse(request, env, { token, status: statusResponse } satisfies ActivateResult);
}

async function handleLicenseDeactivate(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<LicenseDeactivateRequest>(request);
	if (!body?.license_key || !body.device_id) {
		return errorResponse(request, env, 'license_key and device_id are required');
	}

	const licenseKey = body.license_key.trim();
	const deviceIdHash = body.device_id.trim();

	const license = await env.DB.prepare('SELECT id as id FROM licenses WHERE license_key = ?')
		.bind(licenseKey)
		.first();
	if (!license) return errorResponse(request, env, 'License not found', 404);

	const now = nowSeconds();
	await env.DB.prepare(
		`UPDATE activations SET revoked_at = ? WHERE license_id = ? AND device_id_hash = ? AND revoked_at IS NULL`,
	)
		.bind(now, String(license.id), deviceIdHash)
		.run();

	return jsonResponse(request, env, { status: 'ok' });
}

async function handleLicenseStatus(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<LicenseStatusRequest>(request);
	if (!body?.license_key) {
		return errorResponse(request, env, 'license_key is required');
	}

	const licenseKey = body.license_key.trim();
	const deviceIdHash = body.device_id?.trim() || null;

	const license = await env.DB.prepare(
		`SELECT id as id, status as status, max_devices as max_devices, trial_end as trial_end, current_period_end as period_end
     FROM licenses WHERE license_key = ?`,
	)
		.bind(licenseKey)
		.first();
	if (!license) return errorResponse(request, env, 'License not found', 404);

	const licenseId = String(license.id ?? '');
	const now = nowSeconds();

	if (deviceIdHash) {
		const existing = await env.DB.prepare(
			`SELECT id as id FROM activations WHERE license_id = ? AND device_id_hash = ? AND revoked_at IS NULL`,
		)
			.bind(licenseId, deviceIdHash)
			.first();
		if (existing?.id) {
			await env.DB.prepare('UPDATE activations SET last_seen_at = ? WHERE id = ?')
				.bind(now, existing.id)
				.run();
		}
	}

	const activations = await listActivations(env, licenseId);
	const statusResponse: LicenseStatusResponse = {
		license_key: licenseKey,
		status: String(license.status ?? ''),
		max_devices: Number(license.max_devices ?? envInt(env.LICENSE_MAX_DEVICES, 2)),
		trial_end: license.trial_end != null ? Number(license.trial_end) : null,
		period_end: license.period_end != null ? Number(license.period_end) : null,
		activations,
	};
	return jsonResponse(request, env, statusResponse);
}

async function handleLicenseResend(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<LicenseResendRequest>(request);
	if (!body?.email && !body?.license_key) {
		return errorResponse(request, env, 'email or license_key is required');
	}

	let licenseKey: string | null = null;
	let email: string | null = null;

	if (body.license_key) {
		const key = body.license_key.trim();
		const row = await env.DB.prepare('SELECT license_key as license_key, email as email FROM licenses WHERE license_key = ?')
			.bind(key)
			.first();
		licenseKey = row?.license_key != null ? String(row.license_key) : null;
		email = row?.email != null ? String(row.email) : null;
	} else if (body.email) {
		const addr = body.email.trim();
		const row = await env.DB.prepare(
			`SELECT license_key as license_key, email as email
	     FROM licenses
	     WHERE email = ?
	     ORDER BY updated_at DESC
	     LIMIT 1`,
		)
			.bind(addr)
			.first();
		licenseKey = row?.license_key != null ? String(row.license_key) : null;
		email = row?.email != null ? String(row.email) : null;
	}

	if (!licenseKey) return errorResponse(request, env, 'License not found', 404);
	if (!email) {
		return errorResponse(
			request,
			env,
			'Email is missing for this license. Contact support for help.',
			400,
		);
	}

	try {
		const id = await sendLicenseEmail(env, { to: email, licenseKey });
		return jsonResponse(request, env, { status: 'sent', id });
	} catch (err) {
		return errorResponse(request, env, (err as Error).message, 502);
	}
}

async function handleLicenseReset(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const expected = env.LICENSE_ADMIN_TOKEN?.trim();
	if (!expected) return errorResponse(request, env, 'Admin token not configured', 403);

	const provided = request.headers.get('x-admin-token');
	if (!provided) return errorResponse(request, env, 'Missing admin token', 401);
	if (provided !== expected) return errorResponse(request, env, 'Invalid admin token', 401);

	const body = await readJson<LicenseResetRequest>(request);
	if (!body?.license_key) return errorResponse(request, env, 'license_key is required');

	const licenseKey = body.license_key.trim();
	const license = await env.DB.prepare('SELECT id as id FROM licenses WHERE license_key = ?')
		.bind(licenseKey)
		.first();
	if (!license?.id) return errorResponse(request, env, 'License not found', 404);

	const now = nowSeconds();
	await env.DB.prepare('UPDATE activations SET revoked_at = ? WHERE license_id = ? AND revoked_at IS NULL')
		.bind(now, String(license.id))
		.run();

	return jsonResponse(request, env, { status: 'reset' });
}

async function handlePortalSession(request: Request, env: Env): Promise<Response> {
	await ensureSchema(env);

	const body = await readJson<PortalSessionRequest>(request);
	if (!body?.license_key && !body?.email) {
		return errorResponse(request, env, 'license_key or email is required');
	}

	if (envFlag(env.LICENSE_MOCK_STRIPE)) {
		return jsonResponse(request, env, { url: portalReturnUrl(env) } satisfies PortalSessionResponse);
	}

	let customerId: string | null = null;
	if (body.license_key) {
		const row = await env.DB.prepare('SELECT stripe_customer_id as stripe_customer_id FROM licenses WHERE license_key = ?')
			.bind(body.license_key.trim())
			.first();
		customerId = row?.stripe_customer_id != null ? String(row.stripe_customer_id) : null;
	}
	if (!customerId && body.email) {
		const row = await env.DB.prepare('SELECT stripe_customer_id as stripe_customer_id FROM licenses WHERE email = ? LIMIT 1')
			.bind(body.email.trim())
			.first();
		customerId = row?.stripe_customer_id != null ? String(row.stripe_customer_id) : null;
	}
	if (!customerId) return errorResponse(request, env, 'Customer not found', 404);

	try {
		const params = new URLSearchParams({ customer: customerId, return_url: portalReturnUrl(env) });
		const session = await stripeRequest(env, 'POST', '/billing_portal/sessions', params);
		const url = session.url != null ? String(session.url) : '';
		if (!url) return errorResponse(request, env, 'Stripe response missing portal url', 502);
		return jsonResponse(request, env, { url } satisfies PortalSessionResponse);
	} catch (err) {
		return errorResponse(request, env, (err as Error).message, 502);
	}
}

async function handleHealth(request: Request, env: Env): Promise<Response> {
	try {
		await ensureSchema(env);
		await env.DB.prepare('SELECT 1').first();
		return jsonResponse(request, env, { status: 'ok' });
	} catch (err) {
		return errorResponse(request, env, String(err), 500);
	}
}

async function handlePublicKey(request: Request, env: Env): Promise<Response> {
	try {
		return jsonResponse(request, env, { public_key_b64: publicKeyB64(env) });
	} catch (err) {
		return errorResponse(request, env, (err as Error).message, 500);
	}
}

export default {
	async fetch(request, env, ctx): Promise<Response> {
		const url = new URL(request.url);
		const { pathname } = url;

		if (request.method === 'OPTIONS') {
			return withCors(request, env as Env, new Response(null, { status: 204 }));
		}

		// Legacy + website endpoints (match `license_service`).
		if (pathname === '/health') {
			if (request.method !== 'GET') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleHealth(request, env as Env);
		}
		if (pathname === '/public_key') {
			if (request.method !== 'GET') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handlePublicKey(request, env as Env);
		}
		if (pathname.startsWith('/download/') || pathname.startsWith('/dl/')) {
			if (request.method !== 'GET' && request.method !== 'HEAD') {
				return errorResponse(request, env as Env, 'method not allowed', 405);
			}
			return handleDownload(request, env as Env);
		}
		if (pathname === '/api/checkout/start') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleCheckoutStart(request, env as Env);
		}
		if (pathname === '/api/checkout/status') {
			if (request.method !== 'GET') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleCheckoutStatus(request, env as Env);
		}
			if (pathname === '/stripe/webhook') {
				if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
				return handleWebhook(request, env as Env, ctx);
			}
		if (pathname === '/license/activate') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseActivate(request, env as Env);
		}
		if (pathname === '/license/deactivate') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseDeactivate(request, env as Env);
		}
		if (pathname === '/license/status') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseStatus(request, env as Env);
		}
		if (pathname === '/license/resend') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseResend(request, env as Env);
		}
		if (pathname === '/license/reset') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseReset(request, env as Env);
		}
		if (pathname === '/portal/session') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handlePortalSession(request, env as Env);
		}

		// Compatibility aliases (older docs referenced these).
		if (pathname === '/api/stripe/checkout/start') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleCheckoutStart(request, env as Env);
		}
		if (pathname === '/api/stripe/checkout/session') {
			// Keep as an alias to checkout status for now.
			if (request.method !== 'GET') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleCheckoutStatus(request, env as Env);
		}
		if (pathname === '/api/stripe/customer-portal/session') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handlePortalSession(request, env as Env);
		}
			if (pathname === '/api/stripe/webhook') {
				if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
				return handleWebhook(request, env as Env, ctx);
			}
		if (pathname === '/api/license/activate') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseActivate(request, env as Env);
		}
		if (pathname === '/api/license/deactivate') {
			if (request.method !== 'POST') return errorResponse(request, env as Env, 'method not allowed', 405);
			return handleLicenseDeactivate(request, env as Env);
		}

		return withCors(request, env as Env, new Response('Not Found', { status: 404 }));
	},
} satisfies ExportedHandler<Env>;

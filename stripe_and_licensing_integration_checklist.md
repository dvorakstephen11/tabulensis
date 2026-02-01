## 2.01 — Initialize a Worker project (you’re on the right screen)

You’re on **Workers & Pages → Create application → Create a Worker**. From here, the fastest correct path is:

### Option A (fastest): “Start with Hello World!” in the dashboard

1. Click **Start with Hello World!**
2. On the next screen, set the **Worker name** to something like:

   * `tabulensis-api` (recommended) or `tabulensis-licensing`
3. Leave the default “Hello World” code as-is for now.
4. Click **Create and deploy** (or **Deploy**).
5. After it deploys, open the provided `workers.dev` URL and confirm you see the Hello World response.

This matches Cloudflare’s “create via dashboard → deploy” flow. ([Cloudflare Docs][1])

---

### Option B (recommended for real development): create the project locally (C3) + deploy with Wrangler

This gives you a repo with `wrangler.jsonc` + `src/index.ts` and makes Stripe/D1 work much smoother.

Run:

```bash
npm create cloudflare@latest -- tabulensis-api
```

When prompted, choose:

* **Hello World example**
* **Worker only**
* **TypeScript**
* **Use git**: Yes
* **Deploy now**: No (you can deploy after you add bindings/secrets)

Then:

```bash
cd tabulensis-api
npx wrangler login
npx wrangler dev
```

And when ready:

```bash
npx wrangler deploy
```

This is Cloudflare’s current recommended CLI bootstrap path. ([Cloudflare Docs][2])

---

### What I’d do *right now* in your position

Do **Option A** to quickly create the Worker and reserve the name, then immediately do **Option B** to generate the real codebase you’ll actually use for Stripe + D1.

If you tell me which name you want (`tabulensis-api` vs `tabulensis-licensing`), I’ll give you **2.02** next: creating the D1 DB + adding the `d1_databases` binding in `wrangler.jsonc` + the exact migration commands.

[1]: https://developers.cloudflare.com/workers/get-started/dashboard/?utm_source=chatgpt.com "Get started - Dashboard · Cloudflare Workers docs"
[2]: https://developers.cloudflare.com/workers/get-started/guide/?utm_source=chatgpt.com "Get started - CLI · Cloudflare Workers docs"

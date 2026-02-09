Here are concrete **cost** and **integration effort/time** details for **EV code signing** (Option 2), with the main gotcha up front: **EV almost always implies hardware-backed keys (token or cloud HSM), which directly affects CI design**. ([sectigo.com](https://www.sectigo.com/ssl-certificates-tls/code-signing?utm_source=openai))

> Update (2026-02-09): For Tabulensis Windows releases, the chosen path is **Azure Artifact Signing (Trusted Signing)** instead of buying/operating an EV cert workflow. This note is kept for EV cost/integration context.

## Cost (Typical, With Real Numbers)

**Certificate price (examples):**
- **DigiCert EV Code Signing**: **$699 USD (1-year)** (datasheet pricing). ([digicert.com](https://www.digicert.com/content/dam/digicert/pdfs/datasheet/digicert-code-signing-instilling-trust-in-your-users-datasheet-en.pdf?utm_source=openai))
- **SSL.com EV Code Signing**: **$349/yr (1-year)**, **$299/yr (2-year)**, **$249/yr (3-year)** on their order page. ([ssl.com](https://www.ssl.com/certificates/ev-code-signing/buy/?utm_source=openai))

**Key storage / signing method cost (this is where EV can get expensive):**
- **Physical token (USB / FIPS key)**: some vendors price/ship it as part of the process; SSL.com shows **YubiKey pricing (example: $279 each)** depending on configuration. ([ssl.com](https://www.ssl.com/certificates/ev-code-signing/buy/?utm_source=openai))
- **Cloud signing subscription (CI-friendly)**: SSL.com’s **eSigner** adds an ongoing subscription (example tiers: **$100/month for 10 signings**, **$300/month for 100**, etc; annual plans available). ([ssl.com](https://www.ssl.com/guide/esigner-pricing/?utm_source=openai))
- **Microsoft Azure Artifact Signing (alternative to EV, but often solves the same “bounce” problem)**: **$9.99/month for up to 5,000 signatures** or **$99.99/month for up to 100,000**, then **$0.005/signature** over quota. ([azure.microsoft.com](https://azure.microsoft.com/en-us/products/artifact-signing?utm_source=openai))

## Time To Obtain The Cert (EV Vetting + Delivery)

Concrete example (SSL.com):
- **Standard validation**: **3–5 days** after complete submission + successful callback. ([ssl.com](https://www.ssl.com/certificates/ev-code-signing/buy/?utm_source=openai))  
- **Standard shipping (continental US)**: **2–3 days**. ([ssl.com](https://www.ssl.com/certificates/ev-code-signing/buy/?utm_source=openai))  
So expect roughly **~1 week** end-to-end in the “everything goes smoothly” case, **~2 weeks** if validation info/callbacks lag.

## Integration Effort/Time (What You Actually Build)

### Key reality: CI on GitHub-hosted runners vs EV key custody
Sectigo explicitly notes code signing certs are **token-installed and shipped** or must be installed on a **FIPS HSM with key attestation**. That means the “easy PFX-in-secrets” flow is often *not available* for EV. ([sectigo.com](https://www.sectigo.com/ssl-certificates-tls/code-signing?utm_source=openai))

### Integration paths (effort + elapsed time)
1) **EV + physical token**
- **Works best with**: self-hosted runner (Windows machine) with token connected.
- **Effort**: typically **0.5–2 days** to set up runner + token drivers + `signtool` steps, plus ongoing maintenance.
- **Elapsed time**: dominated by cert issuance/shipping (~1–2 weeks).

2) **EV + cloud signing (recommended if you want GitHub-hosted runners)**
- **Works with**: GitHub-hosted runners; secrets become API creds (not a PFX).
- **Effort**: typically **4–16 engineering hours** to wire vendor tooling into CI (install tool, authenticate, sign, timestamp, verify; then ensure hashes are computed *after* signing).
- **Elapsed time**: cert issuance (~1–2 weeks) + **~1 day** CI wiring once credentials are ready.

### Repo-specific work you’d do here
In this repo’s `.github/workflows/release.yml`, you’d add a signing step in `build-windows` **after** `cargo build ...` and **before** checksums/packaging, signing `target/release-cli/tabulensis.exe`, then produce the `tabulensis-...exe` and zip. (This keeps SHA256 stable for the signed artifacts.)

## Important near-term change (affects cost + ops immediately)
Starting **Feb 23–24, 2026**, major CAs begin enforcing max code-signing cert validity of **459 days**, with the broader effective date **March 1, 2026**. ([sectigo.com](https://www.sectigo.com/resource-library/shorter-validity-periods-for-code-signing-certificates?utm_source=openai))  
Practical impact:
- You should assume **renew/reissue about every ~15 months**, not every 3 years. ([digicert.com](https://www.digicert.com/blog/understanding-the-new-code-signing-certificate-validity-change?utm_source=openai))
- Token-based workflows are the most operationally painful under shorter lifetimes; cloud signing services are designed to reduce that disruption. ([digicert.com](https://www.digicert.com/blog/understanding-the-new-code-signing-certificate-validity-change?utm_source=openai))

If you want, I can translate this into a “recommended EV path” for *your* constraints (GitHub-hosted CI, minimize bounce, minimize ops), but the key fork is: **self-hosted token runner** vs **cloud signing**.

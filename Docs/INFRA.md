# Infra

Scope follows [ROADMAP.md](ROADMAP.md): no infra should exist ahead
of the phase that needs it. Phase 0–2 need almost none of this.

## Repo structure

- `underwater-core` — Rust crate, the engine. No UI, no networking,
  no Supabase dependency. This is the part that goes Apache 2.0 in
  Phase 4, so keep it clean of anything proprietary from day one
  (don't bolt the ML model or license checks directly into it —
  gate those at the app layer via the plugin trait).
- `underwater-cli` — Rust binary crate (`clap`), Phase 0 test
  harness, same Cargo workspace as `underwater-core`.
- App shell(s) — added in Phase 2 (macOS) / Phase 5 (iOS), thin
  Swift wrappers calling into `underwater-core` via UniFFI, same
  layering discipline as Currents (View → ViewModel → Service, no
  business logic in the view).

## Deep Sea Labs shared infra

Underwater shares the Deep Sea Labs Supabase **org** (billing entity,
Apple Developer account, domain) with Currents, but gets its **own
Supabase project** — not a shared schema. Reasons:
- Underwater's data (license entitlements, cloud render jobs) has
  nothing to do with Currents' dive-log domain; a shared schema would
  just be two unrelated products fighting over migrations.
- Keeps Underwater's data independently deletable/exportable if it's
  ever spun off or sunset — doesn't drag Currents' data model along.
- Still gets the org-level benefit (single Supabase bill, consistent
  auth provider config, reused Edge Function conventions).

If this turns out wrong once Phase 3 actually starts, it's a cheap
decision to revisit — no data exists yet to migrate.

## Phase 3 (licensing) infra

- Supabase project: `licenses` table (`client_id`, `product_tier`,
  `issued_at`, `revoked_at`) + Edge Function to validate a license
  key against a build. Follows the same
  `client_id`/`server_version` shape Currents uses for its
  sync-ready tables, even though Underwater doesn't need dive-log
  sync — consistency here means one mental model across Deep Sea
  Labs projects, not a real sync requirement.
- Payment processor: Paddle or Lemon Squeezy (merchant-of-record).
  Do not stand up raw Stripe + manual tax handling solo — that's a
  part-time job by itself.

## Phase 2 distribution infra (direct download, pre–App Store)

- Notarization: `notarytool` via a signed Developer ID cert (Deep
  Sea Labs Apple Developer account). Scriptable, doesn't need Xcode
  Cloud for a single-target macOS app.
- Auto-update: Sparkle framework, EdDSA-signed appcast. This is the
  standard for indie direct-download Mac apps and avoids building a
  custom update mechanism.
- Hosting for the `.dmg` + appcast feed: a static host is enough
  (e.g. GitHub Releases assets, or an S3/Cloudflare R2 bucket if
  GitHub Releases bandwidth becomes a problem — it won't, early on).

## Phase 6 (cloud grading) infra — not built yet, flagged for later

This is the one phase with real ongoing infra cost (GPU-backed
render workers), so don't provision anything here until Phase 3 has
proven people will pay. When it happens: separate render-queue
service (not Supabase Edge Functions — those aren't built for
long-running GPU jobs), job status tracked in the Underwater Supabase
project, results delivered via signed storage URLs.

## CI

- GitHub Actions on `underwater-core`: build + run the golden-image
  regression tests from Phase 0 on every PR. This is the one piece
  of CI worth having from day one — a color-engine regression is
  exactly the kind of bug that's invisible in a diff and obvious in
  the output.
- App-shell CI (build/notarize/sign) can wait until Phase 2 actually
  has an app to build.

# Roadmap

Solo, nights-and-weekends pace. Each phase ends with a checkpoint —
don't start the next until the checkpoint is satisfied. Engine
before UI; UI before monetization; monetization before cloud.

## Test data

Real underwater stills for manual/visual testing live in
`engine/tests/fixtures/` (5 images spanning clear blue, low-vis cave,
and green/turbid freshwater — see `ATTRIBUTION.md` in that directory
for sources/licenses).

**Deliberately not sourced from academic underwater-image-enhancement
benchmarks** (checked 2026-07-03): UIEB and SQUID explicitly forbid
commercial use; RUIE and U45 carry no license grant at all (default
all-rights-reserved); EUVP/LSUI licensing is unstated; Sea-thru's
dataset is patent-gated behind written approval. None of these are
usable for a commercial open-core product without separately
negotiating rights — don't reach for them later without re-checking
this. Fixtures instead come from Wikimedia Commons files with an
explicit, individually-verified permissive license (public
domain/CC0/CC-BY-SA).

For Phase 1b's ML training data, this constraint doesn't bite — the
approach is synthetic degradation of ordinary clean photos (see Phase
1b below), and clean photos have no underwater-specific licensing
problem; any CC0 source works.

A quick real-world data point from testing manual sliders against
`redsea_cave_ccbysa.jpg`: pushing temperature/exposure/contrast/vibrance
hard by hand still leaves a strong blue cast — manual white-balance
correction alone can't fully counter it. Concrete confirmation of why
Phase 1a (below) is a distinct feature, not just "a stronger
temperature slider."

## Phase 0 — Core color engine (Rust workspace, no UI)
Goal: prove the engine is correct and fast before spending any time
on app chrome — and prove it's genuinely cross-platform from the
start, not "Swift with the Apple parts avoided."

Engine is Rust, not Swift. Swift's color/GPU/video/ML frameworks
(Core Image, Metal, AVFoundation, Core ML) don't exist outside
macOS/iOS, so a cross-platform engine can't be built on them without
hand-rolled FFI to Vulkan/ffmpeg/ONNX anyway — Rust's crates for all
of that are mature already. Swift only enters at the app-shell layer
(Phase 2+), calling into this engine via UniFFI.

- `underwater-core` crate: manual white balance, color temp, tint,
  exposure, contrast, saturation, vibrance
- RAW decode (`rawler`, pure Rust — fall back to `libraw` FFI only if
  format coverage falls short) + 4K–8K video frame pipeline
  (`ffmpeg-next`; must build against an LGPL-only ffmpeg — no
  GPL codecs like x264 — to stay compatible with the Apache 2.0
  license this crate ships under in Phase 4)
- GPU-accelerated rendering via `wgpu` (WGSL compute shaders, one
  codepath that runs on Metal/Vulkan/DX12 depending on platform)
- Plugin trait (`ColorFilter`) so filters are swappable modules, not
  hardcoded into the pipeline
- `underwater-cli` binary crate (`clap`) that applies adjustments to
  a file from the command line — this is the test harness, not a
  product, and needs no FFI since it's Rust calling Rust
- Golden-image regression tests (`cargo test`, known input +
  adjustment → expected output, pixel-diff tolerance) running in CI
  on Linux runners — cheaper than macOS runners and doubles as
  continuous proof the engine stays portable

**Checkpoint:** CLI can correct a real dive clip and it looks right
to your own eye against a manually-graded reference in DaVinci
Resolve. Don't move on until that's true.

## Phase 1 — One-click auto-correct (still engine-only)
Goal: the first premium-tier feature, built and validated before any
UI exists to sell it through. Split into a classical stage that ships
fast and an ML stage that upgrades it later — see research notes
below for why.

### Research context
Surveyed both physics-based and ML approaches before picking a path
(background research, 2026-07-03; see git history/conversation for
full brief — summarized here):

- **Physics-based restoration** (Sea-thru, Akkaynak & Treibitz, CVPR
  2019) is the most accurate underwater restoration method, but
  requires per-pixel range (RGBD), typically from multi-view
  structure-from-motion — not viable for single frames/video without
  a depth model. Ruled out for this product.
- **Depth-free physics-adjacent methods** (UDCP, IBLA, GUDCP) estimate
  transmission/backscatter from image statistics instead of a real
  depth map, but are more fragile and complex than the option below
  for similar output quality.
- **Classical fusion** — Ancuti et al., "Color Balance and Fusion for
  Underwater Image Enhancement" (IEEE TIP, 2018) — needs no depth map
  and no training data: blends a white-balance/color-compensated
  branch with a contrast-enhanced branch via multi-scale (Laplacian
  pyramid) weighted fusion. Directly implementable from the paper now.
  **This is Phase 1a.**
- **ML restoration nets** (Water-Net, FUnIE-GAN, Ucolor, Deep SESR)
  mostly require curated paired real-underwater benchmark datasets
  (UIEB, UFO-120) that don't exist for us and are expensive to build
  solo. **UWCNN** and **Shallow-UWnet** are the exceptions — trainable
  on *synthetically* degraded in-air photos via the physics formation
  model, sidestepping the paired-dataset problem, and small enough for
  on-device ONNX inference. **This is Phase 1b.**

### Phase 1a — Classical fusion (Ancuti et al.)

Source: Ancuti, Ancuti, De Vleeschouwer, Bekaert, "Color Balance and
Fusion for Underwater Image Enhancement," IEEE TIP vol. 27 no. 1,
2018. Spec below is from the actual paper text (equation numbers
match), cross-checked against two independent MATLAB ports
(`fergaletto/...` and `bilityniu/underimage-fusion-enhancement`) —
deep-dive research pass, 2026-07-03. No training data required;
deterministic, testable with golden-image regression like the rest
of Phase 0.

**Algorithm — two branches from one white-balanced image `Iwb`:**

1. **Red-channel compensation** (Eq. 4), exploiting that green
   survives underwater attenuation best:
   `Irc(x) = Ir(x) + α·(Ῑg − Ῑr)·(1 − Ir(x))·Ig(x)`, `α = 1` (the
   paper's stated default). `Ῑr, Ῑg` are global per-channel means.
   The `(1 − Ir(x))` term suppresses correction on already-red pixels.
   Blue compensation (Eq. 5, same form) is **off by default** — the
   paper only enables it for high-turbidity scenes with strong blue
   attenuation.
2. **Gray-World white balance** on the compensated image, to remove
   residual illuminant cast — paper names the algorithm but doesn't
   spell out the equation; use plain per-channel mean-matching to
   global luma (simplest faithful reading, easiest to port to WGSL).
   Output: `Iwb`.
3. **Branch A (contrast):** gamma correction of `Iwb`. Paper doesn't
   state a numeric γ (treat as a tunable, not a derived constant —
   pick empirically against the fixture set in
   `engine/tests/fixtures/`).
4. **Branch B (sharpened):** normalized unsharp masking (Eq. 6):
   `S = (I + N{I − G*I}) / 2`, where `G*I` is a Gaussian blur and
   `N{}` is linear normalization/histogram-stretch — **not** the
   traditional `I + β(I − G*I)` unsharp mask. This is deliberate: it
   removes the fragile β tuning knob traditional unsharp masking
   needs. Preserve this exactly.

**Three weight maps per branch** (confirmed exactly 3, not 4 — see
pitfall below), combined per Eq. 7 + the paper's aggregation formula:

- **Laplacian contrast `WL`**: `|Laplacian(luminance(Ik))|` — an
  actual Laplacian-kernel convolution, not a color-deviation formula.
- **Saliency `WS`**: Achanta et al. frequency-tuned saliency on a
  Gaussian-blurred, Lab-converted branch:
  `WS(x) = (L(x)−Lm)² + (a(x)−am)² + (b(x)−bm)²`, global means
  subtracted, then normalized by max.
- **Saturation `WSat`** (Eq. 7, exact):
  `WSat = sqrt( ((Rk−Lk)² + (Gk−Lk)² + (Bk−Lk)²) / 3 )`.

Aggregate and normalize across the two branches:
`Wk = WLk + WSk + WSatk`,
`W̃k = (Wk + δ) / (ΣWk + K·δ)`, `δ = 0.1`, `K = 2`.

**Fusion — multi-scale Laplacian pyramid, not naive blending** (paper
explicitly rejects naive single-scale blending for halo artifacts):
decompose each branch into a Laplacian pyramid, each normalized
weight map into a Gaussian pyramid (same depth), fuse per-level as
`R_l(x) = Σk G_l{W̃k(x)} · L_l{Ik(x)}`, reconstruct coarsest-to-finest.
Pyramid depth `N` should be size-adaptive (paper: coarsest level's
short side lands in the tens-of-pixels range, e.g. `N ≈
floor(log2(min(H,W)/10)) + 1`) — both reference ports hardcode a
fixed depth instead; don't copy that, it won't generalize across the
range of photo resolutions this engine needs to handle.

**Known pitfalls** (from cross-checking the two reference ports
against the actual paper — worth internalizing before writing code,
not after debugging blind):

- The 2018 paper **drops the "exposedness" weight map** used in the
  authors' own 2012 CVPR predecessor. Three weight maps only. If any
  secondary source mentions a 4th weight for "the Ancuti underwater
  fusion paper," it's citing the 2012 paper.
- One reference port's "Laplacian contrast weight" is actually a
  copy-paste of its saturation-weight formula (no real Laplacian
  filter applied) — don't pattern-match implementation off that repo
  without checking the math actually differs between the two weights.
- Reference ports disagree with the paper's own stated α=1 for red
  compensation (one uses α=0.1, a 10x weaker correction) — use the
  paper's α=1 as the starting point, tune from there against real
  fixtures if it looks too aggressive on strobe-lit shots.
- Watch L* normalization range when converting to Lab for the
  saliency/saturation weights: CIE L* is [0,100], not [0,255] — a
  wrong divisor silently desaturates the saliency term relative to
  the RGB channels it's compared against.
- Gamma value and unsharp-mask blur σ are not numerically specified
  in the paper — both are tunables to set empirically, not constants
  to look up.

Exposed as a `ColorFilter` plugin, gated behind a license check stub
(real licensing lands in Phase 3).

**Checkpoint:** one-click fusion beats manual-baseline correction on a
blind comparison across 10+ real underwater stills/clips in different
water conditions (green Pacific, blue Caribbean, low-vis). This is
what Phase 3 actually sells — reaching this checkpoint unblocks
monetization without waiting on Phase 1b.

### Phase 1b — ML upgrade (UWCNN / Shallow-UWnet)
Deliberately sequenced after Phase 3 has real usage — don't build a
training pipeline speculatively.

- Synthetic training data: degrade clean in-air photos using the
  underwater image formation model (wavelength-dependent attenuation +
  backscatter) rather than sourcing real paired underwater images
- Train UWCNN or Shallow-UWnet (Shallow-UWnet is ~18x smaller, better
  ONNX/on-device fit) offline, export to ONNX, inference via ONNX
  Runtime (`ort` crate — cross-platform, and still dispatches through
  Core ML/ANE as an execution provider on macOS)
- Ships as an upgrade to the existing Phase 1a `ColorFilter` plugin,
  not a new licensing tier — same Pro entitlement covers both

**Checkpoint:** ML output beats Phase 1a fusion output on the same
blind-comparison clip set, by more than noise.

## Phase 2 — macOS app shell
Goal: first shippable product. Thin SwiftUI wrapper over
`underwater-core` — the app should contain almost no logic of its
own.

- UniFFI bindings + XCFramework packaging so Swift can call the Rust
  core — this is the one integration point where the two languages
  actually meet; everything upstream of it (Phase 0–1) is pure Rust
- Import → adjust → export flow (single clip/photo at a time, no
  batch yet)
- Manual controls from Phase 0 exposed as UI
- Notarized, direct-download `.dmg` (see [INFRA.md](INFRA.md)) —
  no App Store yet
- Sparkle for auto-updates

**Checkpoint:** you can hand the `.dmg` to 3–5 people in the
dive/underwater-videography community and they can use it without
you sitting next to them.

## Phase 3 — Licensing + Pro tier
Goal: turn Phase 1's auto-correct into an actual paid feature.

- License-key entitlement table in Supabase (Deep Sea Labs org, see
  [INFRA.md](INFRA.md))
- Payment via Paddle or Lemon Squeezy (merchant-of-record — avoids
  you handling VAT/sales tax solo)
- Free tier = Phase 0 manual engine. Pro tier = ML auto-correct +
  premium LUT packs.
- See [BUSINESS_MODEL.md](BUSINESS_MODEL.md) for pricing.

**Checkpoint:** one real stranger (not a friend, not a dive buddy)
buys a license.

## Phase 4 — Open-source the core engine
Goal: open-core as the README states, once there's something worth
building trust around.

- Apache 2.0 release of `underwater-core` (Phase 0 scope only — ML
  model and cloud stay proprietary)
- Public repo hygiene: CONTRIBUTING.md, issue templates, a couple of
  example third-party filter plugins to prove the plugin API is
  actually usable by someone who isn't you
- Deliberately sequenced *after* Phase 3 — open-sourcing before
  there's a paid tier means giving away the whole product with
  nothing to fund maintenance

**Checkpoint:** someone outside Deep Sea Labs opens a PR that isn't
a typo fix.

## Phase 5 — iOS app shell
Goal: on-the-go review/quick edits, once macOS has proven the engine
and the business model.

- Same `underwater-core` package, new thin SwiftUI shell
- Reuse the Phase 3 licensing/entitlement backend
- Photo-focused first (video on iOS is a much heavier lift —
  don't commit to it here, revisit based on Phase 2–4 usage data)

## Phase 6 — Cloud grading service
Goal: the recurring-revenue layer. Deliberately last — it's the only
phase with ongoing infra cost, so it shouldn't start until there's
paying-customer evidence to justify that cost.

- Server-side rendering (GPU-backed) for the ML auto-correct + batch
  export, sold as a subscription (usage has a marginal cost, so it
  can't be a one-time fee like Phase 3)
- Shared Deep Sea Labs Supabase org, new project (not shared schema
  with Currents — see [INFRA.md](INFRA.md))

## Phase 7 — App Store distribution
Goal: add the App Store as a second channel once direct-download has
proven the product, not as the first channel.

- Mac App Store build (sandboxing changes may be needed —
  re-audit entitlements)
- iOS App Store submission (Phase 5 app)
- Existing Pro-tier license holders should not have to re-buy —
  plan the entitlement bridge in Phase 3, not here

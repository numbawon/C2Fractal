# TODO — C2Fractal

_Handoff from 2026-11-09 session. Branch: `phase-4-gpu` (local, unpushed). Milestone status in the c2fractal skill + agent memory._

## Where things stand

- **P1 bugfixes** — merged. **P2 UI** — merged (#7). **P3 CI** — merged (#8).
- **P4 — all code committed on `phase-4-gpu`, NOT yet pushed.** Commits, in order:
  - `c5b9478` 4A: eframe 0.37 / wgpu 30 bump + GPU backend probe
  - `d053b26` 4B/4C: wgpu paint callback — fullscreen WGSL shader, per-frame `GpuParams`
  - `9d17eb2` GPU (f32) zoom capped at 500× (`MAX_GPU_ZOOM`, `src/main.rs:86`), CPU f64 fallback beyond it
  - `735398f` window-aware control bar (wraps on narrow windows) + `661cc85` merge
  - `de071bc` window-size persistence (eframe) + `8142eab` merge
  - `78caeca` control bar overflow fix — exact egui footprints (`control_group`/`slider_footprint`/`FRACTAL_COMBO_WIDTH`); **Claude cross-review verdict: approve** (2 nits, non-blocking, recorded in `scratch/claude-review-wrapfix-verdict.json`)
- Working tree clean. Push gate green on the current tree (2026-11-09, `gate.bat`: fmt / check -D warnings / clippy / fmt --check, ARM64 MSVC).
- **GPU-in-the-loop pixel test (2026-11-08, decisive, default view only):** faithful f32 emulation of the exact shader math for all 1.75M texture pixels → 99.914% exact vs faithful-f32 model, 99.906% vs f64 ground truth; residual = isolated boundary pixels where GPU FMA contraction changes orbits (expected, not a defect). The earlier smoke "PASS, mean_abs_diff 7–12" was a **geometry artifact** (reference rendered at 1600×1134 vs texture 1568×1118 on that session's window).
- **Only P4 work left: GPU-mode render verification** (below). Then push at the milestone boundary (user go-ahead required; merge only on explicit go-ahead).

## Serena MCP server — installed & registered (done this session)

- [x] **Serena 2.1.0** (https://github.com/oraios/serena, "the IDE for your coding agent", GPL-3.0-or-later) is installed as a uv tool (`serena-agent` on PyPI; plain `serena` is an unrelated AMQP client) and registered in Hermes under `mcp_servers.serena` (`~/.hermes/config.yaml`).
- Exposes **29 semantic tools** (tools interface): `mcp_serena_*` — find_symbol, get_symbols_overview, replace_symbol_body, rename_symbol, get_diagnostics_for_file, execute_shell_command, memories, … Use `activate_project` with the repo path first in a session, and `initial_instructions` for the usage manual.
- **ARM64-Windows landmines (already defused, do not re-break):**
  - `cryptography 50.0.0` has **no win_arm64 wheel** — was built from sdist against vcpkg OpenSSL 3.6.5 (`arm64-windows` triplet).
  - uv's managed CPython ships OpenSSL **3.5.9** in its `DLLs\` dir and registers it with the loader at startup; the uv tool shim's process therefore resolved `libcrypto-3-arm64.dll` to 3.5.9 → `_rust` import failed 127 (procedure not found). Fix: `sitecustomize.py` in the tool's site-packages (`AppData/Roaming/uv/tools/serena-agent/Lib/site-packages/`) preloads the 3.6.5 DLLs by absolute path from `cryptography/hazmat/bindings/` at interpreter startup. **If serena ever re-installs/upgrades, re-verify `~/.local/bin/serena.exe --version`.**
  - Hermes MCP stdio env is filtered (no USERPROFILE) — `env: {USERPROFILE, HOME}` is set in the server config; `Path.home()` needs it.
- Residual (benign): "OpenSSL 3's legacy provider failed to load" warning on stderr — legacy algorithms unavailable, no impact on Serena.

## GPU-mode verification — the remaining P4 work

### Geometry (CONFIRMED 2026-11-09 — use these)

- `cap_now.png` in scratch = valid full-window-frame capture, **1000×800 buffer px**, via PrintWindow PW_RENDERFULLCONTENT (`cap2.py`, takes an output path). **DWM thumbnail API is broken on this ARM64 host (returns pointer 1) — never use it.**
- Client origin in buffer coords: **(L=13, T=58)**; client area 974×729. (The 13-px "black strip" in captures is the window's left border, not a render artifact.)
- Shader mapping (`src/gpu.rs:133-137`): pixel index i from texture origin → `cx = (i − W/2)/(z·W/2) + center_x` (y analogous; screen-down = +cy — invisible for the symmetric Mandelbrot).
- Texture rect (`src/main.rs:429-450`): CentralPanel `available_rect_before_wrap()` × `pixels_per_point()`. With 8pt margins + the 2-row bar at ppu=2.0: expected client origin **(16, 158)**, **W=942, H=555**.
- Palette = Bernstein basis (`src/main.rs:65-76`): interior renders **exactly (0,0,0)**; near-boundary exterior is dark blue ((12,90,218) class). **Set mask = exactly (0,0,0)**; a ≤12 threshold is contaminated.
- `MAX_GPU_ZOOM = 500` → manifest `mandelbrot_deepzoom` (z=400) runs on the GPU path (that's what we're verifying).

### Anchor facts the prior session had WRONG (do not reuse)

- Cusp (rightmost point) is at **(+0.25, 0)** — not −0.25.
- Bounding box: Re ∈ [−2, 0.25], Im ∈ [−1.1226, 1.1226].
- Real-axis structure: antenna [−2, −1.75], bulb [−1.75, −0.75], cardioid [−0.75, +0.25]; junction at (−0.75, 0).
- Topmost point of the set ≈ (0, ±1.1226) — NOT (−0.75, 1.1226). Cardioid's own top: (−0.125, 0.6495).

### Open problem: the currently-running app is NOT at the default view

`cap_now.png`'s features don't fit c=(−0.5, 0), z=1 under any origin assumption (Im=0-row anchors imply z≈0.74–1.1; the topmost blob at client (701, 162) contradicts every candidate anchor; silhouette dump in `scratch/silhouette.txt`). Likely a stale interaction pre-compaction or a wrong geometry assumption. **Do not keep fitting that capture.**

### Next-session plan (in order)

- [ ] Kill the running app (PID 28000 / hwnd 459662 if still alive), relaunch, capture **immediately** (no interaction) → by construction the default view (c=(−0.5, 0), z=1, maxiter=100, offset=0 — matches `FractalParams` defaults and `mode_refs/manifest.json` exactly).
- [ ] **Invert** the geometry: derive texture origin/W/H in buffer px from KNOWN features of the default view — cusp (0.25, 0) = rightmost (0,0,0) pixel; junction (−0.75, 0) = gap center on the widest (0,0,0) row; widest row = Im=0; bulb top (−1.5, 0.5) for the y-scale. Verify H≈555, origin≈(16, 158) before trusting the comparison.
- [ ] `mode_test.py --capture <fresh capture> --tex <top,left,w,h>` against the `mandelbrot` default reference in `mode_refs/` — first real GPU-vs-reference pixel comparison on the current window.
- [ ] Deep zoom: read the pan/zoom handlers in `src/main.rs` (Scroll/Dragged/DoubleClicked) for the exact zoom factor + zoom center, then drive the app to `mandelbrot_deepzoom` (c=(−1.749, 0), z=400), capture, compare with its reference.
- [ ] Non-default modes (4.4): Julia, BurningShip, `color_offset ≠ 0`, changed maxiter — same capture/compare flow.
- [ ] Full GUI smoke with GPU active (pan, wheel/double-click zoom, sliders, Julia toggle, Save PNG, Reset) — the 2026-11-06/07 CPU-path assertions are stale.
- [ ] Update the `c2fractal` skill milestone to P4 done; then push (user go-ahead required; merge only on explicit go-ahead).

Scratch inventory: `cap2.py` (PrintWindow capture), `mode_test.py` (reference comparison, `--tex` flag), `mode_refs/` (manifest + reference PNGs), `png_compare.py`, `dump_silhouette.py` / `fit_view.py` (view analysis — superseded by the invert step), `silhouette.txt`, `app.log`, `gate-run.log`, `claude-review-wrapfix-verdict.json`.

## Phase 4B — GPU paint callback (`src/gpu.rs`) — done, committed (`d053b26`)

- [x] `init()`: resources built + cached in `callback_resources` (shader module, pipeline layout, bind group layout, pipeline, uniform buffer, bind group)
- [x] `prepare()`: `queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&self.params))`
- [x] `paint()`: `set_pipeline` / `set_bind_group` / fullscreen-triangle draw
- [x] WGSL uniform struct mirrors the frozen `GpuParams` `#[repr(C)]` layout field-for-field (verified via uniform-buffer readback)
- [x] Fragment shader mirrors the CPU iteration/palette math (Mandelbrot/Julia/BurningShip, `color_offset` palette, center/zoom → complex-plane mapping) — verified pixel-wise for the default Mandelbrot view

## Phase 4C — wire into `main.rs` — done, committed (`d053b26`)

- [x] Register `GpuFractal` per frame; push `GpuParams` from live UI state each frame
- [x] Keep CPU renderer as the fallback
- [x] Remove the three `#[allow(dead_code)] // wired into main.rs in Phase 4C` annotations
- [x] Verify GPU output against CPU — done via the in-the-loop pixel test (see "Where things stand"); remaining mode gaps listed in "Next-session plan"
- [ ] Re-run full GUI smoke test with the GPU path active

## Standing rules (recap)

- Push gate mechanics in the `c2fractal` skill; **GATE_DONE is not green** — tail all three stage logs.
- Push only at milestone boundaries; merge only on explicit user go-ahead.
- No agent scratch/build folders in the repo — finished artifacts only.
- Reverts never rewrite history.

## After P4

- P5: dependency refresh (per milestone plan).

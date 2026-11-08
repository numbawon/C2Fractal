# TODO — C2Fractal

_Handoff from 2026-11-08 session. Branch: `phase-4-gpu`. Milestone status in the c2fractal skill + agent memory._

## Where things stand

- **P1 bugfixes** — merged. **P2 UI** — merged (#7). **P3 CI** — merged (#8).
- **P4 — 4A committed (`c5b9478`), 4B + 4C implemented and verified, not yet committed.**
- Push gate green on the current working tree (2026-11-08): fmt / check `-D warnings` / clippy / fmt --check, ARM64 MSVC.
- Runtime verification (2026-11-08): GPU backend active and rendering; uniform buffer values verified against shader expectations; colormap identical to CPU.
- **GPU-in-the-loop pixel test (decisive):** for all 1,753,024 texture pixels, faithful step-by-step f32 emulation of the exact shader math (pixel→complex mapping, iteration semantics, Bernstein-basis colormap) → **99.914% exact match vs faithful-f32 model, 99.906% vs f64 ground truth, 99.938% within L1≤3 of either.** Iteration deltas: d=0 at 99.907%, symmetric tail (±40 max, ~1,200 px) confined to the chaotic boundary. Residual 0.06% = isolated boundary pixels where GPU FMA contraction changes the orbit — expected f32 hardware behavior, not a defect.
- The earlier smoke "PASS, mean_abs_diff 7–12, 13% beyond tolerance" was a **geometry artifact**: the f64 reference was 1600×1134 while the GPU texture is 1568×1118 (offset (16,0) in the capture; 16+1568+16=1600, 66+1118+16=1200, 1:1 device pixels) — a 2.0%/1.4% stretch, up to ~32 px misalignment. Cross-check: resampling the reference onto the correct grid → per-channel mean 1.75–2.28, ~3% beyond 10 (boundary blending only).
- **Uncommitted working tree:** `src/gpu.rs` (+262 — `GpuFractal` CallbackTrait: init/prepare/paint, fullscreen WGSL shader, frozen `GpuParams` mirror; double-wrap fix), `src/main.rs` (+65/−48 — per-frame `GpuFractal` registration at main.rs:354, PrintWindow DC-sizing fix in capture path, CPU fallback kept, `dead_code` annotations gone).

## Serena MCP server — installed & registered (done this session)

- [x] **Serena 2.1.0** (https://github.com/oraios/serena, "the IDE for your coding agent", GPL-3.0-or-later) is installed as a uv tool (`serena-agent` on PyPI; plain `serena` is an unrelated AMQP client) and registered in Hermes under `mcp_servers.serena` (`~/.hermes/config.yaml`).
- Exposes **29 semantic tools** (tools interface): `mcp_serena_*` — find_symbol, get_symbols_overview, replace_symbol_body, rename_symbol, get_diagnostics_for_file, execute_shell_command, memories, … Use `activate_project` with the repo path first in a session, and `initial_instructions` for the usage manual.
- **ARM64-Windows landmines (already defused, do not re-break):**
  - `cryptography 50.0.0` has **no win_arm64 wheel** — was built from sdist against vcpkg OpenSSL 3.6.5 (`arm64-windows` triplet).
  - uv's managed CPython ships OpenSSL **3.5.9** in its `DLLs\` dir and registers it with the loader at startup; the uv tool shim's process therefore resolved `libcrypto-3-arm64.dll` to 3.5.9 → `_rust` import failed 127 (procedure not found). Fix: `sitecustomize.py` in the tool's site-packages (`AppData/Roaming/uv/tools/serena-agent/Lib/site-packages/`) preloads the 3.6.5 DLLs by absolute path from `cryptography/hazmat/bindings/` at interpreter startup. **If serena ever re-installs/upgrades, re-verify `~/.local/bin/serena.exe --version`.**
  - Hermes MCP stdio env is filtered (no USERPROFILE) — `env: {USERPROFILE, HOME}` is set in the server config; `Path.home()` needs it.
- Residual (benign): "OpenSSL 3's legacy provider failed to load" warning on stderr — legacy algorithms unavailable, no impact on Serena.

## Up next (in order)

- [ ] Commit 4B+4C. Suggested message: `Phase 4B/4C: wgpu paint callback — fullscreen WGSL fractal shader with per-frame GpuParams push`
- [ ] Push `phase-4-gpu` at the P4 milestone boundary — **awaiting user go-ahead**; merge only on explicit go-ahead.
- [ ] Re-run the full GUI smoke test with the GPU path active: pan, wheel/double-click zoom, iterations + color-offset sliders, Julia switch, Save PNG, Reset. (The 2026-11-07 smoke ran the CPU path; GPU is now the renderer.)
- [ ] Verify GPU shader in the non-default modes the in-the-loop test did not cover: Julia, BurningShip, `color_offset ≠ 0`, changed iteration count, deep zoom (f32 caps zoom depth vs CPU f64 — decide on a guard or a documented expectation).
- [ ] Fix the smoke harness geometry so future CPU↔GPU comparisons are valid: render the f64 reference on the texture's own geometry, or crop the capture to the texture rect (offset (16,0), 1568×1118 device px).
- [ ] Update the `c2fractal` skill milestone status to P4 done (memory line already updated).

## Phase 4B — GPU paint callback (`src/gpu.rs`) — done, uncommitted

- [x] `init()`: resources built + cached in `callback_resources` (shader module, pipeline layout, bind group layout, pipeline, uniform buffer, bind group)
- [x] `prepare()`: `queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&self.params))`
- [x] `paint()`: `set_pipeline` / `set_bind_group` / fullscreen-triangle draw
- [x] WGSL uniform struct mirrors the frozen `GpuParams` `#[repr(C)]` layout field-for-field (verified via uniform-buffer readback)
- [x] Fragment shader mirrors the CPU iteration/palette math (Mandelbrot/Julia/BurningShip, `color_offset` palette, center/zoom → complex-plane mapping) — verified pixel-wise for the default Mandelbrot view

## Phase 4C — wire into `main.rs` — done, uncommitted

- [x] Register `GpuFractal` per frame; push `GpuParams` from live UI state each frame
- [x] Keep CPU renderer as the fallback
- [x] Remove the three `#[allow(dead_code)] // wired into main.rs in Phase 4C` annotations
- [x] Verify GPU output against CPU — done via the in-the-loop pixel test (see "Where things stand"); remaining mode gaps listed in "Up next"
- [ ] Re-run full GUI smoke test with the GPU path active

## Standing rules (recap)

- Push gate mechanics in the `c2fractal` skill; **GATE_DONE is not green** — tail all three stage logs.
- Push only at milestone boundaries; merge only on explicit user go-ahead.
- No agent scratch/build folders in the repo — finished artifacts only.
- Reverts never rewrite history.

## After P4

- P5: dependency refresh (per milestone plan).

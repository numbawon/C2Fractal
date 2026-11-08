# TODO — C2Fractal

_Handoff from 2026-11-07 session. Branch: `phase-4-gpu`. Milestone status in the c2fractal skill + agent memory._

## Where things stand

- **P1 bugfixes** — merged. **P2 UI** — merged (#7). **P3 CI** — merged (#8).
- **P4 GPU — Phase 4A done, 4B next.** eframe 0.27 → 0.37, wgpu → 30.0.1, and the `src/gpu.rs` paint-callback scaffold are in place on `phase-4-gpu`.
- Push gate green (check / clippy `-D warnings` / fmt).
- GUI smoke test passed 2026-11-07: window launches under eframe 0.37, Mandelbrot renders (CPU path), probe prints `GPU backend active (callback wiring: Phase 4C)`, full UIA element surface present. Pan, wheel/double-click zoom, iterations + color-offset sliders, Julia switch, Save PNG, Reset — all manually verified by the user, no issues.
- **Uncommitted working tree:** `Cargo.toml`, `Cargo.lock`, `src/main.rs` (`ui.ctx()` inlined — see upgrade notes), `src/gpu.rs` (new).

## Serena MCP server — installed & registered (done this session)

- [x] **Serena 2.1.0** (https://github.com/oraios/serena, "the IDE for your coding agent", GPL-3.0-or-later) is installed as a uv tool (`serena-agent` on PyPI; plain `serena` is an unrelated AMQP client) and registered in Hermes under `mcp_servers.serena` (`~/.hermes/config.yaml`).
- Exposes **29 semantic tools** (tools interface): `mcp_serena_*` — find_symbol, get_symbols_overview, replace_symbol_body, rename_symbol, get_diagnostics_for_file, execute_shell_command, memories, … Use `activate_project` with the repo path first in a session, and `initial_instructions` for the usage manual.
- **ARM64-Windows landmines (already defused, do not re-break):**
  - `cryptography 50.0.0` has **no win_arm64 wheel** — was built from sdist against vcpkg OpenSSL 3.6.5 (`arm64-windows` triplet).
  - uv's managed CPython ships OpenSSL **3.5.9** in its `DLLs\` dir and registers it with the loader at startup; the uv tool shim's process therefore resolved `libcrypto-3-arm64.dll` to 3.5.9 → `_rust` import failed 127 (procedure not found). Fix: `sitecustomize.py` in the tool's site-packages (`AppData/Roaming/uv/tools/serena-agent/Lib/site-packages/`) preloads the 3.6.5 DLLs by absolute path from `cryptography/hazmat/bindings/` at interpreter startup. **If serena ever re-installs/upgrades, re-verify `~/.local/bin/serena.exe --version`.**
  - Hermes MCP stdio env is filtered (no USERPROFILE) — `env: {USERPROFILE, HOME}` is set in the server config; `Path.home()` needs it.
- Residual (benign): "OpenSSL 3's legacy provider failed to load" warning on stderr — legacy algorithms unavailable, no impact on Serena.

## First act next session

- [ ] Commit the 4A checkpoint (bump + scaffold + ctx fixes). Suggested message: `Phase 4A: eframe 0.37 / wgpu 30 bump with GPU backend probe`
- [ ] Then start 4B below.

## Phase 4B — GPU paint callback (`src/gpu.rs`, per its Part B markers)

- [ ] `init()`: from `cc.wgpu_render_state` build + cache in `render_state.renderer.write().callback_resources`: WGSL shader module, pipeline layout, bind group layout, pipeline, uniform buffer, bind group
- [ ] `prepare()`: `queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&self.params))`
- [ ] `paint()`: `set_pipeline` / `set_bind_group` / `draw(0..3, 0..1)` fullscreen triangle
- [ ] WGSL uniform struct must mirror the **frozen** `GpuParams` `#[repr(C)]` layout field-for-field (order, names, types) — do not touch the Rust struct's layout
- [ ] Fragment shader mirrors the CPU iteration/palette math (type 0/1/2 = Mandelbrot/Julia/BurningShip, `color_offset` palette, center/zoom → complex-plane mapping from `origin_x/y`, `width/height`)

## Phase 4C — wire into `main.rs`

- [ ] Register `GpuFractal` per frame; push `GpuParams` from live UI state each frame
- [ ] Keep CPU renderer as the fallback (the 4A probe already selects the path)
- [ ] Remove the three `#[allow(dead_code)] // wired into main.rs in Phase 4C` annotations once wiring lands
- [ ] Verify GPU output against CPU: same view/params → compare saved PNGs pixel-wise; then re-run the full GUI smoke test

## Standing rules (recap)

- Push gate mechanics in the `c2fractal` skill; **GATE_DONE is not green** — tail all three stage logs.
- Push only at milestone boundaries; merge only on explicit user go-ahead.
- No agent scratch/build folders in the repo — finished artifacts only.
- Reverts never rewrite history.

## After P4

- P5: dependency refresh (per milestone plan).

# handoff.md — FanForge
> Written: 2026-05-19 | Branch: main | Phase: 0 complete → Phase 1 next

---

## Goal

Build **FanForge** — a Telegram bot backed by an Aomi-native Rust plugin that wraps Zora's creator coin economy. A non-crypto-native music creator types one sentence and gets a live fan coin on Base. Everything downstream — leaderboards, missions, reward delivery, social recaps — is automated.

Stack: Rust cdylib (aomi-sdk 0.1.19), TypeScript/grammY (Node.js 22), Supabase (state), Zora Protocol API, `@aomi-labs/client 0.1.37`. Deadline: May 30.

---

## What Was Done — Phase 0 (2026-05-19)

### Infrastructure built
- Rust workspace: `plugin/` with `cdylib` crate, aomi-sdk wired, all 5 tools registered.
- TypeScript bot: `bot/` with grammY, `@aomi-labs/client` Session, `/start` flow.
- `.gitignore`: excludes `target/`, `node_modules/`, `.env`, `Cargo.lock`.
- `.env.example`: all 8 env vars documented with comments.
- `CLAUDE.md`: living operational guide — tech stack, architecture rules, do-nots, commands.
- `GOAL.md`: initial architecture design document.
- `handoff.md`: this file.

### Build status
- `cargo check` (plugin): ✅ passes with 2 expected warnings (unused `zora_post`, unused `description`/`image_url` fields — both used in Phase 1).
- `tsc --noEmit` (bot): ✅ passes clean.

### Files created this session
| File | Purpose |
|------|---------|
| `plugin/Cargo.toml` | cdylib crate, aomi-sdk + reqwest + tokio deps |
| `plugin/src/lib.rs` | dyn_aomi_app! macro, PREAMBLE, 3 secrets declared |
| `plugin/src/client.rs` | FanForgeApp, Zora/Supabase HTTP helpers, 5 Args structs |
| `plugin/src/tool.rs` | 5 DynAomiTool impls (1 stub, 4 partially live) |
| `bot/package.json` | grammy, @aomi-labs/client, tsx, biome deps |
| `bot/tsconfig.json` | strict TS, NodeNext modules |
| `bot/biome.json` | Biome 1.9, spaces, recommended rules |
| `bot/src/index.ts` | Bot init, message relay, chunked reply, error handler |
| `bot/src/session.ts` | Map<userId, Session>, getOrCreateSession, closeSession |
| `bot/src/handlers/start.ts` | /start + /help: plain-English welcome, no crypto jargon |
| `.env.example` | All 8 env vars documented |
| `.gitignore` | target/, node_modules/, .env, Cargo.lock, .agents/ |
| `CLAUDE.md` | Main Claude Code operational guide |
| `GOAL.md` | Initial architecture document |

---

## Key Discoveries (Read Before Phase 1)

### 1. Aomi SDK is a hot-loaded cdylib, not an HTTP server
The plugin compiles to a `.so` shared library. The Aomi runtime loads it dynamically. There is no `main.rs`, no `tokio` runtime owned by the plugin process, and no long-lived connection pool. Each tool call is a synchronous `run()` invocation. This is why Supabase REST API (stateless HTTP per call) is used for state instead of PostgreSQL + connection pool.

### 2. `SessionOptions.app`, not `namespace`
The README for `@aomi-labs/client` shows `namespace` in an example, but the actual TypeScript type is `app`. The env var is `AOMI_APP`.

### 3. `AomiMessage.sender`, not `role`
`AomiMessage` has `sender?: "user" | "agent" | "system"`. There is no `.role` property. The bot filters for `sender === "agent"` to get the AI's response.

### 4. Existing `apps/zora` in the SDK
The Aomi SDK repo already has a `zora` plugin with 6 read tools (`zora_get_coin`, `zora_get_coin_holders`, `zora_get_coin_price_history`, `zora_get_trends_by_name`, `zora_get_featured_creators`, `zora_get_profile`). FanForge is a SEPARATE plugin with a `fanforge_` prefix — it adds creator-specific tools (launch, missions, recap) on top of what the existing Zora plugin provides.

### 5. Zora coin creation is NOT in the existing SDK
The existing `apps/zora` only has read tools. Coin launch (POST to create a new ERC-20 via ZoraFactory) is not implemented. Phase 1 must wire this from scratch.

---

## Current State

### What works end-to-end
- `cargo check` compiles the plugin with all 5 tools registered.
- Bot starts with `npm run dev` and responds to `/start`.
- `GetFanLeaderboard` calls the real Zora `/coinHolders` endpoint (tested against compile types, not a live call yet).
- `CreateFanMission` and `DistributeRewards` call Supabase REST (Supabase tables not yet created — need to run the SQL in CLAUDE.md §8 before Phase 3 testing).
- `GetCreatorRecap` calls Zora `/coin` + Supabase missions.

### What does NOT work yet
- `LaunchFanCoin` is a stub — returns placeholder JSON. Phase 1 wires the real Zora coin creation.
- No Telegram bot token or Aomi API key is set — bot can't start without `.env`.
- Supabase tables don't exist yet — Phase 3 tools will error until `fan_missions` and `reward_distributions` are created.
- Plugin not registered with Aomi yet — bot can't route messages to it.

---

## Known Remaining Tech Debt

| # | Issue | When |
|---|-------|------|
| 1 | `LaunchFanCoin` is a stub | Phase 1 |
| 2 | Supabase tables not created | Before Phase 3 testing |
| 3 | Plugin not registered with Aomi | Before end-to-end testing |
| 4 | No bot conversation state machine | Phase 1 (multi-turn coin launch flow) |
| 5 | `zora_post` in client.rs unused | Phase 1 (coin creation will use it) |
| 6 | No `.env` file created | User must copy `.env.example` → `.env` and fill in tokens |

---

## Next Step — Phase 1: Coin Launch

Goal: Telegram message → real Zora coin live on Base → bot replies with the link.

**Steps:**
1. Research the Zora coin creation API. Check: `https://api-sdk.zora.engineering` OpenAPI spec or Zora SDK docs. The existing `apps/zora` PREAMBLE mentions Uniswap V4 and `ZoraFactory` — find the correct POST endpoint or on-chain call.
2. Implement `launch_fan_coin` in `tool.rs`:
   - Validate ticker (3–5 chars, not taken)
   - POST to Zora factory (via `zora_post` helper in `client.rs`, or via the `evm-core` host execution path if it requires signing)
   - Store the coin in Supabase `creator_coins` table (need to create this table too)
   - Return `{ coin_address, zora_url, ticker, name }`
3. Add `creator_coins` table to Supabase schema.
4. Wire bot conversation flow in `bot/src/handlers/launch.ts`:
   - Bot asks for coin name → ticker → description (multi-turn via grammY conversation API or simple state machine)
   - Calls Aomi with the full prompt once all info is collected
5. Gate: Telegram message → real coin live on Zora (or Zora testnet).

**Critical question to answer first:** Does Zora coin creation require on-chain signing (which goes through Aomi's `evm-core` host execution path) or does Zora have a server-side API endpoint for it? Read `apps/zora` PREAMBLE — it mentions `ZoraFactory` and Uniswap V4. The coin might need to be deployed on-chain, which means the Aomi host tools (`stage_tx`, `simulate_batch`, `commit_txs`) are involved, NOT a simple HTTP POST.

If it requires signing: `LaunchFanCoin` tool returns a `ToolReturn::with_routes(...)` envelope instead of bare JSON, and the bot's Aomi session handles the wallet request event.

---

## Files in Flight

None. All Phase 0 changes are committed.

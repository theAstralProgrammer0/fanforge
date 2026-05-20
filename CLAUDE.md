# CLAUDE.md — FanForge
> Last updated: 2026-05-20

## 1. Project Identity

- **Purpose:** Aomi-native on-chain fan economy platform for non-crypto-native music creators. Wraps Zora creator coin primitives as agentic tools so a creator can launch a fan coin, automate holder rewards, and get daily recaps — all in plain English via Telegram.
- **Persona:** Temi Falola — 26-year-old Nigerian musician, 80k TikTok followers, zero crypto knowledge.
- **Protocol:** Zora (on Base mainnet, chain ID 8453).
- **Hackathon:** OpenPandora Early Forge — May 20–30 deadline.
- **Stage:** Active build. Phase 1 (coin launch) in progress.

## 2. Tech Stack

- **Rust plugin:** Rust 1.95 (edition 2024), `aomi-sdk 0.1.19`, `reqwest 0.13` (blocking), `schemars 1`, `serde`/`serde_json`, `tokio 1`.
  - Crate type: `cdylib` — compiles to a `.so` hot-loaded by the Aomi runtime. NOT a binary, NOT an HTTP server.
- **Telegram bot:** TypeScript, Node.js 22, `grammy 1.43`, `@aomi-labs/client 0.1.37`, `dotenv`.
  - Dev runner: `tsx watch` (no build step needed in dev).
  - Type checking: `tsc --noEmit` (strict mode).
  - Lint/format: Biome 1.9.
- **State store:** Supabase (free tier). Accessible via REST API (`SUPABASE_URL` + `SUPABASE_ANON_KEY`) from inside the cdylib. No local PostgreSQL or Redis — a connection pool doesn't survive Aomi plugin hot-reloads.
- **Blockchain:** Base mainnet. Zora Protocol API (`https://api-sdk.zora.engineering`).
- **Agent runtime:** Aomi cloud. Plugin registered under app name `fanforge`. Bot connects via `@aomi-labs/client` `Session` class.
- **Package manager:** `cargo` (Rust), `npm` (TypeScript). Never mix.

## 3. Project Structure

```
/
├── CLAUDE.md                  ← This file. Living operational guide.
├── GOAL.md                    ← Initial architecture design document.
├── handoff.md                 ← Session-to-session handoff.
├── .env.example               ← All env vars documented. Copy to .env.
├── .gitignore
├── plugin/                    ← Rust cdylib Aomi plugin
│   ├── Cargo.toml             ← crate-type = ["cdylib"], aomi-sdk dep
│   └── src/
│       ├── lib.rs             ← dyn_aomi_app! macro, PREAMBLE, secret declarations
│       ├── client.rs          ← FanForgeApp struct, Zora/Supabase HTTP helpers, Args structs
│       └── tool.rs            ← 5 DynAomiTool implementations
└── bot/                       ← TypeScript Telegram bot
    ├── package.json
    ├── tsconfig.json
    ├── biome.json
    └── src/
        ├── index.ts           ← Bot init, message relay to Aomi, chunked replies
        ├── session.ts         ← One Session per Telegram user ID (Map<number, Session>)
        └── handlers/
            └── start.ts       ← /start and /help command handler
```

## 4. The 7 Agentic Tools

Every tool is in `plugin/src/tool.rs`. Tool handler: validate args → call service logic → return `ok(json!({...}))` or `ToolReturn::route(...)`.

| Tool | NAME constant | Phase | Status |
|------|--------------|-------|--------|
| LaunchFanCoin | `fanforge_launch_fan_coin` | 1 | Live — routes to get_account_info → BuildCoinTx |
| BuildCoinTx *(internal)* | `fanforge_build_coin_tx` | 1 | Live — calls Zora /create/content + stages tx |
| FinalizeLaunch *(internal)* | `fanforge_finalize_launch` | 1 | Live — stores in Supabase, returns Zora URL |
| GetFanLeaderboard | `fanforge_get_fan_leaderboard` | 2 | Live — calls Zora `/coinHolders` |
| CreateFanMission | `fanforge_create_fan_mission` | 3 | Live — writes to Supabase `fan_missions` |
| DistributeRewards | `fanforge_distribute_rewards` | 3 | Live — reads holders + writes Supabase |
| GetCreatorRecap | `fanforge_get_creator_recap` | 4 | Live — Zora `/coin` + Supabase missions |

### Coin launch route chain (discovered Phase 1)
```
LaunchFanCoin
  → route: get_account_info (binds "creator_wallet")
  → after: fanforge_build_coin_tx (receives wallet injected)
      → route: stage_tx (enforce: simulate_batch → commit_txs binds "transaction_hash")
      → after: fanforge_finalize_launch (receives tx_hash injected)
          → stores creator_coins in Supabase
          → returns { zora_url, coin_address, ... }
```
- Zora coin creation: `POST https://api-sdk.zora.engineering/create/content` returns `{calls:[{to,data,value}], predictedCoinAddress}`
- `data` for `stage_tx` must be in `{"raw": "0x..."}` object format
- Internal tools have `DESCRIPTION` starting with "Internal tool — Do not call directly."

### Critical SDK facts (discovered Phase 0 — do not re-learn)
- `AomiMessage.sender` is `"user" | "agent" | "system"`. NOT `.role`. NOT `"assistant"`.
- `SessionOptions.app` selects the plugin. NOT `namespace`. The env var is `AOMI_APP`.
- Tools can override `run_with_routes` (returns `ToolReturn`) for routing flows; `run` (returns `Value`) for simple tools.
- `dyn_aomi_app!` macro generates the C ABI exports (`aomi_create`, `aomi_manifest`, etc.).
- Secrets declared in `dyn_aomi_app!` via `secrets = [...]` are injected at call time via `ctx.secrets`.

## 5. Architecture Rules

1. **Tools are the only execution surface.** The Telegram bot never calls Zora or Supabase directly. All on-chain and state interactions route through a tool in `plugin/src/tool.rs`.
2. **Business logic in tools, HTTP in client.rs.** Tool handlers validate args and return output. HTTP calls to Zora and Supabase live in helper functions in `client.rs`.
3. **No crypto jargon in user-facing strings.** The PREAMBLE in `lib.rs` defines the rule. Forbidden terms: `deploy`, `contract`, `wallet address`, `ERC-20`, `on-chain`, `mint`, `gas`, `transaction hash`. Replacements: `fan coin`, `supporters`, `fan economy`, `exclusive access`, `unlock`.
4. **Idempotent writes.** Every Supabase write is safe to retry. `reward_distributions` has a `UNIQUE(mission_id, recipient_wallet)` constraint — the insert will fail cleanly on a duplicate rather than requiring a pre-check.
5. **Wallet delegation, not custody.** FanForge never stores or logs private keys. Aomi handles signing via `evm-core` namespace.
6. **One Aomi Session per Telegram user.** `session.ts` maintains a `Map<number, Session>`. A new session is created on `/start`. Sessions are not persisted across bot restarts.
7. **Keep `Cargo.lock` out of git for the plugin.** It's a library crate — Cargo.lock in libraries is not committed per Rust convention. It IS in `.gitignore`.

## 6. Commands

### Rust plugin
```bash
# Type-check (fastest feedback loop — use this constantly)
source "$HOME/.cargo/env" && cd plugin && cargo check

# Full compile check with lints
source "$HOME/.cargo/env" && cd plugin && cargo clippy -- -D warnings

# Format
source "$HOME/.cargo/env" && cd plugin && cargo fmt

# Build release plugin (.so)
source "$HOME/.cargo/env" && cd plugin && cargo build --release
```

### TypeScript bot
```bash
# Type-check
cd bot && npm run typecheck

# Lint
cd bot && npm run lint

# Dev (hot reload, no compile step)
cd bot && npm run dev

# Build for production
cd bot && npm run build
```

### Run both in dev
```bash
# Terminal 1 — plugin build
source "$HOME/.cargo/env" && cd plugin && cargo build

# Terminal 2 — bot dev server
cd bot && npm run dev
```

## 7. Environment Variables

| Variable | Required | Notes |
|----------|----------|-------|
| `TELEGRAM_BOT_TOKEN` | yes | From @BotFather |
| `AOMI_API_KEY` | yes | From aomi.dev dashboard |
| `AOMI_BASE_URL` | no | Default: `https://api.aomi.dev` |
| `AOMI_APP` | no | Default: `fanforge`. Must match registered plugin name. |
| `SUPABASE_URL` | yes | Free project at supabase.com |
| `SUPABASE_ANON_KEY` | yes | Supabase project anon/public key |
| `PINATA_JWT` | yes* | Pinata JWT for IPFS metadata upload. *Required for coin launch. Free at pinata.cloud. |
| `ZORA_API_KEY` | no | All Zora reads work unauthenticated at lower rate limits |
| `BASE_RPC_URL` | no | Default: `https://mainnet.base.org`. Used for direct on-chain reads. |

Single source of truth: repo-root `.env`. Never commit `.env`. Run all commands from repo root so `dotenv/config` resolves correctly.

## 8. Supabase Schema

Apply manually via Supabase dashboard SQL editor.

**Phase 1 (required before coin launch testing):**
```sql
CREATE TABLE creator_coins (
  id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  creator_telegram_id TEXT NOT NULL,
  coin_address        TEXT NOT NULL UNIQUE,
  ticker              TEXT NOT NULL,
  name                TEXT NOT NULL,
  transaction_hash    TEXT NOT NULL,
  zora_url            TEXT NOT NULL,
  created_at          TIMESTAMPTZ DEFAULT NOW()
);
```

**Phase 3 (required before mission testing):**
```sql
CREATE TABLE fan_missions (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  coin_address TEXT NOT NULL,
  title       TEXT NOT NULL,
  content_url TEXT NOT NULL,
  threshold   NUMERIC NOT NULL,
  expires_at  TEXT,
  status      TEXT NOT NULL DEFAULT 'active',
  created_at  TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE reward_distributions (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  mission_id       UUID REFERENCES fan_missions(id) NOT NULL,
  recipient_wallet TEXT NOT NULL,
  delivered_at     TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE (mission_id, recipient_wallet)
);
```

## 9. Phase Status

| Phase | Name | Status | Gate |
|-------|------|--------|------|
| 0 | Foundation | ✅ complete | `cargo check` + `tsc --noEmit` both pass |
| 1 | Coin Launch | 🔄 in progress | Telegram msg → live Zora coin link |
| 2 | Fan Intelligence | 🔲 | `get_fan_leaderboard` live with real data |
| 3 | Fan Missions | 🔲 | Mission creates, rewards distribute |
| 4 | Recap & Automation | 🔲 | Daily cron push + on-demand recap |
| 5 | Polish & Demo Prep | 🔲 | 90-sec demo video + README |
| 6 | Buffer & Stretch | 🔲 | Web dashboard; fan-side Telegram |

## 10. Module Development Checklist

When adding or updating a tool:
- [ ] Args struct in `client.rs` — `#[derive(Debug, Deserialize, JsonSchema)]`, doc comments on every field (they are model-facing)
- [ ] Tool impl in `tool.rs` — `NAME` and `DESCRIPTION` tell the model WHEN to call the tool
- [ ] Validate args at the top of `run()` before any HTTP call
- [ ] Return `ok(json!({...}))` for success; `Err("error_code: message".to_string())` for failure
- [ ] Error codes in `error_code: message` format — the bot surfaces these as plain English
- [ ] Tool registered in `dyn_aomi_app! tools = [...]` in `lib.rs`
- [ ] Bot handler in `bot/src/handlers/` updated to handle any new conversation flow
- [ ] `cargo check` passes clean
- [ ] `npm run typecheck` passes clean
- [ ] Jargon audit: every user-facing string checked against §5 forbidden terms list

## 11. Do Not Patterns

- **NEVER store private keys, seed phrases, or wallet secrets anywhere in the codebase.**
- **NEVER call Zora or Supabase from the Telegram bot layer.** All external calls go through tools.
- **NEVER display full wallet addresses in Telegram messages.** Use `wallet_short` from the leaderboard (first 6 + `…` + last 4 chars).
- **NEVER block the Telegram response.** `session.send()` is already async + polling. If a tool is slow, it resolves when Aomi is done.
- **NEVER use crypto jargon** in any user-facing string in `tool.rs`, `lib.rs` PREAMBLE, or `bot/src/`. See §5 forbidden terms.
- **NEVER allow duplicate reward delivery.** `UNIQUE(mission_id, recipient_wallet)` is the DB-level guarantee. Do not add a pre-check; trust the constraint.
- **NEVER add a `main.rs` to the plugin crate.** It's `cdylib`. `lib.rs` IS the entry point.
- **NEVER import `reqwest::blocking` from `tokio`-spawned tasks** — blocking reqwest calls are fine in `run()` because the Aomi runtime calls tools synchronously.
- **NEVER hardcode API keys, RPC URLs, or contract addresses** in source. Env vars only.
- **NEVER commit `.env`.**
- **NEVER change `crate-type`** from `["cdylib"]` — the Aomi runtime requires a shared library.

## 12. Git Workflow

### Branch
- `main` is the working branch. Single committer for now.

### Commit messages
Conventional Commits: `feat(<scope>): …`, `fix(<scope>): …`, `chore(<scope>): …`, `docs(<scope>): …`.

Scopes: `plugin`, `bot`, `docs`, `infra`.

### Pre-commit checklist
- [ ] `cargo check` passes (run from `plugin/`)
- [ ] `npm run typecheck` passes (run from `bot/`)
- [ ] No `println!` debug leftover in tool handlers
- [ ] No `console.log` leftover in bot handlers (use proper error reporting)
- [ ] No new env var without updating `§7` of this file AND `.env.example`
- [ ] Phase status table in `§9` updated if a phase completed

### After each session
1. Update `handoff.md` with what changed and what's next.
2. Commit with `docs(handoff): update session state`.
3. Push to `origin/main`.

## 13. Naming Conventions

- **Tool NAME constants** use `fanforge_` prefix, snake_case: `fanforge_launch_fan_coin`.
- **Tool struct names** are PascalCase matching the tool name: `LaunchFanCoin`.
- **Args structs** are PascalCase with `Args` suffix: `LaunchFanCoinArgs`.
- **Bot handlers** are kebab-case files: `handlers/launch.ts`, `handlers/mission.ts`.
- **Env vars** are `SCREAMING_SNAKE_CASE` with product prefix where relevant: `AOMI_APP`, `SUPABASE_URL`.

## 14. Submission Checklist (May 30)

- [ ] Product Twitter page with logo
- [ ] Demo video on Twitter (90 sec, tag @DecentralDev_, @aomi_labs, @base, @zora)
- [ ] GitHub repo public with README
- [ ] Live demo link (bot or web)
- [ ] `docs/user-guide.md` — how a creator uses FanForge
- [ ] Polished UI (no raw JSON visible to users)
- [ ] Submission form: https://docs.google.com/forms/d/e/1FAIpQLSckjV_L8qIjyI9scGU0AiBfzSElkd_RXsTDlvggmkoUpcnNSw/viewform

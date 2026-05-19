# GOAL.md — FanForge
> Created: 2026-05-18 | Hackathon: OpenPandora Early Forge (May 20–30)

## 1. Project Identity

- **Purpose:** An Aomi-native on-chain fan economy platform for non-crypto-native music creators. Wraps Zora's creator coin primitives as agentic tools so a creator can launch a fan coin, automate holder rewards, and get daily recaps — all in plain English via Telegram.
- **Persona:** Temi Falola — 26-year-old Nigerian musician, 80k TikTok followers, 40k Audiomack streams/month. Not crypto-native. No time to learn wallets. Wants real income from real fans.
- **Protocol:** Zora (on Base) — creator coins, fan economies, gated content.
- **Runtime:** Aomi on-chain agent runtime — aomi-sdk Rust crate + Aomi cloud orchestration.
- **Stage:** Hackathon build. Demo-quality by May 30. Full end-to-end must work: Telegram message → live Zora coin → automated mission delivery → daily recap.
- **Core promise:** 6-step coin launch → 1 Telegram message. Zero crypto jargon visible to the creator.

## 2. Tech Stack

- **Backend:** Rust 1.95 (edition 2024), `aomi-sdk 0.1.19`, `reqwest 0.13` (blocking), tokio, schemars, serde/serde_json.
  - Crate type: `cdylib` — compiles to a `.so` hot-loaded by the Aomi runtime. NOT a binary or HTTP server.
- **Frontend:** TypeScript, grammY 1.43 (Telegram Bot API), Node.js 22, `@aomi-labs/client 0.1.37`.
- **State store:** Supabase (free tier) — accessed via REST API (`SUPABASE_URL` + `SUPABASE_ANON_KEY`). A local Postgres connection pool is not viable inside a cdylib (no persistent process, no long-lived socket).
- **Blockchain:** Base mainnet (chain ID 8453). Zora Protocol API (`https://api-sdk.zora.engineering`).
- **Agent runtime:** Aomi cloud. Plugin connects via `app: "fanforge"` in `SessionOptions`. Bot uses `@aomi-labs/client` `Session` class.
- **Package manager:** `cargo` (Rust) + `npm` (TypeScript bot). Never mix.
- **Lint/format:** `clippy` + `rustfmt` (Rust); `biome` (TypeScript).

> **Phase 0 corrections:** Original design specified PostgreSQL + Redis. Revised to Supabase REST API after discovering cdylib plugins don't own a persistent process. `SessionOptions.namespace` does not exist — the correct field is `app`. `AomiMessage.role` does not exist — the correct field is `sender` (`"user" | "agent" | "system"`).

## 3. Project Structure

```
/
├── GOAL.md                      ← This file. Project identity + architecture rules.
├── .env.example                 ← All required env vars documented. Copy to .env.
├── .env                         ← SINGLE source of truth. Never commit.
├── Cargo.toml                   ← Rust workspace root
├── fanforge-tools/              ← Rust crate: aomi-sdk tool server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              ← Boot: validates env, connects DB + Redis, registers tools, starts server
│       ├── tools/               ← One file per agentic tool
│       │   ├── launch_fan_coin.rs
│       │   ├── get_fan_leaderboard.rs
│       │   ├── create_fan_mission.rs
│       │   ├── distribute_rewards.rs
│       │   └── get_weekly_recap.rs
│       ├── services/            ← Business logic. Tools call services. Never call services from bot.
│       │   ├── zora.rs          ← Zora API + coin creation
│       │   ├── chain.rs         ← Base RPC: ERC-20 balances, Transfer events
│       │   ├── mission.rs       ← Mission creation, eligibility check, delivery
│       │   └── recap.rs         ← Recap generation + plain-English formatting
│       ├── db/
│       │   ├── mod.rs           ← sqlx pool init
│       │   └── queries/         ← One file per model: creators.rs, coins.rs, missions.rs, etc.
│       ├── cache/
│       │   └── mod.rs           ← Redis helpers: get, set, del, lock, unlock
│       └── config.rs            ← Env var loading + boot validation
├── bot/                         ← TypeScript Telegram bot
│   ├── package.json
│   ├── tsconfig.json
│   ├── biome.json
│   └── src/
│       ├── index.ts             ← Bot init + middleware
│       ├── handlers/            ← One file per conversation flow
│       │   ├── start.ts         ← /start → creator onboarding
│       │   ├── launch.ts        ← "launch my fan coin" conversation
│       │   ├── leaderboard.ts   ← "show my top fans"
│       │   ├── mission.ts       ← "create a mission" conversation
│       │   └── recap.ts         ← "give me my recap" + daily cron push
│       ├── aomi.ts              ← Aomi runtime client (one place)
│       └── scheduler.ts         ← Daily recap + snapshot cron jobs
├── migrations/                  ← PostgreSQL migration files (sequential, named)
│   ├── 001_create_creators.sql
│   ├── 002_create_creator_coins.sql
│   ├── 003_create_fan_missions.sql
│   ├── 004_create_reward_distributions.sql
│   └── 005_create_fan_snapshots.sql
└── docs/
    ├── user-guide.md            ← How a creator uses FanForge
    └── tool-spec.md             ← Full tool input/output contracts
```

## 4. The 5 Agentic Tools

Every tool lives in `fanforge-tools/src/tools/`. Tool handlers: parse inputs → validate → call a service → return output. Business logic lives only in `src/services/`.

### `launch_fan_coin`

```
Input:
  name:        String           — display name of the coin
  ticker:      String           — 3–5 alphanumeric chars
  description: String           — creator's plain-English pitch
  image_url:   Option<String>   — optional cover art

Steps:
  1. Validate ticker format (3–5 chars, alphanumeric, uppercase-normalized)
  2. Acquire Redis lock: launch_lock:{creator_id} (60s TTL) — reject if already held
  3. Check ticker uniqueness on-chain via Zora API
  4. Call zora::create_coin() — mints ERC-20, funds initial pool
  5. Upsert creator_coins row (idempotent on coin_address)
  6. Release lock

Output:
  { coin_address: String, zora_url: String, ticker: String, name: String }

Error codes:
  ticker_taken | invalid_ticker | launch_in_progress | zora_api_error
```

### `get_fan_leaderboard`

```
Input:
  coin_address: String
  limit:        Option<u32>   — default 10, max 50

Steps:
  1. Check Redis: leaderboard:{coin_address} (TTL 5 min)
  2. Cache miss → chain::get_holder_balances(coin_address) via Base RPC
  3. Sort by balance descending, compute pct_held
  4. Write cache
  5. Return ranked list

Output:
  { entries: [{ rank, wallet_short, balance, pct_held }], total_holders, last_synced }

Error codes:
  coin_not_found | rpc_error
```

### `create_fan_mission`

```
Input:
  coin_address: String
  title:        String
  content_url:  String        — the gated content link to deliver to eligible fans
  threshold:    Decimal       — minimum coin balance to qualify
  expires_at:   Option<DateTime>

Steps:
  1. Verify coin exists in creator_coins
  2. Validate threshold > 0
  3. Insert fan_missions row

Output:
  { mission_id: String, title: String, threshold: Decimal, status: "active" }

Error codes:
  coin_not_found | invalid_threshold
```

### `distribute_rewards`

```
Input:
  mission_id: String

Steps:
  1. Load mission + coin, assert status = "active" and not expired
  2. Get current leaderboard (cache-aware, same logic as get_fan_leaderboard)
  3. Filter holders meeting threshold
  4. tokio::spawn (non-blocking):
       for each eligible wallet NOT in reward_distributions(mission_id, wallet):
         insert distribution record → deliver content_url via callback
  5. Return immediately

Output:
  { eligible_count: u32, newly_dispatched: u32, status: "dispatching" }

Invariant: UNIQUE(mission_id, recipient_wallet) in DB prevents double delivery.

Error codes:
  mission_not_found | mission_expired | coin_not_found
```

### `get_weekly_recap`

```
Input:
  coin_address: String
  days:         Option<u32>   — default 7

Steps:
  1. Query fan_snapshots for holder trend over window
  2. Query reward_distributions for mission activity count
  3. Fetch current holder count (leaderboard cache first)
  4. Format plain-English summary — no crypto jargon
  5. Generate Twitter-ready post (≤280 chars)

Output:
  { summary: String, twitter_post: String,
    metrics: { new_holders, total_holders, missions_fired, top_wallet_short } }

Error codes:
  coin_not_found | insufficient_history
```

## 5. State Schema (Supabase)

> Note: Original design used PostgreSQL with sqlx. Revised to Supabase REST API — see tech stack note above. Tables are created via the Supabase SQL editor.

```sql
-- Telegram creators onboarded to FanForge
CREATE TABLE creators (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  telegram_id     BIGINT UNIQUE NOT NULL,
  telegram_handle TEXT,
  display_name    TEXT NOT NULL,
  aomi_wallet     TEXT,
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Zora coins launched through FanForge
CREATE TABLE creator_coins (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  creator_id      UUID REFERENCES creators(id) NOT NULL,
  coin_address    TEXT UNIQUE NOT NULL,
  coin_name       TEXT NOT NULL,
  ticker          TEXT NOT NULL,
  zora_url        TEXT NOT NULL,
  launched_at     TIMESTAMPTZ NOT NULL,
  last_synced_at  TIMESTAMPTZ
);

-- Gated content missions (unlocked by holding minimum balance)
CREATE TABLE fan_missions (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  coin_id         UUID REFERENCES creator_coins(id) NOT NULL,
  title           TEXT NOT NULL,
  content_url     TEXT NOT NULL,
  threshold       NUMERIC NOT NULL,
  expires_at      TIMESTAMPTZ,
  status          TEXT NOT NULL DEFAULT 'active',
  created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Delivery ledger — idempotency enforced at DB level
CREATE TABLE reward_distributions (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  mission_id       UUID REFERENCES fan_missions(id) NOT NULL,
  recipient_wallet TEXT NOT NULL,
  delivered_at     TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE (mission_id, recipient_wallet)
);

-- Daily holder snapshots for trend data in recaps
CREATE TABLE fan_snapshots (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  coin_id          UUID REFERENCES creator_coins(id) NOT NULL,
  wallet_address   TEXT NOT NULL,
  balance          NUMERIC NOT NULL,
  rank             INT NOT NULL,
  snapshotted_at   TIMESTAMPTZ DEFAULT NOW()
);
```

## 6. Architecture Rules

1. **Tools are the only execution surface.** The Telegram bot never calls Zora or Base RPC directly. Every on-chain action routes through a tool in `fanforge-tools/`.
2. **Services own business logic.** Tool handlers parse inputs, call a service, return output. All Zora API calls and RPC interactions live only in `src/services/`.
3. **Cache is required at boot.** Redis is a hard dependency. `main.rs` exits immediately with a clear error if `REDIS_URL` is missing or unreachable. Leaderboard reads always go cache-first.
4. **Non-blocking reward distribution.** `distribute_rewards` spawns delivery as a background task and returns immediately. The tool call never waits for on-chain confirmation or Telegram message delivery.
5. **No crypto jargon in user strings.** Every user-facing string is reviewed against the jargon list in Section 11. "Your fan economy is live" not "contract deployed." "Top supporters" not "top token holders."
6. **Wallet delegation, not custody.** FanForge never stores or sees private keys. Aomi runtime handles all signing. Never accept, log, or echo a seed phrase.
7. **Idempotent writes everywhere.** Coin creation, mission creation, and reward delivery are each safe to retry without creating duplicates.
8. **Concurrency guard on launch.** Redis lock `launch_lock:{creator_id}` (60s TTL) prevents two simultaneous coin launches for the same creator.
9. **DB unique constraint is the delivery deduplication guarantee.** `UNIQUE(mission_id, recipient_wallet)` in `reward_distributions` — the service layer does not need a pre-check.
10. **Env vars validated at boot.** Missing any of `DATABASE_URL`, `REDIS_URL`, `AOMI_API_KEY`, `TELEGRAM_BOT_TOKEN`, `BASE_RPC_URL`, `ZORA_API_KEY` causes `main.rs` to exit 1 with a plain-language message before connecting to anything.

## 7. Environment Variables

| Variable          | Required | Purpose                                        |
|-------------------|----------|------------------------------------------------|
| `DATABASE_URL`    | yes      | PostgreSQL connection string                   |
| `REDIS_URL`       | yes      | Cache — required at boot, consistency layer    |
| `AOMI_API_KEY`    | yes      | Aomi runtime authentication                    |
| `TELEGRAM_BOT_TOKEN` | yes   | Telegram Bot API token                         |
| `BASE_RPC_URL`    | yes      | Base mainnet RPC endpoint for ERC-20 reads     |
| `ZORA_API_KEY`    | yes      | Zora Protocol API authentication               |
| `PORT`            | no       | Tool server port (default 8080)                |

Single source of truth = repo-root `.env`. Never commit `.env`. Run Rust tooling from repo root so `dotenv` resolves correctly.

## 8. Commands

```bash
# Run Rust tool server (development)
cargo run -p fanforge-tools

# Run TypeScript Telegram bot (development)
npm --prefix bot run dev

# Type-check Rust
cargo check -p fanforge-tools

# Lint Rust
cargo clippy -p fanforge-tools -- -D warnings

# Format Rust
cargo fmt -p fanforge-tools

# Type-check TypeScript
npm --prefix bot run typecheck

# Lint TypeScript
npm --prefix bot run lint

# Run database migrations
# (migration runner TBD — sqlx-cli or custom migration script)
```

## 9. Phased Implementation

### Phase 0 — Foundation ✅ complete (2026-05-19)
- [x] Rust workspace: `plugin/` crate, aomi-sdk 0.1.19 wired, all 5 tools registered and compiling
- [x] TypeScript bot scaffold: grammY, `/start` command returns a message, env config loaded
- [x] Supabase schema defined (tables created manually via SQL editor before Phase 3)
- [x] `.env.example` documents all 8 required variables
- [x] `cargo check` passes (2 expected warnings — unused stubs, resolved in Phase 1)
- [x] `tsc --noEmit` passes clean
- [x] CLAUDE.md, GOAL.md, handoff.md all current

### Phase 1 — Coin Launch (Days 1–2, May 20–21)
- [ ] `launch_fan_coin` tool — Zora integration end-to-end on Base
- [ ] Creator onboarding: `/start` → name → link Aomi wallet
- [ ] Telegram launch flow: prompts for coin name, ticker, description → calls tool → returns Zora link
- [ ] Redis launch lock implemented and tested
- [ ] Idempotent `creator_coins` upsert on `coin_address`
- [ ] Bot reply uses zero crypto jargon
- [ ] Gate: Telegram message → real coin live on Zora testnet

### Phase 2 — Fan Intelligence (Days 2–3, May 21–22)
- [ ] `get_fan_leaderboard` tool — Base RPC via alloy-rs
- [ ] Redis cache: `leaderboard:{coin_address}`, 5-min TTL
- [ ] Wallet display: first 6 + last 4 chars only (never full address in chat)
- [ ] Telegram: "who are my top fans?" → formatted top-10 reply with inline refresh button

### Phase 3 — Fan Missions (Days 3–4, May 22–23)
- [ ] `create_fan_mission` tool — validated, stored, returned
- [ ] `distribute_rewards` tool — tokio::spawn background delivery, idempotent
- [ ] Delivery mechanism: content URL sent via Telegram DM (where fan has linked their wallet) or stored for fan-side retrieval
- [ ] Duplicate delivery blocked at DB level — no pre-check needed in service
- [ ] Creator sees delivery count in bot reply

### Phase 4 — Recap & Automation (Days 4–5, May 23–24)
- [ ] `get_weekly_recap` tool — plain-English summary + Twitter post generated
- [ ] Daily cron in `bot/src/scheduler.ts`: 09:00 → push recap to all active creators
- [ ] Daily snapshot cron: captures holder rankings → `fan_snapshots` table
- [ ] Creator can request recap on-demand: "give me my recap"

### Phase 5 — Polish & Demo Prep (Days 5–7, May 24–26)
- [ ] Telegram UX: inline keyboards for all multi-step flows, loading states, error messages in plain English
- [ ] Jargon audit: every user-facing string reviewed against Section 11 forbidden terms
- [ ] 90-second demo video: Temi persona, full flow — launch → leaderboard → mission → recap
- [ ] README + `docs/user-guide.md`
- [ ] Product Twitter page with logo
- [ ] GitHub repo made public

### Phase 6 — Buffer & Stretch Goals (Days 7–10, May 26–30)
- [ ] Web dashboard (Next.js): coin analytics chart, mission management, holder count over time
- [ ] Fan-side Telegram flow: fan discovers a creator's coin, gets guided buy link
- [ ] Final submission form entry
- [ ] GitHub stars push for repo bounty

## 10. Module Development Checklist

When adding any new agentic tool:
- [ ] Tool handler in `src/tools/<name>.rs` — parse, validate, call service, return output
- [ ] Service module in `src/services/<name>.rs` owns all business logic and external calls
- [ ] Cache invalidation: any write that affects a cached resource calls `cache::del()` with the correct key pattern
- [ ] Idempotency: define and test the retry behavior before shipping
- [ ] Error codes: defined in the tool spec (Section 4) and mapped to user-facing plain-English messages in the bot handler
- [ ] No crypto jargon: run every user-facing string through the Section 11 checklist
- [ ] `cargo check` passes clean after adding the tool
- [ ] Tool registered with aomi-sdk in `main.rs`
- [ ] Corresponding bot handler in `bot/src/handlers/` updated
- [ ] Update Section 4 of this file if the tool spec changes

## 11. Do Not Patterns

- **NEVER store private keys, seed phrases, or wallet secrets.** If a secret ever appears in a log or string, treat it as a production incident.
- **NEVER call Zora or Base RPC from the Telegram bot layer.** All on-chain interaction routes through a tool in `fanforge-tools/`.
- **NEVER expose full wallet addresses in Telegram messages.** Display first 6 + last 4 chars only.
- **NEVER block the Telegram response waiting for on-chain confirmation.** Use tokio::spawn + async polling.
- **NEVER let reward distribution run synchronously inside a tool call.** It must be a background task.
- **NEVER let the service start without Redis.** Cache is the consistency layer. Exit with a clear error if `REDIS_URL` is unreachable at boot.
- **NEVER use crypto jargon in user-facing strings.** Forbidden terms: "contract," "deploy," "wallet address," "token," "ERC-20," "on-chain," "mint," "gas," "transaction hash." If a term appears in a bot reply, it must be replaced.
- **NEVER allow duplicate reward delivery.** The `UNIQUE(mission_id, recipient_wallet)` constraint is the guarantee — trust it, don't work around it.
- **NEVER ship without an `.env.example`** that documents every required variable with a comment.
- **NEVER commit `.env`** to the repository.
- **NEVER hardcode RPC URLs, API keys, or contract addresses** in source code — env vars only.
- **NEVER query the database from a tool handler.** Tool handlers call services. Services call `db/queries/`. No DB access outside `src/db/`.
- **NEVER catch and swallow errors silently** in background tasks — log them and emit a counter metric.

## 12. Submission Checklist (May 30 deadline)

- [ ] Product Twitter page with logo exists
- [ ] Demo video posted on Twitter (90 sec, tag @DecentralDev_, @aomi_labs, @base, @zora)
- [ ] GitHub repo is public with a README
- [ ] Live demo link or video walkthrough accessible
- [ ] `docs/user-guide.md` explains how a creator uses FanForge
- [ ] UI is polished — no raw JSON, no error stack traces visible to users
- [ ] Submission form filled: https://docs.google.com/forms/d/e/1FAIpQLSckjV_L8qIjyI9scGU0AiBfzSElkd_RXsTDlvggmkoUpcnNSw/viewform

## 13. Voice & Phrasing

When writing any copy, bot reply, or documentation for FanForge:
- The product is **"a fan economy platform for music creators — powered by AI."**
- The creator audience is **"artists and musicians with an established fanbase who want to deepen fan relationships and build new income streams."**
- Avoid all Web3-native framing in creator-facing surfaces. Crypto is the implementation detail, not the product.
- In developer-facing docs (README, tool-spec.md), crypto-accurate terminology is appropriate and expected.

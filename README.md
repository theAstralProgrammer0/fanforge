# FanForge

> On-chain fan economy for music creators — powered by [Aomi](https://aomi.dev) + [Zora](https://zora.co)

FanForge turns a Telegram message into a live creator coin on Base. A musician with 80k TikTok followers and zero crypto experience types one sentence. Their fans get a coin to hold, missions to complete, and exclusive content to unlock. The creator gets daily recaps and a real income stream — without ever seeing a wallet address.

Built for the **OpenPandora Early Forge hackathon** (deadline June 5, 2026).

---

## Demo

> Bot: [@FanForgeBot](https://t.me/FanForgeBot) on Telegram

Send `/start` to begin. The bot is currently running on the `default` Aomi app pending activation of the `fanforge` plugin on the Aomi backend. Once activated, the full coin launch → mission → recap flow goes live.

---

## What It Does

| You say | What happens |
|---------|-------------|
| "Launch a fan coin called TEMI" | Aomi wallet is linked, metadata pinned to IPFS, Zora coin deployed on Base, link returned |
| "Who are my top fans?" | Live leaderboard of coin holders ranked by balance |
| "Reward fans holding 100+ coins with this track link" | Mission created, content delivered to every qualifying holder |
| "Give me my weekly recap" | Plain-English summary + ready-to-post Twitter copy |

---

## Architecture

```
Telegram User
     │
     ▼
[grammY Bot — TypeScript]          ← bot/
     │  session.send(message)
     ▼
[Aomi Cloud Runtime]               ← routes through AI + tool dispatch
     │  calls tools
     ▼
[FanForge Plugin — Rust cdylib]    ← plugin/
     ├── fanforge_launch_fan_coin  → IPFS pin → Zora API → stage_tx → FinalizeLaunch
     ├── fanforge_get_fan_leaderboard
     ├── fanforge_create_fan_mission
     ├── fanforge_distribute_rewards
     └── fanforge_get_creator_recap
          │
          ├── Zora Protocol API (api-sdk.zora.engineering)
          ├── Pinata IPFS (coin metadata)
          └── Supabase (missions + reward ledger)
```

The Telegram bot **never** calls Zora or Supabase directly. Every on-chain action and state write routes through the Rust plugin via Aomi's tool dispatch.

---

## The 7 Tools

| Tool | Phase | Description |
|------|-------|-------------|
| `fanforge_launch_fan_coin` | 1 | One-message coin launch — pins metadata to IPFS, calls Zora API, stages + commits tx |
| `fanforge_finalize_launch` *(internal)* | 1 | Fires after tx commit — stores coin in Supabase, returns live Zora URL |
| `fanforge_get_fan_leaderboard` | 2 | Ranked holder list from Zora with balances |
| `fanforge_create_fan_mission` | 3 | Gates exclusive content behind a minimum coin holding |
| `fanforge_distribute_rewards` | 3 | Delivers content URL to every qualifying holder (idempotent) |
| `fanforge_get_creator_recap` | 4 | Weekly summary with holder count, market cap, mission activity, Twitter post |

### Coin launch route chain

```
fanforge_launch_fan_coin
  wallet ← ctx.attribute_string(["domain","evm","address"])
  → ipfs_pin_json()  →  Pinata
  → POST /create/content  →  Zora API
  → route: evm-core/stage_tx
  → enforce: simulate_batch → commit_txs  (binds "transaction_hash")
  → after: fanforge_finalize_launch
      → supabase_post("creator_coins")
      → returns { zora_url, coin_address, ticker, name }
```

---

## Stack

| Layer | Technology |
|-------|-----------|
| Plugin | Rust 1.95 (edition 2024), `aomi-sdk 0.1.20`, `reqwest 0.13` (blocking) |
| Bot | TypeScript, Node.js 22, `grammy 1.43`, `@aomi-labs/client 0.1.37` |
| Chain | Base mainnet (chain ID 8453) |
| Protocol | [Zora](https://zora.co) — creator coins via `api-sdk.zora.engineering` |
| State | [Supabase](https://supabase.com) (missions + reward ledger) |
| IPFS | [Pinata](https://pinata.cloud) (coin metadata) |
| Runtime | [Aomi](https://aomi.dev) cloud — plugin hot-loaded as `.so` |

---

## Setup

### Prerequisites

- Rust 1.79+ (`rustup`)
- Node.js 22+ (`nvm`)
- A [Supabase](https://supabase.com) free project
- A [Pinata](https://pinata.cloud) free account (JWT key)
- A [Telegram bot token](https://t.me/BotFather)
- An [Aomi API key](https://aomi.dev)

### 1. Clone & install

```bash
git clone https://github.com/theAstralProgrammer0/fanforge.git
cd fanforge
cd bot && npm install && cd ..
```

### 2. Environment

```bash
cp .env.example .env
# Fill in all required variables — see .env.example for docs
```

Required variables:

| Variable | Source |
|----------|--------|
| `TELEGRAM_BOT_TOKEN` | @BotFather on Telegram |
| `AOMI_API_KEY` | [aomi.dev](https://aomi.dev) dashboard |
| `SUPABASE_URL` | Supabase project settings |
| `SUPABASE_ANON_KEY` | Supabase project settings |
| `PINATA_JWT` | Pinata → API Keys → New Key (JWT) |

### 3. Supabase schema

Run this in the Supabase SQL editor:

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

CREATE TABLE fan_missions (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  coin_address  TEXT NOT NULL,
  title        TEXT NOT NULL,
  content_url  TEXT NOT NULL,
  threshold    NUMERIC NOT NULL,
  expires_at   TEXT,
  status       TEXT NOT NULL DEFAULT 'active',
  created_at   TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE reward_distributions (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  mission_id       UUID REFERENCES fan_missions(id) NOT NULL,
  recipient_wallet TEXT NOT NULL,
  delivered_at     TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE (mission_id, recipient_wallet)
);
```

### 4. Build and run

```bash
# Build the Rust plugin
source "$HOME/.cargo/env" && cd plugin && cargo build --release && cd ..

# Start the bot (hot reload)
cd bot && npm run dev
```

---

## Development

```bash
# Type-check plugin (fastest feedback loop)
source "$HOME/.cargo/env" && cd plugin && cargo check

# Lint plugin
source "$HOME/.cargo/env" && cd plugin && cargo clippy -- -D warnings

# Type-check bot
cd bot && npm run typecheck

# Lint bot
cd bot && npm run lint
```

---

## Plugin Deployment (Aomi)

FanForge deploys via `aomi-git` to `aomi-labs/community-apps`. Once the commit lands on that repo's publish branch, CI builds the `.so` and the Aomi team activates it.

```bash
# Requires write access to aomi-labs/community-apps
cd plugin
aomi-git deploy
```

Until activation, the bot runs on `AOMI_APP=default` (Aomi's built-in EVM assistant). To test the plugin locally without activation:

```bash
# aomi-run version must match aomi-sdk version in Cargo.toml (currently 0.1.20)
aomi-run --plugin target/release/libfanforge.so \
  --prompt "launch a fan coin called Temi Coin with ticker TEMI"
```

---

## Design Principles

- **Zero crypto jargon for creators.** No "deploy", "contract", "wallet address", "ERC-20", "mint", "gas". The creator sees "fan coin", "supporters", "unlock", "exclusive access".
- **Tools are the only execution surface.** The bot never calls Zora or Supabase directly.
- **Idempotent writes.** Every Supabase write is safe to retry. `UNIQUE(mission_id, recipient_wallet)` prevents double reward delivery at the DB level.
- **Wallet delegation, not custody.** FanForge never sees or stores private keys. Aomi's `evm-core` namespace handles all signing.

---

## Submission

Built for [OpenPandora Early Forge](https://docs.google.com/forms/d/e/1FAIpQLSckjV_L8qIjyI9scGU0AiBfzSElkd_RXsTDlvggmkoUpcnNSw/viewform) — an Aomi hackathon on top of Base + Zora.

Tags: `@aomi_labs` `@base` `@zora` `@DecentralDev_`

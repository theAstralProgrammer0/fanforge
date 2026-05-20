# handoff.md — FanForge
> Written: 2026-05-20 | Branch: main | Phase: 1 in progress

---

## Goal

Build **FanForge** — a Telegram bot backed by an Aomi-native Rust plugin that wraps Zora's creator coin economy. A non-crypto-native music creator types one sentence and gets a live fan coin on Base. Everything downstream — leaderboards, missions, reward delivery, social recaps — is automated.

Stack: Rust cdylib (aomi-sdk 0.1.19), TypeScript/grammY (Node.js 22), Supabase (state), Zora Protocol API, `@aomi-labs/client 0.1.37`. Deadline: May 30.

---

## Phase 0 (2026-05-19) — Complete ✅

Foundation: Rust cdylib plugin + TypeScript Telegram bot scaffold. `cargo check` + `tsc --noEmit` both pass. All Phase 0 files committed and pushed.

---

## Phase 1 (2026-05-20) — In Progress 🔄

### What was done this session

**Coin launch architecture:**
- Researched Zora coin creation: requires on-chain transaction signing (no server-side-only path).
- Discovered Zora's REST endpoint: `POST https://api-sdk.zora.engineering/create/content` returns `{calls:[{to,data,value}], predictedCoinAddress}` — pre-built calldata, no Rust ABI encoding needed.
- Read the full aomi-sdk route system (route.rs, builder.rs, types.rs) to understand `ToolReturn::route(...)`, `stage_tx`, `simulate_batch`, `commit_txs` patterns.

**Code written:**
- `LaunchFanCoin` — converted from stub to `run_with_routes`. Routes to `get_account_info` (binds "creator_wallet"), then to `fanforge_build_coin_tx`.
- `BuildCoinTx` — new internal tool. Extracts wallet address from injected `creator_wallet`, pins metadata JSON to IPFS via Pinata, calls Zora `/create/content`, then routes `stage_tx → simulate_batch → commit_txs` (binds "transaction_hash") → `fanforge_finalize_launch`.
- `FinalizeLaunch` — new internal tool. Stores coin in Supabase `creator_coins`, returns `{ zora_url, coin_address, ... }`.
- `ipfs_pin_json()` — new Pinata API helper in `client.rs`.
- `BuildCoinTxArgs`, `FinalizeLaunchArgs` — new Args structs in `client.rs`.
- `SECRET_PINATA_JWT` — new secret in `lib.rs`.
- `PINATA_JWT` — added to `.env.example`.

**Build status:**
- `cargo check`: ✅ passes clean (0 errors, 0 warnings).

### The complete coin launch route chain

```
LaunchFanCoin
  → route: get_account_info (binds "creator_wallet")
  → after: fanforge_build_coin_tx
      → ipfs_pin_json() → Pinata
      → POST /create/content → Zora API
      → route: stage_tx + enforce(simulate_batch → commit_txs binds "transaction_hash")
      → after: fanforge_finalize_launch
          → supabase_post("creator_coins", ...)
          → returns live Zora URL
```

### Key discoveries this session

1. **Zora coin creation is on-chain only.** No server-side-only path. The REST API at `/create/content` generates pre-built calldata but signing and submitting still require the user's wallet via Aomi `evm-core`.
2. **`data` for `stage_tx` is `{"raw": "0x..."}` format**, not a bare string. Confirmed from aomi-sdk builder.rs test at line 716.
3. **`get_account_info` returns the wallet address**. Exact response shape is unknown — `BuildCoinTx` extracts address by trying `.as_str()`, then `.get("address")`, then `.get("account").get("address")`.
4. **`commit_txs` binds the transaction hash**. Exact shape unknown — `FinalizeLaunch` extracts by trying `.as_str()`, then `.get("hash")`, then `.get("transactionHash")`.
5. **`run_with_routes` is the correct method to override** for tools that route. The dispatch system calls `run_with_routes` (which falls back to `run` by default).
6. **Pinata free tier** (1GB, no rate limit for small JSON) is the simplest IPFS approach for the hackathon. Endpoint: `POST https://api.pinata.cloud/pinning/pinJSONToIPFS`.

---

## Current State

### What works
- `cargo check` passes clean.
- `LaunchFanCoin`, `BuildCoinTx`, `FinalizeLaunch` all compile with correct route wiring.
- Bot continues to start with `npm run dev` and respond to `/start`.

### What does NOT work yet (blockers before testing)
1. **No `.env` file** — need `TELEGRAM_BOT_TOKEN`, `AOMI_API_KEY`, `PINATA_JWT`, `SUPABASE_URL`, `SUPABASE_ANON_KEY` set.
2. **`creator_coins` Supabase table not created** — run the Phase 1 SQL in CLAUDE.md §8.
3. **Plugin not registered with Aomi** — need to register `fanforge` app on aomi.dev.
4. **`get_account_info` return shape unverified** — if the wallet extraction fails, add logging and adjust the field paths in `BuildCoinTx`.

---

## Known Remaining Tech Debt

| # | Issue | When |
|---|-------|------|
| 1 | `get_account_info` return shape assumed (3 fallback paths) | Verify in Phase 1 live testing |
| 2 | `commit_txs` return shape assumed (3 fallback paths) | Verify in Phase 1 live testing |
| 3 | `creator_coins` Supabase table not created | Before Phase 1 testing |
| 4 | Plugin not registered with Aomi | Before end-to-end testing |
| 5 | No bot conversation state machine for launch flow | Phase 1 bot handler |
| 6 | Supabase `fan_missions` + `reward_distributions` tables not created | Before Phase 3 testing |

---

## Next Step — Complete Phase 1

**To reach gate:** "Telegram message → real Zora coin live on Base → bot replies with the link."

1. **Environment setup:**
   - Copy `.env.example` → `.env` and fill in all tokens.
   - Get `PINATA_JWT` from [pinata.cloud](https://pinata.cloud) (free tier, create JWT API key).
   - Create `creator_coins` table in Supabase (SQL in CLAUDE.md §8).
   - Register plugin with Aomi.

2. **Bot launch handler** (`bot/src/handlers/launch.ts`):
   - Add `/launch` command (or let the message relay handle it naturally via Aomi's LLM).
   - The existing message relay in `index.ts` already passes everything to Aomi — the LLM will guide the coin launch flow using the PREAMBLE's workflow instructions.
   - For Phase 1, the relay-based flow is sufficient (no custom multi-turn state machine needed yet).

3. **Live test:**
   - Run `npm run dev` in bot/
   - Send "I want to launch a fan coin called Temi Coin with ticker TEMI"
   - Verify the Zora coin appears at the returned URL.

4. **If `get_account_info` shape is wrong:** add a `println!` in `BuildCoinTx.run_with_routes` to inspect `args.creator_wallet`, adjust extraction logic, `cargo check`.

---

## Files Changed This Session

| File | Change |
|------|--------|
| `plugin/src/client.rs` | Added `ipfs_pin_json()`, `PINATA_API` const, `BuildCoinTxArgs`, `FinalizeLaunchArgs` |
| `plugin/src/tool.rs` | Replaced `LaunchFanCoin` stub with route impl; added `BuildCoinTx` + `FinalizeLaunch` |
| `plugin/src/lib.rs` | Registered `BuildCoinTx`, `FinalizeLaunch`; added `SECRET_PINATA_JWT` |
| `.env.example` | Added `PINATA_JWT` |
| `CLAUDE.md` | Updated §1 stage, §4 tools table + coin launch route docs, §7 env vars, §8 Supabase schema, §9 phase status |
| `handoff.md` | This file |

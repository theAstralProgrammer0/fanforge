# handoff.md — FanForge
> Updated: 2026-05-29 | Branch: main | Phase: 1 in progress (blocked on PR merge)

---

## Goal

Build **FanForge** — a Telegram bot backed by an Aomi Rust plugin that wraps Zora's creator coin economy. A non-crypto-native music creator types one sentence and gets a live fan coin on Base.

Stack: Rust cdylib (aomi-sdk 0.1.19), TypeScript/grammY (Node.js 22), Supabase (state), Zora Protocol API, `@aomi-labs/client 0.1.37`. Deadline: **June 5**.

---

## Phase 0 — Complete ✅

Foundation: Rust cdylib plugin + TypeScript Telegram bot scaffold. `cargo check` + `tsc --noEmit` both pass.

---

## Phase 1 — Code complete, blocked on PR merge 🔄

### What's done

**Plugin (all 7 tools implemented and compiling):**
- `LaunchFanCoin` → routes to `get_account_info` → `BuildCoinTx` (Pinata IPFS + Zora `/create/content` + evm-core) → `FinalizeLaunch` (Supabase + Zora URL)
- `GetFanLeaderboard` — Zora `/coinHolders`, correct `zora20Token.tokenBalances.edges[].node` path, balance ÷ 10^18
- `CreateFanMission` — Supabase write, threshold validation
- `DistributeRewards` — Supabase read + write, idempotent, correct Zora holder parsing
- `GetCreatorRecap` — Zora `/coin` (via `zora20Token` wrapper) + Supabase missions

**Bot (TypeScript):**
- Session-per-user relay via `@aomi-labs/client`
- `/start`, `/help` commands
- Smart response formatter: extracts `.message` field from tool results, graceful Markdown fallback, proper chunk splits at newlines
- dotenv ESM fix: `import.meta.url`-based path resolution

**Submission materials:**
- `README.md` — full architecture, setup, tool table, demo link
- `docs/user-guide.md` — creator-facing walkthrough (no crypto jargon)

**Tests:**
- 8 unit tests via `aomi_sdk::testing` — all pass
- Tests: ticker validation (5), threshold validation (2), Zora response shape (1)

**Zora API bugs fixed:**
- Responses wrapped in `zora20Token` — all 3 read tools updated
- coinHolders structure: `edges[].node.ownerAddress` (not `holders[].user.publicKey`)
- Balances are raw 18-decimal integers — divided by 1e18 for display and comparison

### Blocker

`fanforge` app not yet in Aomi app list. PR #49 open and clean:
- `https://github.com/aomi-labs/aomi-sdk/pull/49`
- All CI passed at time of submission. Pending admin merge.
- Bot currently runs on `AOMI_APP=default` — pipeline confirmed working.

### When PR merges

1. Switch `.env`: `AOMI_APP=fanforge`
2. Inject secrets: `aomi secret add SUPABASE_URL=... SUPABASE_ANON_KEY=... PINATA_JWT=...`
3. Restart bot
4. Test: "I want to launch a fan coin called Temi Coin with ticker TEMI"
5. Verify Zora URL in response

---

## Current State

### What works

- `cargo check` — clean ✅
- `cargo test` — 8/8 pass ✅
- `npm run typecheck` — clean ✅
- Bot starts, `/start` works, message relay works with `default` app ✅
- Zora API calls (read endpoints) confirmed working via curl ✅
- Supabase `creator_coins` table created ✅

### What does NOT work yet (blocked on PR)

- `fanforge_launch_fan_coin` end-to-end (requires `fanforge` app to be live)
- All FanForge-specific tools (same blocker)
- Demo video not recorded yet

---

## Coin launch route chain

```
LaunchFanCoin
  → route: get_account_info (binds "creator_wallet")
  → after: fanforge_build_coin_tx
      → ipfs_pin_json() → Pinata
      → POST /create/content → Zora API
      → route: stage_tx + enforce(simulate_batch → commit_txs binds "transaction_hash")
      → after: fanforge_finalize_launch
          → supabase_post("creator_coins")
          → returns { zora_url, coin_address, ... }
```

---

## Known Tech Debt

| # | Issue | When |
|---|-------|------|
| 1 | `get_account_info` return shape unverified (3 fallback paths in BuildCoinTx) | Verify in live test |
| 2 | `commit_txs` return shape unverified (3 fallback paths in FinalizeLaunch) | Verify in live test |
| 3 | `fan_missions` + `reward_distributions` Supabase tables not tested end-to-end | Phase 3 |

---

## Next Steps

1. **Record demo video** — show bot + code + explain PR blocker
2. **Submit form** — https://docs.google.com/forms/d/e/1FAIpQLSckjV_L8qIjyI9scGU0AiBfzSElkd_RXsTDlvggmkoUpcnNSw/viewform
3. **Social** — product Twitter page, tweet with `@aomi_labs @base @zora @DecentralDev_`
4. **When PR merges** — full end-to-end test, update demo video if time allows

## Files Changed This Session

| File | Change |
|------|--------|
| `plugin/src/tool.rs` | Fixed Zora API response parsing (3 tools); added 8 unit tests |
| `bot/src/index.ts` | Smart response formatter, graceful Markdown fallback |
| `README.md` | Created — full project overview |
| `docs/user-guide.md` | Created — creator-facing walkthrough |
| `handoff.md` | This file |

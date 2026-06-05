# FanForge — Complete Test & Demo Guide

---

## Before You Begin: What You Are Actually Testing

FanForge is not one system. It is five independent external services wired together through a Rust plugin and a TypeScript bot. When something breaks in a demo, the failure could be in any one of these layers:

```
You (Telegram) → grammY bot → Aomi session API → fanforge Rust plugin
                                                          ↓
                                               Pinata (IPFS upload)
                                               Zora API (coin deploy)
                                               Supabase (state writes)
```

The purpose of this guide is to let you verify each layer independently before you trust the whole chain. That way, when something goes wrong during the demo, you can say "it's not Supabase — I confirmed that 10 minutes ago — so the problem must be in the Aomi routing." Without this isolation discipline, you will waste precious demo time chasing ghosts.

You need three things open simultaneously:
- **Terminal A** — for running the bot (you watch logs here)
- **Terminal B** — for running verification commands
- **Telegram** — @fan_forge_bot on web.telegram.org or your phone

---

## Part 1 — Pre-flight: Verify Every Service Before You Touch the Bot

---

### Step 1.1 — Check your environment variables

**Run this in Terminal B:**
```bash
cd ~/Work/hackathons/fanforge
grep -E "^(TELEGRAM_BOT_TOKEN|AOMI_API_KEY|AOMI_APP|SUPABASE_URL|SUPABASE_ANON_KEY|PINATA_JWT)=" .env
```

**Expected output — all six lines, none empty:**
```
TELEGRAM_BOT_TOKEN=7xxxxxxxxx:AAF...
AOMI_API_KEY=aomi_...
AOMI_APP=default
SUPABASE_URL=https://xxxxxxxxxxx.supabase.co
SUPABASE_ANON_KEY=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
PINATA_JWT=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Why this check matters:** Every subsequent step in this guide depends on these values being present and correct. When your bot, your plugin, or any curl command calls one of the five external services, it reads its credential from this file. If a variable is missing or blank, the failure happens silently and deep inside a function call — you get a confusing error that looks like a network problem or a type error when the real cause is a missing string. This check costs you five seconds and eliminates an entire category of confusion before it can happen.

Why use `grep` to read the file instead of just opening it? Because `grep` shows you only the lines that matter, and it shows you the raw file content — not what Node.js happened to read at startup, not what the shell has in memory, but what is literally on disk. If a line shows `SUPABASE_URL=` with nothing after the equals sign, that is the bug, and you can fix it before wasting time starting the bot.

**Bug signals — what to do if something is wrong:**
- A line is missing entirely → the variable was never added to `.env`. Add it now using `.env.example` as a reference.
- A line shows `KEY=` with nothing after it → the value is blank. Go to the relevant service's dashboard and copy the key again.
- `AOMI_APP=` is blank → the bot will default to `"fanforge"` (the hardcoded fallback in `session.ts`), which will give 401 errors until the app is activated. Set it to `default` for now.

---

### Step 1.2 — Verify all three Supabase tables exist and are reachable

**Run this in Terminal B:**
```bash
source ~/Work/hackathons/fanforge/.env

echo "--- creator_coins ---"
curl -s "$SUPABASE_URL/rest/v1/creator_coins?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"

echo ""
echo "--- fan_missions ---"
curl -s "$SUPABASE_URL/rest/v1/fan_missions?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"

echo ""
echo "--- reward_distributions ---"
curl -s "$SUPABASE_URL/rest/v1/reward_distributions?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"
```

**Expected output from all three:**
```
--- creator_coins ---
[]
--- fan_missions ---
[]
--- reward_distributions ---
[]
```

**Why three separate checks:** Each table serves exactly one feature. `creator_coins` is written to when a creator launches a coin. `fan_missions` is written to when a creator creates a gated content mission. `reward_distributions` is written to when rewards are distributed to qualifying fans. If any one of these tables is missing, that specific feature will fail at the database write step, and the other two features will still work fine. Checking them separately tells you exactly which feature is blocked.

**Why the expected output is `[]` and not something else:** `[]` is an empty JSON array. It means: the table exists, your credentials are valid, and the REST API can reach it. There is no data in it yet — that is correct at this stage because you haven't run any features yet. You are verifying the container, not the contents.

**Why `?select=id&limit=1`:** This is a minimal query — it asks for just the `id` column and at most one row. It is faster than asking for all columns, and it proves the table exists and is queryable without transferring unnecessary data. You are not testing what is in the table; you are testing whether the table is reachable.

**Why both `-H "apikey: ..."` and `-H "Authorization: Bearer ..."` are needed:** Supabase requires both headers for REST API access. The `apikey` header is a Supabase-specific convention for their gateway. The `Authorization: Bearer` header is the standard OAuth/JWT authentication header. You need both because Supabase's gateway checks `apikey` first (rate limiting, project routing) and PostgREST checks `Authorization` second (row-level security). Omitting either one gives you a 401 or 403 error.

**Bug signals and what they mean:**

| What you see | What it means | What to do |
|---|---|---|
| `{"code":"PGRST205","message":"Could not find the table..."}` | The table does not exist in your Supabase project | Go to Supabase SQL Editor and run the CREATE TABLE SQL from the handoff |
| `{"message":"Invalid API key"}` | Your `SUPABASE_ANON_KEY` is wrong | Go to Supabase → Project Settings → API → copy the anon/public key |
| `{"message":"...permission denied..."}` | Row-level security is blocking the request | Add a public SELECT policy in Supabase → Authentication → Policies |
| `curl: (6) Could not resolve host` | The `SUPABASE_URL` is wrong or empty | Check Step 1.1 |

---

### Step 1.3 — Verify Pinata IPFS uploads succeed

**Run this in Terminal B:**
```bash
source ~/Work/hackathons/fanforge/.env

curl -s -X POST "https://api.pinata.cloud/pinning/pinJSONToIPFS" \
  -H "Authorization: Bearer $PINATA_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "pinataContent": {
      "name": "FanForge Test Coin",
      "symbol": "TEST",
      "description": "Smoke test — safe to ignore"
    },
    "pinataMetadata": {"name": "fanforge-smoke-test"}
  }'
```

**Expected output:**
```json
{"IpfsHash":"QmYxV...","PinSize":74,"Timestamp":"..."}
```

**Why this test exists:** Pinata is the first thing your Rust plugin calls during a coin launch. The flow inside `LaunchFanCoin` is: validate ticker → get wallet from context → upload metadata to IPFS → call Zora API → stage transaction. If Pinata fails at step 3, the entire coin launch fails before Zora is ever called. By testing Pinata first in isolation, you eliminate it as a suspect before testing the full launch flow.

**Why you're uploading a JSON object:** A Zora creator coin needs a metadata file describing it — name, symbol, description, and optionally an image URL. That metadata file lives on IPFS, not on Zora's servers, which means it's permanent and can't be changed after upload. The `pinataContent` field is exactly what gets stored on IPFS. The `pinataMetadata` field is just a label for your Pinata dashboard — it does not affect the stored content and is not visible to anyone else.

**What `IpfsHash` actually is:** IPFS (InterPlanetary File System) is a content-addressed storage network. "Content-addressed" means the address of a file is derived from its contents — specifically, a cryptographic hash of the bytes. The `Qm...` you see in the hash is a CID (Content Identifier) — a fingerprint of your JSON. If you upload the exact same JSON again, you get the exact same hash. This is what makes IPFS trustworthy for on-chain metadata: once a coin is launched pointing at `ipfs://QmYxV...`, that metadata can never be changed without changing the coin's metadata URI.

**Why `-X POST`:** The Pinata pinning API only accepts HTTP POST requests. GET requests are for retrieving content, not uploading it. Without `-X POST`, curl defaults to GET and Pinata returns a 404 or 405.

**Bug signals and what they mean:**

| What you see | What it means | What to do |
|---|---|---|
| `{"error":"UNAUTHORIZED"}` | Your `PINATA_JWT` is expired or wrong | Go to pinata.cloud → API Keys → Create New Key → copy the JWT |
| `{"error":"GATEWAY_TIMEOUT"}` | Pinata's API is temporarily slow | Wait 30 seconds and try again |
| Empty output or `curl: (6)` | Network issue or `PINATA_JWT` is blank | Check Step 1.1 |

---

### Step 1.4 — Verify the Zora API returns real data in the expected shape

**Run this in Terminal B:**
```bash
echo "=== /coin endpoint (used by GetCreatorRecap) ===" && \
curl -s "https://api-sdk.zora.engineering/coin?address=0x493e88b9ba3a479c03c28af366adff4457d58d94&chain=8453" \
  | python3 -c "
import sys, json
t = json.load(sys.stdin).get('zora20Token', {})
print('name:       ', t.get('name'))
print('symbol:     ', t.get('symbol'))
print('holders:    ', t.get('uniqueHolders'))
print('marketCap:  ', t.get('marketCap'))
print('volume24h:  ', t.get('volume24h'))
"
```

**Expected output:**
```
=== /coin endpoint (used by GetCreatorRecap) ===
name:        music
symbol:      music
holders:     6
marketCap:   2839.56
volume24h:   0.0
```

```bash
echo "=== /coinHolders endpoint (used by GetFanLeaderboard and DistributeRewards) ===" && \
curl -s "https://api-sdk.zora.engineering/coinHolders?address=0x493e88b9ba3a479c03c28af366adff4457d58d94&chainId=8453&count=5" \
  | python3 -c "
import sys, json
d = json.load(sys.stdin)
edges = d.get('zora20Token', {}).get('tokenBalances', {}).get('edges', [])
print(f'Total holders returned: {len(edges)}')
print()
for i, e in enumerate(edges):
    n = e['node']
    bal = float(n['balance']) / 1e18
    handle = n.get('ownerProfile', {}).get('handle', 'no handle')
    wallet = n.get('ownerAddress', 'unknown')
    short = wallet[:6] + '...' + wallet[-4:] if len(wallet) > 10 else wallet
    print(f'  #{i+1}  handle: {handle:<22}  balance: {bal:>15,.2f} coins  wallet: {short}')
"
```

**Expected output:**
```
=== /coinHolders endpoint (used by GetFanLeaderboard and DistributeRewards) ===
Total holders returned: 3

  #1  handle: 0x4985...2b2b          balance: 991,109,807.32 coins  wallet: 0x4985...2b2b
  #2  handle: skinnywhitegirl        balance:   8,244,103.54 coins  wallet: 0x4cdf...bebd
  #3  handle: madbeets               balance:     361,460.81 coins  wallet: 0xd204...20ad
```

**Save these numbers. You will use them for comparison in Feature 2 and Feature 5.**

**Why you're using a coin that isn't yours:** You cannot test "will the Zora API return data for my coin" before your coin exists. So instead you use a known real coin — the "music" coin at that address — to verify that your code's parsing logic works on live data. This test proves two distinct things: first, that you can reach `api-sdk.zora.engineering` from your network; second, that the Python parsing logic (which mirrors the Rust parsing logic in your plugin) correctly traverses `zora20Token → tokenBalances → edges → node → balance/ownerAddress/ownerProfile`.

**Why the balance parsing looks complex:** The Zora API returns balances as raw integer strings, not as decimal numbers. The raw balance `991109807323410696972909414` is not 991 billion coins — it is a large integer in the token's smallest unit (called "wei" for ETH-compatible chains). ERC-20 tokens have a `decimals` setting, and Zora creator coins use 18 decimals. That means 1 full coin = 10^18 raw units. So `991109807323410696972909414 / 10^18 = 991,109,807.32` readable coins, which is about 99% of the total supply. Your Rust tool does this same division: `balance_raw.parse::<f64>().map(|b| b / 1e18)`.

**Why the Python script accesses `zora20Token.tokenBalances.edges[].node`:** This is the actual JSON structure the Zora API returns. It is not obvious or intuitive — you would expect a flat list of holders at the top level. But Zora uses GraphQL internally, and the response reflects that nested structure. The path `zora20Token → tokenBalances → edges → node` is the exact same path your Rust tools traverse. If this Python script fails on this path, your Rust tools will fail the same way — which means a bug in the parsing.

**Bug signals and what they mean:**

| What you see | What it means | What to do |
|---|---|---|
| Empty output, no print statements | The Python script failed silently | Run the curl alone first without the pipe and look at the raw JSON |
| `KeyError: 'zora20Token'` | The API changed its response structure | Check the raw Zora response and update the parsing path in `tool.rs` |
| All balances show `0.0` | The balance field is not a string parseable as float | Inspect the raw edge and check the type of `node.balance` |
| `holders: None` | The `uniqueHolders` field moved in the API response | Check if it's now nested differently inside `zora20Token` |

---

### Step 1.5 — Verify the Aomi API session works with your key

**Run this in Terminal B:**
```bash
AOMI_KEY=$(grep '^AOMI_API_KEY=' ~/Work/hackathons/fanforge/.env | cut -d= -f2-)

aomi --api-key "$AOMI_KEY" --app default --new-session --prompt "hello" \
  2>&1 | grep -v "DeprecationWarning\|punycode"
```

**Expected output:**
```
Hello! I'm Aomi, your execution assistant. I can help you check balances...
```
Any coherent English response from the Aomi LLM means this step passes.

**Why this check is separate from testing the bot:** The Telegram bot is a relay. When you send a message to @fan_forge_bot, it creates an Aomi `Session` object and calls `session.send(yourMessage)`. That is an HTTP POST to `https://api.aomi.dev`. If that POST fails with 401, every single message in the Telegram bot will return "Something went wrong" — and you might spend 20 minutes inspecting bot code when the real issue is a single character wrong in your API key.

By calling `aomi` directly, you are calling the exact same API endpoint that `session.ts` calls, with the same key, but without Telegram or grammY involved. The `--new-session` flag forces a fresh session so you are not reusing any cached state. If this call works and the bot's message relay doesn't, the bug is definitively in the bot code. If this call doesn't work, the bug is definitively in your Aomi credentials.

**Why `--app default` and not `--app fanforge`:** The `default` app always exists — it is Aomi's built-in general EVM assistant. The `fanforge` app only exists after your PR is merged and activated. Testing with `default` confirms your API key and network connection are fine. Testing with `fanforge` before activation would give you a 401 or 404, which would be a false negative — it would look like your key is broken when actually the app just isn't deployed yet.

**Bug signals and what they mean:**

| What you see | What it means | What to do |
|---|---|---|
| `Error: HTTP 401: Unauthorized` | Your `AOMI_API_KEY` is wrong or expired | Go to aomi.dev dashboard and regenerate your API key |
| `Error: HTTP 404` | The `default` app doesn't exist for your account | Contact Aomi support |
| `Error: fetch failed` or `ECONNREFUSED` | Network issue or `AOMI_BASE_URL` is wrong | Check that `AOMI_BASE_URL=https://api.aomi.dev` in your `.env` |
| No response after 30+ seconds | The Aomi backend is slow or down | Try again; if it persists, check Aomi's status page |

---

## Part 2 — Bot Pipeline: Start and Verify the Relay

---

### Step 2.1 — Start the bot

**Run this in Terminal A:**
```bash
cd ~/Work/hackathons/fanforge/bot && npm run dev
```

**Expected Terminal A output within 3 seconds:**
```
> fanforge-bot@0.1.0 dev
> tsx watch src/index.ts

FanForge bot started as @fan_forge_bot
```

**What is actually happening here:** `tsx watch` compiles your TypeScript on the fly and starts the Node.js process. The bot immediately begins long-polling Telegram's servers — this means it sends a GET request to `https://api.telegram.org/bot<token>/getUpdates` every few seconds asking "are there any new messages for me?" When Telegram has a message, it sends it back in the response. This is the `polling` mode in grammY. It is how the bot receives messages without needing a public server or HTTPS endpoint.

`FanForge bot started as @fan_forge_bot` is printed by this callback in `index.ts`:
```typescript
bot.start({
  onStart: (info) => console.log(`FanForge bot started as @${info.username}`),
});
```
This callback only fires after the first successful long-poll request to Telegram confirms your token is valid. If the token is wrong, you never see this line.

**Leave Terminal A running and visible for the entire test session.** Every message, every error, every tool call will appear here. This is your most important debugging tool.

**Bug signals and what they mean:**

| What you see in Terminal A | What it means | What to do |
|---|---|---|
| `Error: 401: Unauthorized` before "started" | `TELEGRAM_BOT_TOKEN` is wrong | Go to Telegram → @BotFather → `/mybots` → select your bot → API Token |
| `Cannot find module './handlers/start.js'` | TypeScript compilation failed | Run `npm run typecheck` in Terminal B to see the error |
| Nothing at all after 10 seconds | Process crashed silently | Scroll up in Terminal A for the error |
| `FanForge bot started as @something_else` | You're using a different bot's token | Check which bot token is in `.env` |

---

### Step 2.2 — Test the /start command

**In Telegram, send:**
```
/start
```

**Expected bot reply in Telegram:**
```
Hey! I'm FanForge 🎵

I help music creators launch a fan economy — so your superfans can hold a piece of your journey.

It takes about 60 seconds to set up. Ready?

Just tell me what you'd like to do:
• "Launch a fan coin for my EP"
• "Show me who my top fans are"
• "Reward fans who hold at least 100 coins"
• "Give me my weekly recap"
```

**Expected Terminal A output:** Nothing. Complete silence.

**Why /start is always the first Telegram test:** `/start` is handled entirely inside the bot code — it never calls Aomi, never calls Zora, never touches any external service. Look at `bot/src/handlers/start.ts`: it closes the existing session, creates a new one, and calls `ctx.reply(...)` with hardcoded text. That's it. If `/start` works, you have confirmed:
1. `TELEGRAM_BOT_TOKEN` is valid — Telegram accepted your bot's identity
2. grammY is routing commands correctly
3. `ctx.reply()` can send messages back to the user
4. The dotenv loading in `index.ts` worked — because `closeSession` and `getOrCreateSession` both read `process.env.*` at runtime

If `/start` does not work, nothing else will. Fix this before testing anything else.

**Why Terminal A shows nothing for a successful /start:** The bot's logging strategy is: only log errors. A successful operation has no side effects worth logging. If you do see output in Terminal A during `/start`, it means an error was caught and logged — read it carefully.

---

### Step 2.3 — Test the basic message relay

**In Telegram, send:**
```
hello
```

**Expected bot reply:** Any coherent response from the Aomi LLM. It might say "Hello! I'm here to help you with on-chain actions..." or something similar. The exact words don't matter.

**Expected Terminal A output:** Silence.

**Why this test matters as its own step:** "hello" goes through the full message relay pipeline: grammY receives the message → `bot.on("message:text")` fires → `getOrCreateSession(userId)` creates or retrieves an Aomi session → `session.send("hello")` sends the text to Aomi's API → Aomi processes it → returns `result.messages` → your `extractText()` function extracts the content → `sendChunked()` sends it back to Telegram. Every single component of the relay is exercised by this one message.

The fact that the response is generic and unhelpful is not a bug — it is expected behaviour because `AOMI_APP=default`. What you are proving with this test is that the pipeline from Telegram through your Node.js bot to Aomi and back is fully functional.

**If this test fails but /start succeeded:** The bug is specifically in the `session.send()` → response handling path. Look at Terminal A. The line starting with `Aomi session error:` will tell you the exact error. Common causes:
- `HTTP 401` → `AOMI_API_KEY` is not being read at runtime (dotenv issue)
- `Cannot read properties of undefined (reading 'filter')` → `result.messages` came back as `undefined`
- `TypeError: fetch failed` → network connectivity issue

---

## Part 3 — Feature Tests: What You See Now vs. After Activation

This section tests all five FanForge features. For each feature you will see two columns: what the bot replies **now** with `AOMI_APP=default`, and what it should reply **after activation** with `AOMI_APP=fanforge`. Knowing what "now" looks like is as important as knowing what "after" looks like — it lets you immediately tell whether the activation happened or not.

---

### Feature 1 — Coin Launch

**In Telegram, send:**
```
I want to launch a fan coin called Temi Coin with the ticker TEMI. It's for my fans who want to support my music journey.
```

**NOW (AOMI_APP=default) — expected reply:**
The bot will say something like "Launching your fan coin Temi Coin ($TEMI) on Base is a great idea. Based on current best practices, the standard way to launch an AI-driven fan coin is through Clanker. To proceed, please connect your wallet..."

The response is generic. It suggests Clanker (a different token launcher entirely). It asks you to connect a wallet in a vague way. It does not return a Zora link. **No Rust code runs. No Pinata upload happens. No Zora API call is made. No Supabase write happens.**

**Terminal A for this test:** Silence. Because the default app handled everything on Aomi's side — your plugin was never loaded.

**AFTER activation (AOMI_APP=fanforge) — expected reply:**
The LLM will first confirm the details:

> Let me confirm before we launch:
> • Name: Temi Coin
> • Ticker: $TEMI
> • Description: for fans who want to support my music journey
>
> Ready to deploy?

After your confirmation, it triggers the wallet approval flow. Your Aomi-connected wallet will show a transaction to approve. Once you approve it:

> Your Temi Coin ($TEMI) is live! Share this with your fans:
> https://zora.co/coin/base:0x...

**Terminal A after activation:** Silence for success. If anything goes wrong in the Rust code, you will see lines like:
```
Aomi session error: [fanforge] Pinata upload failed: ...
Aomi session error: [fanforge] Zora POST /create/content returned 400: ...
Aomi session error: wallet_not_connected: your fan economy wallet isn't linked yet
```

**Verify the launch actually happened — run in Terminal B:**
```bash
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/creator_coins?select=ticker,coin_address,zora_url,created_at" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -m json.tool
```

**Expected output after a successful launch:**
```json
[
    {
        "ticker": "TEMI",
        "coin_address": "0x...",
        "zora_url": "https://zora.co/coin/base:0x...",
        "created_at": "2026-06-02T..."
    }
]
```

**Why the Supabase check is mandatory after the launch:** Getting a Zora URL in the Telegram reply proves that `LaunchFanCoin` ran and the Zora calldata was submitted. It does NOT prove that `FinalizeLaunch` ran. `FinalizeLaunch` is the last step in the route chain — it writes the coin record to Supabase. If `FinalizeLaunch` failed silently (for example, the `SUPABASE_URL` secret wasn't injected into the Aomi session), the coin would exist on-chain at that Zora URL, but FanForge would have no record of it — meaning missions and recaps would fail to find the coin later.

**Why the Zora URL contains `base:` in it:** The URL format `https://zora.co/coin/base:0x...` tells Zora's frontend which blockchain network this coin lives on. Zora supports multiple chains. The `base:` prefix specifies Base mainnet (chain ID 8453). This prefix is constructed in your `FinalizeLaunch` Rust code: `format!("https://zora.co/coin/base:{}", args.predicted_coin_address.to_lowercase())`.

**Why the coin address is deterministic before the transaction confirms:** Zora uses a deployment pattern called `CREATE2`, which lets a smart contract compute the address of another contract it's about to deploy based on fixed inputs (the creator's wallet, the coin parameters, and a salt). This is why Zora's `/create/content` API can return a `predictedCoinAddress` before the transaction is submitted — it calculated it mathematically. Your plugin stores this address in Supabase immediately after the transaction, without waiting for on-chain confirmation, because the address is guaranteed.

**Bug signals post-activation:**

| What you see | What it means |
|---|---|
| `wallet_not_connected` | The user hasn't connected their wallet to Aomi yet |
| `[fanforge] Pinata upload failed: 401` | `PINATA_JWT` secret was not injected into the Aomi session |
| `[fanforge] Zora POST /create/content returned 400` | The payload sent to Zora is malformed |
| `[fanforge] Supabase POST creator_coins returned 401` | `SUPABASE_URL` or `SUPABASE_ANON_KEY` secrets not injected |
| Zora URL in Telegram but empty Supabase result | `FinalizeLaunch` failed — transaction succeeded but DB write did not |

---

### Feature 2 — Fan Leaderboard

**In Telegram, send:**
```
Show me the top fans holding coin 0x493e88b9ba3a479c03c28af366adff4457d58d94
```

**NOW (AOMI_APP=default) — expected reply:**
Something like: "The address `0x493e...` is an unverified contract on Ethereum Mainnet. I cannot query a top holders list without a connected wallet..." The default app tries to use Base RPC directly but fails without a wallet and suggests DexScreener instead. **No Zora API call is made by your plugin.**

**AFTER activation (AOMI_APP=fanforge) — expected reply:**
```
Top supporters:

#1  0x4985…2b2b          991,109,807 coins
#2  skinnywhitegirl         8,244,103 coins
#3  madbeets                  361,460 coins
#4  laodin                    284,628 coins
#5  0xbc1d…c973                    0 coins
```

**How to verify the data is correct:** Compare these numbers directly to what you got in Step 1.4. They should match — or be very close, allowing for minor trading activity between the two calls.

**Why the leaderboard data comes from Zora and not Supabase:** Zora is the authoritative source for who holds what. Your Supabase only stores missions and rewards — it does not track holder balances, because balances change every time someone buys or sells the coin. Reading from Zora on every leaderboard request means the data is always live.

**Why some holders show wallet addresses instead of handles:** Not every wallet owner has created a Zora profile with a display name. For those wallets, `ownerProfile.handle` defaults to a shortened wallet address. Your code has this fallback: it tries the handle first, and if the profile is null, it uses the shortened wallet address.

**Bug signals post-activation:**

| What you see | What it means |
|---|---|
| Empty leaderboard `[]` | Zora returned no holders, or the `zora20Token` path changed |
| All balances `0.00` | The balance field parsing failed — check the raw Zora response with curl |
| `[fanforge] Zora GET /coinHolders... failed` | Network issue or Zora rate limit — add a `ZORA_API_KEY` |

---

### Feature 3 — Create a Fan Mission

**In Telegram, send:**
```
Create a mission: fans holding 100 or more coins of 0x493e88b9ba3a479c03c28af366adff4457d58d94 should unlock access to my unreleased track at https://drive.google.com/file/d/your-file-id
```

**NOW (AOMI_APP=default) — expected reply:**
Generic response. The bot might say "I've set up the configuration for your fan mission..." but this is the LLM improvising — **no row is written to Supabase**.

**AFTER activation (AOMI_APP=fanforge) — expected reply:**
```
Mission created!

Fans holding 100 or more coins can unlock: unreleased track access
Mission ID: 3f7a2b1c-...

Run distribute rewards whenever you're ready to send it out.
```

**Copy the Mission ID. You will need it for Feature 4.**

**Verify the mission was actually saved — run in Terminal B:**
```bash
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/fan_missions?select=*" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -c "
import sys, json
missions = json.load(sys.stdin)
if not missions:
    print('No missions found — something went wrong')
else:
    for m in missions:
        print('id:          ', m['id'])
        print('coin_address:', m['coin_address'])
        print('title:       ', m['title'])
        print('threshold:   ', m['threshold'])
        print('content_url: ', m['content_url'])
        print('status:      ', m['status'])
"
```

**Expected output:**
```
id:           3f7a2b1c-...
coin_address: 0x493e88b9ba3a479c03c28af366adff4457d58d94
title:        unreleased track access
threshold:    100
content_url:  https://drive.google.com/file/d/your-file-id
status:       active
```

**Why `threshold` is stored as `NUMERIC` and not `FLOAT`:** SQL's `NUMERIC` type stores exact decimal values without floating-point rounding. If you set a threshold of 100.5 coins, it stores exactly `100.5`. A `FLOAT` column might store `100.49999999...` due to floating-point representation. Since this threshold is later compared directly against holder balances, exactness matters. If the threshold rounds down, some fans might qualify who shouldn't; if it rounds up, some fans might be excluded who should qualify.

**Why the mission stores `content_url` instead of the actual content:** FanForge is a delivery mechanism, not a content host. It records where the content lives (a Google Drive link, a YouTube link, a private server URL) and delivers that link to qualifying fans. The creator is responsible for making sure the URL is only accessible if you have the link — FanForge doesn't gate the content itself, it gates delivery of the link.

---

### Feature 4 — Distribute Rewards

**In Telegram, send (paste the actual mission ID from Feature 3):**
```
Distribute the rewards for mission 3f7a2b1c-[your-full-uuid-here]
```

**NOW (AOMI_APP=default) — expected reply:**
Generic response. No rows are written to `reward_distributions`.

**AFTER activation (AOMI_APP=fanforge) — expected reply:**
```
3 fans just unlocked the content. 0 had already received it.
```

**Verify distributions were recorded — run in Terminal B:**
```bash
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/reward_distributions?select=*" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -c "
import sys, json
rows = json.load(sys.stdin)
print(f'Total distributions recorded: {len(rows)}')
for r in rows:
    print(f'  mission: {r[\"mission_id\"][:8]}...  wallet: {r[\"recipient_wallet\"][:10]}...  at: {r[\"delivered_at\"]}')
"
```

**Expected output:**
```
Total distributions recorded: 3
  mission: 3f7a2b1c...  wallet: 0x498581...  at: 2026-06-02T...
  mission: 3f7a2b1c...  wallet: 0x4cdfbb...  at: 2026-06-02T...
  mission: 3f7a2b1c...  wallet: 0xd2043d...  at: 2026-06-02T...
```

**Now run the idempotency test — send the exact same message again:**
```
Distribute the rewards for mission 3f7a2b1c-[same-uuid]
```

**Expected reply the second time:**
```
0 fans just unlocked the content. 3 had already received it.
```

**Run the Supabase check again — the count must not increase:**
```bash
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/reward_distributions?select=id" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -c "import sys,json; rows=json.load(sys.stdin); print(f'Row count: {len(rows)} — should still be 3')"
```

**Expected output:** `Row count: 3 — should still be 3`

**Why the idempotency test is the most important test in the entire guide:** This test verifies a property called idempotency — the guarantee that running an operation multiple times produces the same result as running it once. In FanForge's case: a fan should never receive the same reward twice, even if the creator accidentally clicks distribute twice, or the bot retries after a network timeout.

The protection comes from a database constraint defined when you created the table:
```sql
UNIQUE (mission_id, recipient_wallet)
```
This tells Postgres: "no two rows in `reward_distributions` can have the same combination of `mission_id` and `recipient_wallet`." If your Rust code tries to insert a row for a fan who already received the reward, Postgres rejects the insert with a unique violation error. Your code handles this gracefully — it only increments the counter if the insert succeeded. Failed inserts (duplicates) are silently ignored.

**If the second distribute call says "3 fans just unlocked" again:** The UNIQUE constraint is not in place. To fix it, go to Supabase SQL Editor and run:
```sql
ALTER TABLE reward_distributions
  ADD CONSTRAINT reward_distributions_mission_id_recipient_wallet_key
  UNIQUE (mission_id, recipient_wallet);
```

---

### Feature 5 — Creator Recap

**In Telegram, send:**
```
Give me my weekly recap for coin 0x493e88b9ba3a479c03c28af366adff4457d58d94
```

**NOW (AOMI_APP=default) — expected reply:**
The bot will fabricate data. It might say something about Clanker market performance or invent a holder count. The numbers will not match what you saw in Step 1.4. This is the LLM improvising without your tools — it is making things up because it doesn't have access to the real Zora data.

**AFTER activation (AOMI_APP=fanforge) — expected reply:**
```
Over the last 7 days, your fan economy is growing. $music now has 6 fans holding your coin. You have 1 active mission delivering exclusive content to your top supporters. Market cap: $2839.56. 24h volume: $0.0.

---
my coin $music now has 6 holders 🔥
holding = access. you already know what's inside 🔐
new mission dropping soon for my real ones
link in bio to join
```

**Verify the numbers are real — compare against Step 1.4:**
- `6 fans` should match `uniqueHolders: 6` from your earlier curl
- `$2839.56` should match `marketCap: 2839.56` from your earlier curl
- `1 active mission` should match the single fan mission you created in Feature 3

If all three numbers match, the recap is reading live data correctly.

**Why the recap has two sections — a summary and a social post:** The summary is for the creator to read privately. The social post is designed to be copy-pasted to Twitter, TikTok, or Instagram without any editing. Both are generated entirely by your Rust code using `format!()` strings with the real values injected — the LLM does not write them. This is intentional: you cannot have an AI inventing a holder count in text that a creator is going to publish. The social post must contain the exact real number.

**Why the `active_missions` count in the recap comes from Supabase and not Zora:** Missions are a FanForge concept — Zora knows nothing about them. Zora only knows about the coin and its holders. FanForge's Supabase database is where missions live. The recap queries both sources: Zora for the coin's financial metrics, Supabase for the creator's active mission count.

**Bug signal:** If the holder count in the recap shows `0` or `None` but your Step 1.4 showed `6`, the `zora20Token` wrapper is not being unwrapped in the `GetCreatorRecap` Rust code. Check `plugin/src/tool.rs` in the `GetCreatorRecap` implementation — the line `let token = coin_data.get("zora20Token")...` must be present.

---

## Part 4 — The Single Switch That Changes Everything

When the Aomi team activates your app, the only change you need to make is in `.env`:

**Open the file:**
```bash
nano ~/Work/hackathons/fanforge/.env
```

**Find and change this line:**
```
AOMI_APP=default
```
**To:**
```
AOMI_APP=fanforge
```

**Save and exit** (`Ctrl+O`, `Enter`, `Ctrl+X` in nano).

**Restart the bot** — in Terminal A, press `Ctrl+C`, then:
```bash
npm run dev
```

**Send `/start` in Telegram** to get a fresh session.

**The reason this one line is the entire difference:** Your bot sends `session.send(message)` to Aomi. Inside the `Session` object, the `app` field was set at creation time from `process.env.AOMI_APP`. With `app: "default"`, Aomi routes your message to the built-in EVM assistant. With `app: "fanforge"`, Aomi loads your Rust plugin and gives the LLM access to your 5 tools. Every other layer — the relay, Zora, Supabase, Pinata — is already working today. The activation is purely an Aomi backend configuration: it registers the `fanforge` app and tells the runtime which plugin binary to load for sessions that request it.

---

## Part 5 — Reading Terminal A: Your Bug Decoder

Every error your Rust plugin produces follows the format `[fanforge] <service> <operation> <error>`. When the bot catches these errors, it logs them exactly:

```
Aomi session error: [fanforge] Zora POST /create/content returned 400: {"error": "..."}
Aomi session error: [fanforge] Pinata upload failed: 401 Unauthorized
Aomi session error: [fanforge] Supabase GET fan_missions returned 404: table not found
Aomi session error: wallet_not_connected: your fan economy wallet isn't linked yet
Aomi session error: ticker_invalid: 'AB' must be 3–5 letters
```

The prefix tells you which service failed. The HTTP status tells you why. Use this table:

| Error prefix | Which layer failed | Most common cause |
|---|---|---|
| `[fanforge] Zora GET` | Zora read API | Rate limit — add `ZORA_API_KEY` to get higher limits |
| `[fanforge] Zora POST` | Zora write API | Malformed request payload or wrong chain ID |
| `[fanforge] Pinata` | IPFS upload | Expired or missing `PINATA_JWT` secret |
| `[fanforge] Supabase GET` | Database read | Table doesn't exist or wrong `SUPABASE_URL` |
| `[fanforge] Supabase POST` | Database write | Secret not injected into Aomi session, or constraint violation |
| `wallet_not_connected` | Aomi session state | User hasn't connected their wallet to the Aomi session |
| `ticker_invalid` | Your own validation | User typed a ticker that's too short, too long, or contains digits |
| `mission_not_found` | Database read | Mission ID was wrong or the row doesn't exist |
| `mission_not_active` | Business logic | Mission status is not `active` — it may have been completed or expired |

If Terminal A shows nothing at all during a Telegram failure, the error happened inside grammY or in the response formatting, not in the Aomi call. In that case, temporarily add `console.log(JSON.stringify(result, null, 2))` in `index.ts` before the `extractText` call to inspect the raw Aomi response shape, then restart the bot and reproduce the failure.

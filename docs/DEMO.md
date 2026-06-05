---
FanForge Demo & Test Guide

You need three things open at the same time:
- Terminal A — running the bot (you watch the logs here)
- Terminal B — running individual verification commands
- Telegram — @fan_forge_bot (web.telegram.org or your phone)

---
Part 1 — Pre-flight: Verify Infrastructure

Run every command in Terminal B. Each one should produce the exact output shown. If it doesn't, stop — something is broken at that layer.

---
1.1 — Environment variables are all loaded

cd ~/Work/hackathons/fanforge
grep -E "^(TELEGRAM_BOT_TOKEN|AOMI_API_KEY|AOMI_APP|SUPABASE_URL|SUPABASE_ANON_KEY|PINATA_JWT)=" .env

Expected output — all 6 lines present, none empty:
TELEGRAM_BOT_TOKEN=<value>
AOMI_API_KEY=<value>
AOMI_APP=default
SUPABASE_URL=https://<your-project>.supabase.co
SUPABASE_ANON_KEY=<value>
PINATA_JWT=<value>

Bug signal: Any line missing or showing = with nothing after it means that service will fail silently when called.

---
1.2 — Supabase: all three tables exist and respond

source ~/Work/hackathons/fanforge/.env

curl -s "$SUPABASE_URL/rest/v1/creator_coins?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"
Expected: []

curl -s "$SUPABASE_URL/rest/v1/fan_missions?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"
Expected: []

curl -s "$SUPABASE_URL/rest/v1/reward_distributions?select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY"
Expected: []

Bug signal: If you see {"code":"PGRST205"...} the table doesn't exist — run the SQL from the previous step again. If you see {"message":"Invalid API key"} your SUPABASE_ANON_KEY is wrong.

---
1.3 — Pinata IPFS: uploads succeed

source ~/Work/hackathons/fanforge/.env

curl -s -X POST "https://api.pinata.cloud/pinning/pinJSONToIPFS" \
  -H "Authorization: Bearer $PINATA_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "pinataContent": {"name":"FanForge Test","symbol":"TEST","description":"smoke test"},
    "pinataMetadata": {"name":"fanforge-smoke-test"}
  }'

Expected:
{"IpfsHash":"Qm...","PinSize":74,"Timestamp":"..."}

Bug signal: {"error":"UNAUTHORIZED"} means your JWT is expired or wrong. Go to pinata.cloud → API Keys → regenerate.

---
1.4 — Zora API: read endpoints return real data

curl -s "https://api-sdk.zora.engineering/coin?address=0x493e88b9ba3a479c03c28af366adff4457d58d94&chain=8453" \
  | python3 -c "
import sys, json
t = json.load(sys.stdin).get('zora20Token', {})
print('name:    ', t.get('name'))
print('symbol:  ', t.get('symbol'))
print('holders: ', t.get('uniqueHolders'))
print('marketCap:', t.get('marketCap'))
"

Expected:
name:     music
symbol:   music
holders:  6
marketCap: 2839.56

curl -s "https://api-sdk.zora.engineering/coinHolders?address=0x493e88b9ba3a479c03c28af366adff4457d58d94&chainId=8453&count=5" \
  | python3 -c "
import sys, json
edges = json.load(sys.stdin)['zora20Token']['tokenBalances']['edges']
for i, e in enumerate(edges):
    n = e['node']
    bal = float(n['balance']) / 1e18
    handle = n.get('ownerProfile', {}).get('handle', 'unknown')
    print(f'#{i+1} {handle:<22} {bal:>14,.2f} coins')
"

Expected:
#1 0x4985...2b2b           991,109,807.32 coins
#2 skinnywhitegirl           8,244,103.54 coins
#3 madbeets                    361,460.81 coins

Bug signal: Empty output or KeyError: 'zora20Token' means the Zora API changed its response shape again — our parsing code needs updating.

---
1.5 — Aomi session: API key is valid

AOMI_KEY=$(grep '^AOMI_API_KEY=' ~/Work/hackathons/fanforge/.env | cut -d= -f2-)

aomi --api-key "$AOMI_KEY" --app default --new-session --prompt "hello" 2>&1 | grep -v "DeprecationWarning\|punycode"

Expected: A response starting with something like Hello! I'm Aomi... or Hello! I'm here to help...

Bug signal: Error: HTTP 401: Unauthorized means your AOMI_API_KEY is wrong or expired. Error: HTTP 404 means the default app doesn't exist on this account.

---
Part 2 — Bot Pipeline Test

2.1 — Start the bot

In Terminal A:
cd ~/Work/hackathons/fanforge/bot && npm run dev

Expected terminal output:
> fanforge-bot@0.1.0 dev
> tsx watch src/index.ts

FanForge bot started as @fan_forge_bot

Bug signal: Any error before "FanForge bot started" means either TELEGRAM_BOT_TOKEN is wrong, or a syntax error in the bot code. The error message in the terminal will tell you which.

Leave Terminal A running and watch it as you send Telegram messages — every message and error will appear here.

---
2.2 — /start command

In Telegram, send:
/start

Expected bot reply:
Hey! I'm FanForge 🎵

I help music creators launch a fan economy...

Just tell me what you'd like to do:
• "Launch a fan coin for my EP"
• "Show me who my top fans are"
...

Expected Terminal A output: Nothing (no logs for successful /start)

Bug signal: No reply after 10 seconds → check Terminal A for errors. "Something went wrong" reply → Terminal A will show the specific error on the line starting with Aomi session error:.

---
2.3 — Basic message relay (proves Aomi pipeline works)

In Telegram, send:
hello

Expected bot reply: Any coherent response from Aomi — even a generic "Hello! How can I help you..." is correct because we're on the default app.

Expected Terminal A output: Nothing unless there's an error.

Bug signal: "Something went wrong" → look at Terminal A. The line Aomi session error: will tell you the exact error. Common ones:
- HTTP 401 → API key issue
- HTTP 404 → session creation failed
- Cannot read properties of undefined (reading 'filter') → the result.messages came back in an unexpected shape

---
Part 3 — Feature Tests (Current Behaviour vs. Post-Activation)

Run each test in Telegram. For each one I show you: the message to send, what you get NOW with default app, and what you will get AFTER fanforge is activated.

---
Feature 1 — Coin Launch

Send in Telegram:
I want to launch a fan coin called Temi Coin with the ticker TEMI. It's for my fans who want to support my music journey.

NOW (default app) — expected reply:
The bot responds with something suggesting Clanker or asking you to connect a wallet generically. It will NOT return a Zora link. It may say something like "To launch a fan coin on Base, I can use Clanker..."

Terminal A — expected: No error lines. Just silence.

AFTER activation (fanforge app) — expected reply:
Let me confirm before we launch:

• Name: Temi Coin
• Ticker: $TEMI
• Description: for fans who want to support my music journey

Ready to deploy?
Then after you confirm, it should trigger your wallet to approve a transaction, and finally return:
Your Temi Coin ($TEMI) is live! Share this with your fans:
https://zora.co/coin/base:0x...

How to verify the launch actually worked:
After getting the Zora link, open it in your browser. You should see the coin page on Zora. Then check Supabase:
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/creator_coins?select=ticker,coin_address,zora_url" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -m json.tool
Expected post-launch: A record with ticker: "TEMI", a real coin_address, and a zora_url.

Bug signal post-activation: If the bot starts the flow but wallet approval never appears, the stage_tx step is not being reached. If wallet approval appears but returns an error, the Zora calldata is malformed. If everything completes but no Supabase record exists, the FinalizeLaunch tool failed silently — check the bot console.

---
Feature 2 — Fan Leaderboard

Send in Telegram:
Show me the top fans holding the music coin at 0x493e88b9ba3a479c03c28af366adff4457d58d94

NOW (default app) — expected reply:
Generic response saying it can't query holders without a connected wallet, or suggesting Etherscan/DexScreener.

AFTER activation (fanforge app) — expected reply:
Top supporters for music ($music):

#1 0x4985…2b2b     991,109,807 coins
#2 skinnywhitegirl   8,244,103 coins
#3 madbeets            361,460 coins
...

How to verify the leaderboard data is real:
Cross-check against what you got in Step 1.4 above — the handles and numbers should match.

Bug signal post-activation: If the leaderboard shows 0 entries or all zeros, the zora20Token.tokenBalances.edges path changed in the Zora API again. Check by running the curl from Step 1.4 and comparing the shape.

---
Feature 3 — Create a Fan Mission

Send in Telegram:
Create a mission: fans holding 100 or more coins of 0x493e88b9ba3a479c03c28af366adff4457d58d94 get access to my unreleased track at https://drive.google.com/file/d/abc123

NOW (default app) — expected reply:
Generic response. No data written to Supabase.

AFTER activation (fanforge app) — expected reply:
Mission created!

• Title: Unreleased track access
• Minimum holding: 100 coins
• Status: Active

Mission ID: [uuid]
Run distribute rewards whenever you're ready to send it out.

How to verify the mission was actually saved:
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/fan_missions?select=*" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -m json.tool
Expected: A record with status: "active", the correct threshold, content_url, and coin_address.

Bug signal: Empty [] after sending the message means the Supabase write failed. Check Terminal A for the error. Most likely cause: SUPABASE_URL or SUPABASE_ANON_KEY wasn't injected as a secret into the Aomi session.

---
Feature 4 — Distribute Rewards

First get the mission ID from the Supabase check above, then:

Send in Telegram:
Distribute rewards for mission [paste the uuid from step 3]

NOW (default app) — expected reply:
Generic response. No reward_distributions rows written.

AFTER activation (fanforge app) — expected reply:
Done! 3 fans just unlocked the content. 0 had already received it.
(Numbers will be based on how many real holders of that coin meet the 100-coin threshold.)

How to verify distributions were recorded:
source ~/Work/hackathons/fanforge/.env
curl -s "$SUPABASE_URL/rest/v1/reward_distributions?select=*" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Bearer $SUPABASE_ANON_KEY" \
  | python3 -m json.tool
Expected: One row per qualifying wallet, each with mission_id, recipient_wallet, delivered_at.

Idempotency test — send the same message again:
Distribute rewards for mission [same uuid]
Expected: "0 fans just unlocked the content. 3 had already received it." — the same fans are NOT counted twice. This is the DB UNIQUE constraint working. If you see the count go up again, the constraint is not in place.

---
Feature 5 — Creator Recap

Send in Telegram:
Give me my weekly recap for coin 0x493e88b9ba3a479c03c28af366adff4457d58d94

NOW (default app) — expected reply:
Invented market data. It might say something about Clanker or fabricate a holder count.

AFTER activation (fanforge app) — expected reply:
Over the last 7 days, your fan economy is growing. $music now has 6 fans holding your coin. You have 1 active mission delivering exclusive content to your top supporters. Market cap: $2839.56. 24h volume: $0.

---
my coin $music now has 6 holders 🔥
holding = access. you already know what's inside 🔐
new mission dropping soon for my real ones
link in bio to join

How to verify the numbers are real: Cross-check uniqueHolders: 6 and marketCap: 2839.56 against your Step 1.4 output. They should match exactly.

Bug signal: If holder count is 0 or market cap is missing, zora20Token wrapper is not being unwrapped in GetCreatorRecap. Check plugin/src/tool.rs around line 260.

---
Part 4 — How to Read Terminal A Logs

While testing, watch Terminal A. Here is what each log line means:

┌────────────────────────────────────────────────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────┐
│                            What you see                            │                                    What it means                                    │
├────────────────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────┤
│ Silence                                                            │ Request succeeded                                                                   │
├────────────────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────┤
│ Aomi session error: HTTP 401                                       │ Your API key is wrong or expired                                                    │
├────────────────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────┤
│ Aomi session error: HTTP 404                                       │ The app name doesn't exist on the backend yet — still waiting for activation        │
├────────────────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────┤
│ Aomi session error: TypeError: Cannot read properties of undefined │ The Aomi response came back in an unexpected shape — check the extractText function │
├────────────────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────┤
│ Aomi session error: fetch failed                                   │ Network issue or Aomi backend is down                                               │
└────────────────────────────────────────────────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────┘

The single most useful command when something breaks:
# In Terminal B — shows the last 50 lines of bot output
# (only if you redirected output to a file — otherwise watch Terminal A directly)

Actually the simplest debug approach: when a message fails in Telegram, immediately look at Terminal A. The exact error is on the line that starts with Aomi session error:. That error message tells you exactly which layer failed.

---
Part 5 — The Switch

When activation happens, the single change is in .env:

# Open the file
nano ~/Work/hackathons/fanforge/.env

# Change this line:
AOMI_APP=default
# to:
AOMI_APP=fanforge

Then in Terminal A, press Ctrl+C to stop the bot and restart:
npm run dev

Send /start again in Telegram to get a fresh session, then run through Features 1–5 above. Every test from Feature 1 onwards should now hit the FanForge tools instead of the default EVM assistant.

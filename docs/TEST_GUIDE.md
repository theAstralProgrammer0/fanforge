FanForge — Complete Test & Demo Guide

---
Before You Begin: What You Are Actually Testing

FanForge is not one system. It is five independent external services wired together through a Rust plugin and a TypeScript bot. When something breaks in a demo, the failure could be in any one of these layers:

You (Telegram) → grammY bot → Aomi session API → fanforge Rust plugin
                                                          ↓
                                               Pinata (IPFS upload)
                                               Zora API (coin deploy)
                                               Supabase (state writes)

The purpose of this guide is to let you verify each layer independently before you trust the whole chain. That way, when something goes wrong during the demo, you can say "it's not Supabase — I confirmed that 10 minutes ago — so the problem must be in the Aomi routing." Without this isolation discipline, you will waste precious demo time chasing ghosts.

You need three things open simultaneously:
- Terminal A — for running the bot (you watch logs here)
- Terminal B — for runn
- Telegram — @fan_forge_bot on web.telegram.org or your phone

---
Part 1 — Pre-flight: Ve Touch the Bot

---
Step 1.1 — Check your environment variables

Run this in Terminal B:
cd ~/Work/hackathons/fa
grep -E "^(TELEGRAM_BOT_TOKEN|AOMI_API_KEY|AOMI_APP|SUPABASE_URL|SUPABASE_ANON_KEY|PINATA_JWT)=" .en

Expected output — all s
TELEGRAM_BOT_TOKEN=7xxxxxxxxx:AAF...
AOMI_API_KEY=aomi_...
AOMI_APP=default
SUPABASE_URL=https://xx
SUPABASE_ANON_KEY=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
PINATA_JWT=eyJhbGciOiJI

Why this check matters:s guide depends on thesevalues being present and correct. When your bot, your plugin, or any curl command calls one of threads its credential from this file. If a variable is missing or blank, the failure happens silently
and deep inside a functg error that looks like a network problem or a type error when the real cause is a missing string.
This check costs you fientire category ofconfusion before it can happen.

Why use grep to read the file instead of just opening it? Because grep shows
you only the lines that raw file content — notwhat Node.js happened to read at startup, not what the shell has in memory,
but what is literally oASE_URL= with nothingafter the equals sign, that is the bug, and you can fix it before wasting
time starting the bot.

Bug signal — what to do
- A line is missing entirely → the variable was never added to .env. Add it now using .env.example
- A line shows KEY= with nothing after it → the value is blank. Go to the
relevant service's dash
- AOMI_APP= is blank → the bot will default to "fanforge" (the hardcoded
fallback in session.ts)until the app isactivated. Set it to default for now.

---
Step 1.2 — Verify all td are reachable

Run this in Terminal B:
source ~/Work/hackathons/fanforge/.env

echo "--- creator_coins ---"
curl -s "$SUPABASE_URL/id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Be

echo ""
echo "--- fan_missions ---"
curl -s "$SUPABASE_URL/d&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Be

echo ""
echo "--- reward_distributions ---"
curl -s "$SUPABASE_URL/select=id&limit=1" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Be

Expected output from al
--- creator_coins ---
[]
--- fan_missions ---
[]
--- reward_distributions ---
[]

Why three separate chec one feature.creator_coins is written to when a creator launches a coin. fan_missions is written to when a creatssion.reward_distributions is written to when rewards are distributed to qualifyinfans. If any one of theecific feature will fail at the database write step, and the other two features will still work fine.Checking them separatelture is blocked.
                                                                            Why the expected output: [] is an empty JSONarray. It means: the table exists, your credentials are valid, and the REST API can reach it. Thereis correct at this stagebecause you haven't run any features yet. You are verifying the container,
not the contents.

Why ?select=id&limit=1: asks for just the idcolumn and at most one row. It is faster than asking for all columns, and it proves the table existsnsferring unnecessarydata. You are not testing what is in the table; you are testing whether the
table is reachable.

Why the -H "Authorizatided alongside -H "apikey: ...": Supabase requires both headers for REST API access. The apikey header
is a Supabase-specific  The Authorization:Bearer header is the standard OAuth/JWT authentication header. You need both
because Supabase's gate limiting, projectrouting) and PostgREST checks Authorization second (row-level security).
Omitting either one giv

Bug signals and what th

What you see: {"code":"t find the table..."}
What it means: The table does not exist in your Supabase project
What to do: Go to SupabEATE TABLE SQL from the
  handoff
───────────────────────
What you see: {"message":"Invalid API key"}
What it means: Your SUP
What to do: Go to Supabase → Project Settings → API → copy the anon/public
key
────────────────────────────────────────
What you see: {"message
What it means: Row-level security is blocking the request
What to do: Add a publi Authentication →Policies
───────────────────────
What you see: curl: (6) Could not resolve host
What it means: The SUPA
What to do: Check Step 1.1

---
Step 1.3 — Verify Pinat
                                                                           Run this in Terminal B:
source ~/Work/hackathons/fanforge/.env                                     
curl -s -X POST "https://api.pinata.cloud/pinning/pinJSONToIPFS" \           -H "Authorization: Be
  -H "Content-Type: application/json" \                                      -d '{
    "pinataContent": {                                                           "name": "FanForge
      "symbol": "TEST",                                                          "description": "S
    },                                                                         "pinataMetadata": {}
  }'                                                                       
Expected output:                                                           {"IpfsHash":"QmYxV...",6-06-02T..."}
                                                                           Why this test exists: P Rust plugin calls during a coin launch. The flow inside LaunchFanCoin is: validate ticker → get     wallet from context → u Zora API → stagetransaction. If Pinata fails at step 3, the entire coin launch fails beforeZora is ever called. Byation, you eliminate itas a suspect before testing the full launch flow.                          
Why you're uploading a JSON object: A Zora creator coin needs a metadata fidescribing it — name, snally an image URL. That metadata file lives on IPFS, not on Zora's servers, which means it's       permanent and can't be ataContent field isexactly what gets stored on IPFS. The pinataMetadata field is just a label for your Pinata dashboatored content and is notvisible to anyone else.                                                    
What IpfsHash actually is: IPFS (InterPlanetary File System) is a          content-addressed storad" means the address of a file is derived from its contents — specifically, a cryptographic hash of  the bytes. The Qm... yoontent Identifier) — afingerprint of your JSON. If you upload the exact same JSON again, you get the exact same hash. Thrthy for on-chainmetadata: once a coin is launched pointing at ipfs://QmYxV..., that metadatcan never be changed wiadata URI.
                                                                           Why -X POST: The PinataP POST requests. GETrequests are for retrieving content, not uploading it. Without -X POST, curdefaults to GET and Pin
                                                                           Bug signals and what th
                                                                           ┌───────────────────────────────────────────┐
│        What you see         │  What it means   │      What to do       │ ├───────────────────────────────────────────┤
│                             │ Your PINATA_JWT  │ Go to pinata.cloud →  │ │ {"error":"UNAUTHORIZEPI Keys → Create New │
│                             │ wrong            │  Key → copy the JWT   │ ├───────────────────────────────────────────┤
│ {"error":"GATEWAY_TIMEOUT"} │ Pinata's API is  │ Wait 30 seconds and   │ │                      ry again             │
├─────────────────────────────┼──────────────────┼───────────────────────┤ │                                           │
│ Empty output or curl: (6)   │  PINATA_JWT is   │ Check Step 1.1        │ │                                           │
└─────────────────────────────┴──────────────────┴───────────────────────┘ 
---                                                                        Step 1.4 — Verify the Zthe expected shape
                                                                           Run this in Terminal B:
echo "=== /coin endpoint (used by GetCreatorRecap) ===" && \               curl -s "https://api-sdss=0x493e88b9ba3a479c03c28af366adff4457d58d94&chain=8453" \                                           | python3 -c "
import sys, json                                                           t = json.load(sys.stdin
print('name:       ', t.get('name'))                                       print('symbol:     ', t
print('holders:    ', t.get('uniqueHolders'))                              print('marketCap:  ', t
print('volume24h:  ', t.get('volume24h'))                                  "
                                                                           Expected output:
=== /coin endpoint (used by GetCreatorRecap) ===                           name:        music
symbol:      music                                                         holders:     6
marketCap:   2839.56                                                       volume24h:   0.0
                                                                           echo "=== /coinHolders rboard andDistributeRewards) ===" && \                                               curl -s "https://api-sds?address=0x493e88b9ba3a479c03c28af366adff4457d58d94&chainId=8453&count=5" \                          | python3 -c "
import sys, json                                                           d = json.load(sys.stdin
edges = d.get('zora20Token', {}).get('tokenBalances', {}).get('edges', []) print(f'Total holders r
print()                                                                    for i, e in enumerate(e
    n = e['node']                                                              bal = float(n['bala
    handle = n.get('ownerProfile', {}).get('handle', 'no handle')              wallet = n.get('own
    short = wallet[:6] + '...' + wallet[-4:] if len(wallet) > 10 else walle    print(f'  #{i+1}  h {bal:>15,.2f} coinswallet: {short}')                                                          "
                                                                           Expected output:
=== /coinHolders endpoint (used by GetFanLeaderboard and DistributeRewards)===
Total holders returned: 3                                                  
  #1  handle: 0x4985...2b2b          balance: 991,109,807.32 coins  wallet:0x4985...2b2b
  #2  handle: skinnywhitegirl        balance:   8,244,103.54 coins  wallet:0x4cdf...bebd
  #3  handle: madbeets               balance:     361,460.81 coins  wallet:0xd204...20ad
                                                                           Save these numbers. You in Feature 2 and Feature 5.                                                                         
Why you're using a coin that isn't yours: You cannot test "will the Zora APreturn data for my coin instead you use a knownreal coin — the "music" coin at that address — to verify that your code's  parsing logic works on wo distinct things:first, that you can reach api-sdk.zora.engineering from your network; seconthat the Python parsingt parsing logic in yourplugin) correctly traverses zora20Token → tokenBalances → edges → node →   balance/ownerAddress/ow
                                                                           Why the balance parsingreturns balances as rawinteger strings, not as decimal numbers. The raw balance                   99110980732341069697290 — it is a large integerin the token's smallest unit (called "wei" for ETH-compatible chains). ERC-tokens have a decimals ns use 18 decimals. That means 1 full coin = 10^18 raw units. So 991109807323410696972909414 / 10^18= 991,109,807.32 readabof the total supply. Your Rust tool does this same division: balance_raw.parse::<f64>().map(|b| b /  1e18).
                                                                           Why the Python script ances.edges[].node: Thisis the actual JSON structure the Zora API returns. It is not obvious or    intuitive — you would eat the top level. ButZora uses GraphQL internally, and the response reflects that nested        structure. The path zorges → node is the exactsame path your Rust tools traverse. If this Python script fails on this patyour Rust tools will fa a bug in the parsing.
                                                                           Bug signals and what th
                                                                           ┌───────────────────┬──────────────────────┐
│   What you see    │     What it means      │        What to do        │  ├───────────────────┼──────────────────────┤
│ Empty output, no  │ The Python script      │ Run the curl alone first │  │ print statements  │ fout the pipe and    │
│                   │                        │ look at the raw JSON     │  ├───────────────────┼──────────────────────┤
│ KeyError:         │ The API changed its    │ Check the raw Zora       │  │ 'zora20Token'     │ rnse and update the  │
│                   │                        │ parsing path in tool.rs  │  ├───────────────────┼──────────────────────┤
│ All balances show │ The balance field is   │ Inspect the raw edge and │  │  0.0              │ nk the type of       │
│                   │  as float              │ node.balance             │  ├───────────────────┼──────────────────────┤
│                   │ The uniqueHolders      │ Check if it's now nested │  │ holders: None     │ ferently inside      │
│                   │  response              │ zora20Token              │  └───────────────────┴──────────────────────┘
                                                                           ---
Step 1.5 — Verify the Aomi API session works with your key                 
Run this in Terminal B:                                                    AOMI_KEY=$(grep '^AOMI_anforge/.env | cut -d=-f2-)                                                                      
aomi --api-key "$AOMI_KEY" --app default --new-session --prompt "hello" \    2>&1 | grep -v "Depre
                                                                           Expected output:
Hello! I'm Aomi, your execution assistant. I can help you check balances...Any coherent English res this step passes.
                                                                           Why this check is separ Telegram bot is a relay. When you send a message to @fan_forge_bot, it creates an Aomi Session objecand calls session.send( POST tohttps://api.aomi.dev. If that POST fails with 401, every single message in the Telegram bot will r— and you might spend 20minutes inspecting bot code when the real issue is a single character wrong   in your API key.
                                                                              By calling aomi directlsame API endpoint thatsession.ts calls, with the same key, but without Telegram or grammY involved. The --new-session flag  are not reusing anycached state. If this call works and the bot's message relay doesn't, the bug is definitively in the t work, the bug isdefinitively in your Aomi credentials.

Why --app default and not --app fanforge: The default app always exists — it
is Aomi's built-in genege app only exists afteryour PR is merged and activated. Testing with default confirms your API key
and network connection ge before activationwould give you a 401 or 404, which would be a false negative — it would look
like your key is brokensn't deployed yet.

Bug signals and what th

┌───────────────┬──────────────────────────┐
│ What you see  │  What it means  │             What to do              │
├───────────────┼──────────────────────────┤
│ Error: HTTP   │ Your            │                                     │
│ 401:          │ AOMI_ashboard and        │
│ Unauthorized  │  wrong or       │ regenerate your API key             │
│               │ expir                    │
├───────────────┼─────────────────┼─────────────────────────────────────┤
│               │ The d                    │
│ Error: HTTP   │  doesn't exist  │ Contact Aomi support — this         │
│ 404           │ for y                    │
│               │ account         │                                     │    ├───────────────┼──────────────────────────┤
│ Error: fetch  │ Network issue   │ Check that                          │    │ failed or     │ or   tps://api.aomi.dev  │
│ ECONNREFUSED  │ AOMI_BASE_URL   │ in your .env                        │    │               │ is wr                    │
├───────────────┼─────────────────┼─────────────────────────────────────┤    │ (no response) │ The A persists, check    │
│  after 30+    │ backend is slow │ Aomi's status page                  │    │ seconds       │  or d                    │
└───────────────┴─────────────────┴─────────────────────────────────────┘    
---                                                                          Part 2 — Bot Pipeline:
                                                                             ---
Step 2.1 — Start the bot                                                     
Run this in Terminal A:                                                      cd ~/Work/hackathons/fa
                                                                             Expected Terminal A out
> fanforge-bot@0.1.0 dev                                                     > tsx watch src/index.t
                                                                             FanForge bot started as
                                                                             What is actually happenyour TypeScript on thefly and starts the Node.js process. The bot immediately begins long-polling  Telegram's servers — thst tohttps://api.telegram.org/bot<token>/getUpdates every few seconds asking "are there any new messages message, it sends it back in the response. This is the polling mode in grammY. It is how the bot receives messages witho HTTPS endpoint.

FanForge bot started as this line in index.ts:
bot.start({
  onStart: (info) => coed as@${info.username}`),
});
This callback only fires after the first successful long-poll request to
Telegram confirms your is wrong, you never seethis line.

Leave Terminal A running and visible for the entire test session. Every
message, every error, ere. This is your mostimportant debugging tool.

Bug signals and what they mean:

What you see in Terminal A: Error: 401: Unauthorized before "started"
What it means: TELEGRAM
What to do: Go to Telegram → @BotFather → /mybots → select your bot → API
  Token
────────────────────────────────────────
What you see in Terminandlers/start.js'
What it means: TypeScript compilation failed
What to do: Run npm runee the error
────────────────────────────────────────
What you see in Terminaseconds
What it means: Process crashed silently
What to do: Scroll up i
────────────────────────────────────────
What you see in Termina@something_else
What it means: You're using a different bot's token
What to do: Check which

---
Step 2.2 — Test the /start command

In Telegram, send:
/start

Expected bot reply in T
Hey! I'm FanForge 🎵

I help music creators launch a fan economy — so your superfans can hold a
piece of your journey.

It takes about 60 secon

Just tell me what you'd
• "Launch a fan coin for my EP"
• "Show me who my top f
• "Reward fans who hold at least 100 coins"
• "Give me my weekly re

Expected Terminal A oute.

Why /start is always th is handled entirelyinside the bot code — it never calls Aomi, never calls Zora, never touches any external service. L.ts: it closes theexisting session, creates a new one, and calls ctx.reply(...) with hardcoded text. That's it. If /st:
1. TELEGRAM_BOT_TOKEN is valid — Telegram accepted your bot's identity    2. grammY is routing co
3. ctx.reply() can send messages back to the user                         4. The dotenv loading iloseSession andgetOrCreateSession both read process.env.* at runtime                     
If /start does not work, nothing else will. Fix this before testing anythielse.
                                                                          Why Terminal A shows no: The bot's loggingstrategy is: only log errors. A successful operation has no side effects  worth logging. If you dring /start, it means anerror was caught and logged — read it carefully.                          
Bug signal: If Telegram shows "Something went wrong on my end. Give it a  moment and try again." from the catch block inthe message handler, and /start has its own separate command handler that doesn't have that catchr /start, something veryunusual happened. Check Terminal A immediately.

---                                                               Step 2.3 — Test the bas
                                                                  In Telegram, send:
hello                                                             
Expected bot reply: Any coherent response from the Aomi LLM. It mi"Hello! I'm here to hel." or something similar.The exact words don't matter.                                     
Expected Terminal A output: Silence.                              
Why this test matters as its own step: "hello" goes through the furelay pipeline: grammY ("message:text") fires →getOrCreateSession(userId) creates or retrieves an Aomi session → session.send("hello") s Aomi processes it →returns result.messages → your extractText() function extracts the content →
sendChunked() sends it e component of the relayis exercised by this one message.

The fact that the response is generic and unhelpful ("I'm here to help with EVM actions") is not a r becauseAOMI_APP=default. The default app is Aomi's general EVM assistant; it knows nothing about FanForge.is test is that thepipeline from Telegram through your Node.js bot to Aomi and back is fully
functional.

If this test fails but specifically in thesession.send() → response handling path. Look at Terminal A. The line
starting with Aomi sessexact error. Commoncauses:
- HTTP 401 → AOMI_API_Kme even though it's in.env (this would be the dotenv hoisting bug we fixed, but may have regressed)
- Cannot read propertieer') → result.messagescame back as undefined, meaning the Aomi response had an unexpected shape
- TypeError: fetch failue

---
Part 3 — Feature Tests: What You See Now vs. After Activation

This section tests all five FanForge features. For each feature you will see
two columns: what the bdefault, and what itshould reply after activation with AOMI_APP=fanforge. Knowing what "now"
looks like is as importooks like — it lets youimmediately tell whether the activation happened or not.

---
Feature 1 — Coin Launch

In Telegram, send:
I want to launch a fan coin called Temi Coin with the ticker TEMI. It's for
my fans who want to sup

NOW (AOMI_APP=default)
The bot will say something like:

▎ "Launching your fan coin Temi Coin ($TEMI) on Base is a great idea. Based on current best practnch an AI-driven fan coin is through Clanker. To proceed, please connect your wallet..."

The response is generic. It suggests Clanker (a different token launcher
entirely). It asks you e way. It does not return a Zora link. No Rust code runs. No Pinata upload happens. No Zora API call
is made. No Supabase wr

Terminal A for this teslt app handled everything on Aomi's si — your plugin was never loaded.

AFTER activation (AOMI_APP=fanforge) — expected reply:
The LLM will first conf

▎ "Let me confirm befor
▎ • Name: Temi Coin
▎ • Ticker: $TEMI
▎ • Description: for fans who want to support my music journey
▎
▎ Ready to deploy?"

After your confirmation, it triggers the wallet approval flow. Your Aomi-connected wallet wrove. Once you approveit:

▎ "Your Temi Coin ($TEMI) is live! Share this with your fans:
▎ https://zora.co/coin/

Terminal A after act if anything goeswrong in the Rust code, you will see lines like:                             Aomi session error: [fa ...
Aomi session error: [fanforge] Zora POST /create/content returned 400: ...   Aomi session error: [fa your fan economy walletisn't linked yet                                                             
Verify the launch actually happened — run in Terminal B:                     source ~/Work/hackathon
curl -s "$SUPABASE_URL/rest/v1/creator_coins?select=ticker,coin_address,zora_url,creed_at" \
  -H "apikey: $SUPABASE_ANON_KEY" \
  -H "Authorization: Be`

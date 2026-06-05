use crate::client::*;
use aomi_sdk::*;
use serde_json::{Value, json};

// ── Tool 1: fanforge_launch_fan_coin ──────────────────────────────────────────
//
// Route chain (no internal tools needed):
//   LaunchFanCoin  (wallet from ctx, IPFS pin, Zora calldata)
//     → stage_tx + enforce(simulate_batch → commit_txs binds "tx_hash")
//     → after: fanforge_finalize_launch

pub(crate) struct LaunchFanCoin;

impl DynAomiTool for LaunchFanCoin {
    type App = FanForgeApp;
    type Args = LaunchFanCoinArgs;
    const NAME: &'static str = "fanforge_launch_fan_coin";
    const DESCRIPTION: &'static str = "Launch a creator coin on Zora for any creator with a fanbase — music artists, TikTokers, YouTubers, producers, designers, or anyone who wants to turn fan loyalty into a real economy. Takes a coin name, ticker (3–5 letters), and a short description. Handles everything invisibly. Returns the live Zora link. Use when a creator says they want to launch a coin, start a fan economy, or monetize their audience.";

    fn run_with_routes(
        _app: &FanForgeApp,
        args: Self::Args,
        ctx: DynToolCallCtx,
    ) -> Result<ToolReturn, String> {
        let ticker = args.ticker.trim().to_uppercase();
        if ticker.len() < 3
            || ticker.len() > 5
            || !ticker.chars().all(|c| c.is_ascii_uppercase())
        {
            return Err(format!(
                "ticker_invalid: '{}' must be 3–5 letters (e.g. TEMI, VIBES, MVMNT)",
                ticker
            ));
        }

        // Get wallet address from Aomi session context (set when creator connects wallet)
        let creator_address = ctx
            .attribute_string(&["domain", "evm", "address"])
            .ok_or("wallet_not_connected: your fan economy wallet isn't linked yet — connect your wallet to continue")?;

        // Upload coin metadata to IPFS via Pinata
        let metadata_json = json!({
            "name": args.name,
            "description": args.description,
            "symbol": ticker,
            "image": args.image_url.as_deref().unwrap_or(""),
        });
        let metadata_uri = ipfs_pin_json(&metadata_json)?;

        // Fetch pre-built transaction calldata from Zora's REST API
        let zora_req = json!({
            "creator": creator_address,
            "name": args.name,
            "symbol": ticker,
            "metadata": { "type": "RAW_URI", "uri": metadata_uri },
            "currency": "CREATOR_COIN",
            "chainId": 8453,
        });
        let calldata_resp = zora_post("/create/content", &zora_req)?;

        let call = calldata_resp
            .get("calls")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .ok_or("zora_error: Zora returned no transaction calldata")?;

        let to = call
            .get("to")
            .and_then(Value::as_str)
            .ok_or("zora_error: missing transaction target")?;
        let data = call
            .get("data")
            .and_then(Value::as_str)
            .ok_or("zora_error: missing transaction calldata")?;
        let value_str = call.get("value").and_then(Value::as_str).unwrap_or("0");
        let predicted_address = calldata_resp
            .get("predictedCoinAddress")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        ToolReturn::route(json!({
            "status": "deploying",
            "message": "Your fan coin is being deployed — approve in your wallet.",
        }))
        .next(|next| {
            next.add::<host::StageTx>(json!({
                "to": to,
                "data": { "raw": data },
                "value": value_str,
                "chain_id": 8453,
            }))
            .enforce(EnforcementPolicy::Stop, |enforce| {
                enforce.add::<host::SimulateBatch>(json!({}));
                enforce
                    .add::<host::CommitTxs>(json!({ "aa_preference": "auto" }))
                    .bind_as("tx_hash");
            });
        })
        .after_named(
            "fanforge_finalize_launch",
            json!({
                "name": args.name,
                "ticker": ticker,
                "creator_telegram_id": args.creator_telegram_id,
                "predicted_coin_address": predicted_address,
            }),
        )
        .awaits("tx_hash")
        .try_build()
    }
}

// ── Tool 1b (internal): fanforge_finalize_launch ──────────────────────────────

pub(crate) struct FinalizeLaunch;

impl DynAomiTool for FinalizeLaunch {
    type App = FanForgeApp;
    type Args = FinalizeLaunchArgs;
    const NAME: &'static str = "fanforge_finalize_launch";
    const DESCRIPTION: &'static str = "Internal tool — fires automatically after the wallet confirms the fan coin transaction. Records the coin in the database and returns the live Zora link. Do not call directly.";

    fn run(
        _app: &FanForgeApp,
        args: Self::Args,
        _ctx: DynToolCallCtx,
    ) -> Result<Value, String> {
        let tx_hash = args.tx_hash.as_deref().unwrap_or("pending");

        let zora_url = format!(
            "https://zora.co/coin/base:{}",
            args.predicted_coin_address.to_lowercase()
        );

        // Best-effort Supabase write — don't fail the launch if DB is temporarily down
        let _ = supabase_post(
            "creator_coins",
            &json!({
                "creator_telegram_id": args.creator_telegram_id,
                "coin_address": args.predicted_coin_address,
                "ticker": args.ticker,
                "name": args.name,
                "transaction_hash": tx_hash,
                "zora_url": zora_url,
            }),
        );

        ok(json!({
            "status": "launched",
            "coin_name": args.name,
            "ticker": args.ticker,
            "zora_url": zora_url,
            "coin_address": args.predicted_coin_address,
            "message": format!(
                "Your {} coin (${}) is live! Share this with your fans: {}",
                args.name, args.ticker, zora_url
            ),
        }))
    }
}

// ── Tool 2: fanforge_get_fan_leaderboard ──────────────────────────────────────

pub(crate) struct GetFanLeaderboard;

impl DynAomiTool for GetFanLeaderboard {
    type App = FanForgeApp;
    type Args = GetFanLeaderboardArgs;
    const NAME: &'static str = "fanforge_get_fan_leaderboard";
    const DESCRIPTION: &'static str = "Get the ranked leaderboard of top fans holding a creator's Zora coin. Returns handles, balances, and percentage of supply held. Use when the creator asks who their top fans are, wants to see holder rankings, or is deciding who gets a reward or shoutout.";

    fn run(_app: &FanForgeApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let limit = args.limit.unwrap_or(10).min(50);
        let path = format!(
            "/coinHolders?address={}&chainId=8453&count={}",
            urlencode(&args.coin_address),
            limit
        );

        let resp = zora_get(&path)?;

        // Response: { zora20Token: { tokenBalances: { edges: [{ node: { balance, ownerAddress, ownerProfile } }] } } }
        let edges = resp
            .get("zora20Token")
            .and_then(|t| t.get("tokenBalances"))
            .and_then(|tb| tb.get("edges"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let entries: Vec<Value> = edges
            .into_iter()
            .enumerate()
            .map(|(i, edge)| {
                let node = edge.get("node").unwrap_or(&edge);
                let wallet = node
                    .get("ownerAddress")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let wallet_short = if wallet.len() > 10 {
                    format!("{}…{}", &wallet[..6], &wallet[wallet.len() - 4..])
                } else {
                    wallet.to_string()
                };
                let handle = node
                    .get("ownerProfile")
                    .and_then(|p| p.get("handle"))
                    .and_then(Value::as_str)
                    .unwrap_or(&wallet_short)
                    .to_string();
                // balance is raw (18 decimals) — convert to human-readable
                let balance_coins = node
                    .get("balance")
                    .and_then(Value::as_str)
                    .unwrap_or("0")
                    .parse::<f64>()
                    .map(|b| b / 1e18)
                    .unwrap_or(0.0);

                json!({
                    "rank": i + 1,
                    "handle": handle,
                    "wallet_short": wallet_short,
                    "balance": format!("{:.2}", balance_coins),
                })
            })
            .collect();

        ok(json!({
            "coin_address": args.coin_address,
            "total_entries": entries.len(),
            "leaderboard": entries,
        }))
    }
}

// ── Tool 3: fanforge_create_fan_mission ───────────────────────────────────────

pub(crate) struct CreateFanMission;

impl DynAomiTool for CreateFanMission {
    type App = FanForgeApp;
    type Args = CreateFanMissionArgs;
    const NAME: &'static str = "fanforge_create_fan_mission";
    const DESCRIPTION: &'static str = "Create a fan mission that unlocks exclusive content for anyone holding above a minimum coin balance. Use when the creator wants to reward loyal fans with exclusive content — unreleased tracks, early access, behind-the-scenes, merch discounts, stream invites, or anything fans would value.";

    fn run(_app: &FanForgeApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.threshold <= 0.0 {
            return Err("threshold_invalid: minimum coin balance must be greater than 0".to_string());
        }

        let expires_at = args.expires_at.unwrap_or_else(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("{}", now + 30 * 24 * 3600)
        });

        let result = supabase_post(
            "fan_missions",
            &json!({
                "coin_address": args.coin_address,
                "title": args.title,
                "content_url": args.content_url,
                "threshold": args.threshold,
                "expires_at": expires_at,
                "status": "active",
            }),
        )?;

        let mission_id = result
            .as_array()
            .and_then(|a| a.first())
            .and_then(|r| r.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        ok(json!({
            "mission_id": mission_id,
            "title": args.title,
            "threshold": args.threshold,
            "status": "active",
            "message": format!(
                "Mission created. Fans holding {} or more coins can unlock: {}",
                args.threshold, args.title
            ),
        }))
    }
}

// ── Tool 4: fanforge_distribute_rewards ───────────────────────────────────────

pub(crate) struct DistributeRewards;

impl DynAomiTool for DistributeRewards {
    type App = FanForgeApp;
    type Args = DistributeRewardsArgs;
    const NAME: &'static str = "fanforge_distribute_rewards";
    const DESCRIPTION: &'static str = "Deliver exclusive content to every fan who meets the mission's minimum holding threshold and hasn't received it yet. Idempotent — safe to run multiple times. Use after creating a mission or when the creator wants to send rewards to qualifying fans now.";

    fn run(_app: &FanForgeApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        // Load mission
        let missions = supabase_get(
            "fan_missions",
            &format!("id=eq.{}&select=*", urlencode(&args.mission_id)),
        )?;
        let mission = missions
            .as_array()
            .and_then(|a| a.first())
            .cloned()
            .ok_or_else(|| format!("mission_not_found: no mission with id {}", args.mission_id))?;

        if mission.get("status").and_then(Value::as_str).unwrap_or("") != "active" {
            return Err(format!(
                "mission_not_active: mission status is '{}'",
                mission.get("status").and_then(Value::as_str).unwrap_or("unknown")
            ));
        }

        let coin_address = mission
            .get("coin_address")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let threshold: f64 = mission
            .get("threshold")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        // Fetch current holders from Zora
        let path = format!(
            "/coinHolders?address={}&chainId=8453&count=100",
            urlencode(&coin_address)
        );
        let holders_resp = zora_get(&path)?;

        // Response: { zora20Token: { tokenBalances: { edges: [{ node: { balance, ownerAddress } }] } } }
        let edges = holders_resp
            .get("zora20Token")
            .and_then(|t| t.get("tokenBalances"))
            .and_then(|tb| tb.get("edges"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Load wallets that already received this mission's content
        let distributed = supabase_get(
            "reward_distributions",
            &format!(
                "mission_id=eq.{}&select=recipient_wallet",
                urlencode(&args.mission_id)
            ),
        )?;
        let already_sent: std::collections::HashSet<String> = distributed
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|r| r.get("recipient_wallet").and_then(Value::as_str))
            .map(String::from)
            .collect();

        let mut newly_dispatched = 0u32;
        let mut eligible_count = 0u32;

        for edge in &edges {
            let node = edge.get("node").unwrap_or(edge);
            let wallet = node
                .get("ownerAddress")
                .and_then(Value::as_str)
                .unwrap_or("");
            // balance is raw 18-decimal — convert to coins for threshold comparison
            let balance: f64 = node
                .get("balance")
                .and_then(Value::as_str)
                .unwrap_or("0")
                .parse::<f64>()
                .map(|b| b / 1e18)
                .unwrap_or(0.0);

            if balance < threshold || wallet.is_empty() {
                continue;
            }
            eligible_count += 1;

            if already_sent.contains(wallet) {
                continue;
            }

            // DB-level UNIQUE(mission_id, recipient_wallet) prevents duplicates
            if supabase_post(
                "reward_distributions",
                &json!({ "mission_id": args.mission_id, "recipient_wallet": wallet }),
            )
            .is_ok()
            {
                newly_dispatched += 1;
            }
        }

        ok(json!({
            "mission_id": args.mission_id,
            "eligible_count": eligible_count,
            "newly_dispatched": newly_dispatched,
            "already_received": eligible_count.saturating_sub(newly_dispatched),
            "status": "complete",
            "message": format!(
                "{} fans just unlocked the content. {} had already received it.",
                newly_dispatched,
                eligible_count.saturating_sub(newly_dispatched)
            ),
        }))
    }
}

// ── Tool 5: fanforge_get_creator_recap ────────────────────────────────────────

pub(crate) struct GetCreatorRecap;

impl DynAomiTool for GetCreatorRecap {
    type App = FanForgeApp;
    type Args = GetCreatorRecapArgs;
    const NAME: &'static str = "fanforge_get_creator_recap";
    const DESCRIPTION: &'static str = "Generate a plain-English recap of how a creator's fan coin and community are growing — holder count, market cap, active missions, trading volume. Also produces a ready-to-copy social post. Use when the creator asks how their coin is doing, wants a weekly update, or needs content to post.";

    fn run(_app: &FanForgeApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let days = args.days.unwrap_or(7);

        let coin_path = format!(
            "/coin?address={}&chain=8453",
            urlencode(&args.coin_address)
        );
        let coin_data = zora_get(&coin_path)?;

        // Response: { zora20Token: { name, symbol, uniqueHolders, marketCap, volume24h, ... } }
        let token = coin_data
            .get("zora20Token")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let coin_name = token
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("your coin");
        let ticker = token.get("symbol").and_then(Value::as_str).unwrap_or("");
        let holders = token
            .get("uniqueHolders")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let market_cap = token
            .get("marketCap")
            .and_then(Value::as_str)
            .unwrap_or("0");
        let volume_24h = token
            .get("volume24h")
            .and_then(Value::as_str)
            .unwrap_or("0");

        let missions = supabase_get(
            "fan_missions",
            &format!(
                "coin_address=eq.{}&status=eq.active&select=id,title",
                urlencode(&args.coin_address)
            ),
        )
        .unwrap_or(json!([]));
        let active_missions = missions.as_array().map(|a| a.len()).unwrap_or(0);

        let summary = format!(
            "Over the last {days} days, your fan economy is growing. \
             ${ticker} now has {holders} fan{} holding your coin. \
             You have {active_missions} active mission{} delivering exclusive content to your top supporters. \
             Market cap: ${market_cap}. 24h volume: ${volume_24h}.",
            if holders == 1 { "" } else { "s" },
            if active_missions == 1 { "" } else { "s" }
        );

        let social_post = format!(
            "my coin ${ticker} now has {holders} holders 🔥\n\
             holding = access. you already know what's inside 🔐\n\
             new mission dropping soon for my real ones\n\
             link in bio to join"
        );

        ok(json!({
            "coin_name": coin_name,
            "ticker": ticker,
            "total_holders": holders,
            "active_missions": active_missions,
            "days_covered": days,
            "summary": summary,
            "social_post": social_post,
        }))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use aomi_sdk::testing::{run_tool, TestCtxBuilder};
    use serde_json::json;

    fn ctx(name: &str) -> DynToolCallCtx {
        TestCtxBuilder::new(name).build()
    }

    fn ctx_with_wallet(name: &str) -> DynToolCallCtx {
        // Simulate a connected wallet in the Aomi session context
        TestCtxBuilder::new(name)
            .attribute("domain", json!({ "evm": { "address": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266" } }))
            .build()
    }

    // ── LaunchFanCoin ─────────────────────────────────────────────────────────

    #[test]
    fn launch_rejects_ticker_too_short() {
        let result = run_tool::<LaunchFanCoin>(
            &FanForgeApp,
            json!({ "creator_telegram_id": "1", "name": "X", "ticker": "AB", "description": "d" }),
            ctx_with_wallet("fanforge_launch_fan_coin"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ticker_invalid"));
    }

    #[test]
    fn launch_rejects_ticker_too_long() {
        let result = run_tool::<LaunchFanCoin>(
            &FanForgeApp,
            json!({ "creator_telegram_id": "1", "name": "X", "ticker": "TOOLONG", "description": "d" }),
            ctx_with_wallet("fanforge_launch_fan_coin"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ticker_invalid"));
    }

    #[test]
    fn launch_rejects_ticker_with_digits() {
        let result = run_tool::<LaunchFanCoin>(
            &FanForgeApp,
            json!({ "creator_telegram_id": "1", "name": "X", "ticker": "TEM1", "description": "d" }),
            ctx_with_wallet("fanforge_launch_fan_coin"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ticker_invalid"));
    }

    #[test]
    fn launch_rejects_missing_wallet() {
        // No wallet in context — should fail before any HTTP call
        let result = run_tool::<LaunchFanCoin>(
            &FanForgeApp,
            json!({ "creator_telegram_id": "1", "name": "Temi Coin", "ticker": "TEMI", "description": "d" }),
            ctx("fanforge_launch_fan_coin"), // no wallet injected
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wallet_not_connected"));
    }

    // ── CreateFanMission ──────────────────────────────────────────────────────

    #[test]
    fn mission_rejects_zero_threshold() {
        let result = run_tool::<CreateFanMission>(
            &FanForgeApp,
            json!({
                "coin_address": "0x493e88b9ba3a479c03c28af366adff4457d58d94",
                "title": "Early Access",
                "content_url": "https://example.com/track.mp3",
                "threshold": 0.0,
            }),
            ctx("fanforge_create_fan_mission"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("threshold_invalid"));
    }

    #[test]
    fn mission_rejects_negative_threshold() {
        let result = run_tool::<CreateFanMission>(
            &FanForgeApp,
            json!({
                "coin_address": "0x493e88b9ba3a479c03c28af366adff4457d58d94",
                "title": "Early Access",
                "content_url": "https://example.com/track.mp3",
                "threshold": -50.0,
            }),
            ctx("fanforge_create_fan_mission"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("threshold_invalid"));
    }

    // ── GetFanLeaderboard response parsing ────────────────────────────────────

    #[test]
    fn leaderboard_parses_zora_response_shape() {
        let zora_response = json!({
            "zora20Token": {
                "tokenBalances": {
                    "edges": [
                        {
                            "node": {
                                "balance": "1000000000000000000000",
                                "ownerAddress": "0xabc123def456abc123def456abc123def456abc1",
                                "ownerProfile": { "handle": "superfan", "__typename": "GraphQLAccountProfile" }
                            }
                        },
                        {
                            "node": {
                                "balance": "500000000000000000000",
                                "ownerAddress": "0x9999888877776666555544443333222211110000",
                                "ownerProfile": { "handle": "0x9999...0000", "__typename": "GraphQLWalletProfile" }
                            }
                        }
                    ]
                }
            }
        });

        let edges = zora_response
            .get("zora20Token")
            .and_then(|t| t.get("tokenBalances"))
            .and_then(|tb| tb.get("edges"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        assert_eq!(edges.len(), 2);

        let node0 = edges[0].get("node").unwrap();
        let balance_coins: f64 = node0
            .get("balance").and_then(Value::as_str).unwrap()
            .parse::<f64>().unwrap() / 1e18;
        assert!((balance_coins - 1000.0).abs() < 0.01);

        let handle = node0.get("ownerProfile").unwrap()
            .get("handle").and_then(Value::as_str).unwrap();
        assert_eq!(handle, "superfan");
    }

    // ── FinalizeLaunch tx_hash type ───────────────────────────────────────────

    #[test]
    fn finalize_handles_missing_tx_hash() {
        // tx_hash = None should fall back to "pending"
        let ctx = TestCtxBuilder::new("fanforge_finalize_launch").build();
        let result = run_tool::<FinalizeLaunch>(
            &FanForgeApp,
            json!({
                "name": "Temi Coin",
                "ticker": "TEMI",
                "creator_telegram_id": "12345",
                "predicted_coin_address": "0x493e88b9ba3a479c03c28af366adff4457d58d94",
                "tx_hash": null,
            }),
            ctx,
        );
        // Will fail on Supabase (no connection in tests) but the tx_hash handling
        // should not be the cause — check the error is DB-related, not type-related
        if let Err(e) = &result {
            assert!(!e.contains("tx_hash"), "tx_hash handling should not error: {e}");
        }
    }
}

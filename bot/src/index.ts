import { config } from "dotenv";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
config({ path: resolve(dirname(fileURLToPath(import.meta.url)), "../../.env") });

import { Bot, type Context } from "grammy";
import { handleStart } from "./handlers/start.js";
import { getOrCreateSession } from "./session.js";

const token = process.env.TELEGRAM_BOT_TOKEN;
if (!token) {
  console.error("TELEGRAM_BOT_TOKEN is required");
  process.exit(1);
}

const bot = new Bot(token);

// ── Command handlers ──────────────────────────────────────────────────────────

bot.command("start", handleStart);
bot.command("help", handleStart);

// ── Message handler — relay every non-command message through Aomi ────────────

bot.on("message:text", async (ctx) => {
  const userId = ctx.from?.id;
  if (!userId) return;

  const text = ctx.message.text;

  await ctx.api.sendChatAction(ctx.chat.id, "typing");

  try {
    const session = getOrCreateSession(userId);
    const result = await session.send(text);

    const agentMessages = result.messages.filter((m) => m.sender === "agent");

    // Find the last agent message with non-empty content (intermediate messages
    // often have empty content while the model is still generating)
    let content = "";
    for (let i = agentMessages.length - 1; i >= 0; i--) {
      const candidate = extractText(agentMessages[i].content);
      if (candidate.length > 0) {
        content = candidate;
        break;
      }
    }

    if (!content) {
      await ctx.reply("Still thinking… try again in a moment.");
      return;
    }

    await sendChunked(ctx, content);
  } catch (err) {
    console.error("Aomi session error:", err);
    await ctx.reply(
      "Something went wrong on my end. Give it a moment and try again.",
    );
  }
});

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Extract a human-readable string from whatever Aomi sends back.
 * Tool results arrive as JSON objects with a "message" field or as plain strings.
 */
function extractText(content: unknown): string {
  if (typeof content === "string") {
    // Strip wrapping JSON if the agent emitted a raw tool result string
    try {
      const parsed = JSON.parse(content);
      return extractText(parsed);
    } catch {
      return content.trim();
    }
  }

  if (Array.isArray(content)) {
    // Multi-part content — join text parts
    return content
      .map((part) => {
        if (typeof part === "object" && part !== null && "text" in part) {
          return String((part as { text: unknown }).text);
        }
        return extractText(part);
      })
      .join("\n")
      .trim();
  }

  if (typeof content === "object" && content !== null) {
    const obj = content as Record<string, unknown>;
    // Prefer a human-readable "message" field if the plugin returned one
    if (typeof obj.message === "string") return obj.message.trim();
    if (typeof obj.text === "string") return obj.text.trim();
    if (typeof obj.summary === "string") return obj.summary.trim();
    // Fall back to pretty-printing — but strip the internal "source" field
    const { source: _source, ...rest } = obj;
    return JSON.stringify(rest, null, 2);
  }

  return String(content);
}

/**
 * Send a message, splitting at 4000 chars if needed.
 * Falls back to plain text if Markdown parse fails.
 */
async function sendChunked(ctx: Context, text: string): Promise<void> {
  const chunks = chunkText(text, 4000);
  for (const chunk of chunks) {
    try {
      await ctx.reply(chunk, { parse_mode: "Markdown" });
    } catch {
      // Markdown parse error — retry as plain text
      await ctx.reply(chunk);
    }
  }
}

function chunkText(text: string, maxLen: number): string[] {
  if (text.length <= maxLen) return [text];
  const chunks: string[] = [];
  let remaining = text;
  while (remaining.length > maxLen) {
    const nlPos = remaining.lastIndexOf("\n", maxLen);
    const breakAt = nlPos > 0 ? nlPos : maxLen;
    chunks.push(remaining.slice(0, breakAt).trim());
    remaining = remaining.slice(breakAt).trim();
  }
  if (remaining) chunks.push(remaining);
  return chunks;
}

// ── Error handling ────────────────────────────────────────────────────────────

bot.catch((err) => {
  console.error("Bot error:", err.error);
});

// ── Start ─────────────────────────────────────────────────────────────────────

bot.start({
  onStart: (info) => console.log(`FanForge bot started as @${info.username}`),
});

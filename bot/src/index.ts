import "dotenv/config";
import { Bot } from "grammy";
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

  // Show a typing indicator while Aomi processes
  await ctx.api.sendChatAction(ctx.chat.id, "typing");

  try {
    const session = getOrCreateSession(userId);
    const result = await session.send(text);

    // Aomi returns an array of message objects; surface the last assistant message
    const assistantMessages = result.messages.filter(
      (m) => m.sender === "agent",
    );

    if (assistantMessages.length === 0) {
      await ctx.reply("Still thinking... try again in a moment.");
      return;
    }

    const lastMessage = assistantMessages[assistantMessages.length - 1];
    const content =
      typeof lastMessage.content === "string"
        ? lastMessage.content
        : JSON.stringify(lastMessage.content, null, 2);

    // Telegram message limit is 4096 chars; split if needed
    if (content.length <= 4096) {
      await ctx.reply(content, { parse_mode: "Markdown" });
    } else {
      const chunks = content.match(/.{1,4000}/gs) ?? [content];
      for (const chunk of chunks) {
        await ctx.reply(chunk);
      }
    }
  } catch (err) {
    console.error("Aomi session error:", err);
    await ctx.reply(
      "Something went wrong on my end. Give it a moment and try again.",
    );
  }
});

// ── Error handling ────────────────────────────────────────────────────────────

bot.catch((err) => {
  console.error("Bot error:", err.error);
});

// ── Start ─────────────────────────────────────────────────────────────────────

bot.start({
  onStart: (info) => console.log(`FanForge bot started as @${info.username}`),
});

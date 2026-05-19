import type { Context } from "grammy";
import { closeSession, getOrCreateSession } from "../session.js";

export async function handleStart(ctx: Context): Promise<void> {
  const userId = ctx.from?.id;
  if (!userId) return;

  // Reset any existing session so the creator starts fresh
  closeSession(userId);
  getOrCreateSession(userId);

  await ctx.reply(
    `Hey! I'm FanForge 🎵\n\n` +
      `I help music creators launch a fan economy — so your superfans can hold a piece of your journey.\n\n` +
      `It takes about 60 seconds to set up. Ready?\n\n` +
      `Just tell me what you'd like to do:\n` +
      `• "Launch a fan coin for my EP"\n` +
      `• "Show me who my top fans are"\n` +
      `• "Reward fans who hold at least 100 coins"\n` +
      `• "Give me my weekly recap"`,
    { parse_mode: "Markdown" },
  );
}

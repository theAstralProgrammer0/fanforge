import { Session } from "@aomi-labs/client";

const AOMI_BASE_URL = process.env.AOMI_BASE_URL ?? "https://api.aomi.dev";
const AOMI_APP = process.env.AOMI_APP ?? "fanforge";

// One Aomi session per Telegram user, keyed by their Telegram user ID.
const sessions = new Map<number, Session>();

export function getOrCreateSession(telegramUserId: number): Session {
  const existing = sessions.get(telegramUserId);
  if (existing) return existing;

  const session = new Session(
    { baseUrl: AOMI_BASE_URL },
    {
      sessionId: `tg-${telegramUserId}`,
      app: AOMI_APP,
      apiKey: process.env.AOMI_API_KEY,
    },
  );

  sessions.set(telegramUserId, session);
  return session;
}

export function closeSession(telegramUserId: number): void {
  const session = sessions.get(telegramUserId);
  if (session) {
    session.close();
    sessions.delete(telegramUserId);
  }
}

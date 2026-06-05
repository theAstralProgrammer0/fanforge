import { Session } from "@aomi-labs/client";


// One Aomi session per Telegram user, keyed by their Telegram user ID.
const sessions = new Map<number, Session>();

export function getOrCreateSession(telegramUserId: number): Session {
  const existing = sessions.get(telegramUserId);
  if (existing) return existing;

  const baseUrl = process.env.AOMI_BASE_URL ?? "https://api.aomi.dev";
  const app = process.env.AOMI_APP ?? "fanforge";
  const sessionId = crypto.randomUUID();
  const session = new Session(
    { baseUrl },
    {
      sessionId,
      app,
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

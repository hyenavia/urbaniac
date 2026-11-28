import { DiscordSDK } from "@discord/embedded-app-sdk";

export interface AuthSession {
  mode: "mock" | "live";
  accessToken: string;
}

let cachedSdk: DiscordSDK | null = null;
let cachedClientId: string | null = null;

export function getDiscordSdk(clientId: string): DiscordSDK {
  if (!cachedSdk || cachedClientId !== clientId) {
    cachedSdk = new DiscordSDK(clientId);
    cachedClientId = clientId;
  }
  return cachedSdk;
}

export function formatError(err: unknown, fallback: string): string {
  if (err instanceof Error) {
    return err.message.trim() || fallback;
  }
  if (typeof err === "string") {
    return err.trim() || fallback;
  }
  if (typeof err === "object" && err !== null) {
    const obj = err as { code?: unknown; message?: unknown };
    const msg = typeof obj.message === "string" ? obj.message.trim() : "";
    const code =
      obj.code !== undefined && obj.code !== null ? String(obj.code).trim() : "";
    if (msg && code) return `${msg} (${code})`;
    if (msg) return msg;
    if (code) return `Error (${code})`;
  }
  return fallback;
}

async function resolveLiveClientId(envClientId: string): Promise<string> {
  if (envClientId) return envClientId;
  try {
    const res = await fetch("/api/config");
    if (res.ok) {
      const body = (await res.json()) as { discord_client_id?: string };
      if (body.discord_client_id?.trim()) {
        return body.discord_client_id.trim();
      }
    }
  } catch {
    // Ignore; handled below if client ID remains empty.
  }
  return "";
}

export async function initDiscordAuth(): Promise<AuthSession> {
  const params = new URLSearchParams(window.location.search);
  const inDiscordFrame = params.has("frame_id");
  const envClientId = (import.meta.env.VITE_DISCORD_CLIENT_ID ?? "").trim();
  const mockFlag = import.meta.env.VITE_USE_MOCK_API;

  const useMock =
    mockFlag === "true" || (!inDiscordFrame && mockFlag !== "false");

  if (useMock) {
    // When opened inside Discord via tunnel during local dev,
    // still call ready() if Client ID is available so Discord's loading screen clears.
    if (inDiscordFrame && envClientId) {
      const sdk = getDiscordSdk(envClientId);
      await sdk.ready();
    }
    return { mode: "mock", accessToken: "mock-token" };
  }

  if (!inDiscordFrame) {
    throw new Error(
      "Live mode requires a Discord Activity iframe (frame_id). Set VITE_USE_MOCK_API=true to test in a regular browser.",
    );
  }

  const clientId = await resolveLiveClientId(envClientId);
  if (!clientId) {
    throw new Error(
      "DISCORD_CLIENT_ID / VITE_DISCORD_CLIENT_ID is not configured.",
    );
  }

  const discordSdk = getDiscordSdk(clientId);
  await discordSdk.ready();

  const { code } = await discordSdk.commands.authorize({
    client_id: clientId,
    response_type: "code",
    state: "",
    prompt: "none",
    scope: ["identify"],
  });

  const tokenRes = await fetch("/api/token", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code }),
  });

  if (!tokenRes.ok) {
    let msg = `Failed to exchange Discord token (HTTP ${tokenRes.status})`;
    try {
      const body = (await tokenRes.json()) as { error?: { message?: string } };
      if (body.error?.message) msg = body.error.message;
    } catch {
      // Ignore non-JSON error body
    }
    throw new Error(msg);
  }

  const { access_token } = (await tokenRes.json()) as { access_token: string };
  await discordSdk.commands.authenticate({ access_token });

  return { mode: "live", accessToken: access_token };
}

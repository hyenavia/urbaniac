import { DiscordSDK } from "@discord/embedded-app-sdk";

export interface AuthSession {
  mode: "mock" | "live";
  accessToken: string;
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
    // Abaikan, akan ditangani pesan error di bawah jika tetap kosong
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
    // Jika dibuka di dalam Discord lewat Cloudflare Tunnel saat dev PC,
    // tetap panggil ready() bila Client ID tersedia agar loading screen Discord selesai.
    if (inDiscordFrame && envClientId) {
      const sdk = new DiscordSDK(envClientId);
      await sdk.ready();
    }
    return { mode: "mock", accessToken: "mock-token" };
  }

  if (!inDiscordFrame) {
    throw new Error(
      "Mode live membutuhkan Discord Activity iframe (frame_id). Aktifkan VITE_USE_MOCK_API=true untuk uji di browser biasa.",
    );
  }

  const clientId = await resolveLiveClientId(envClientId);
  if (!clientId) {
    throw new Error(
      "DISCORD_CLIENT_ID / VITE_DISCORD_CLIENT_ID belum diisi di konfigurasi.",
    );
  }

  const discordSdk = new DiscordSDK(clientId);
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
    let msg = `Gagal menukar token Discord (HTTP ${tokenRes.status})`;
    try {
      const body = (await tokenRes.json()) as { error?: { message?: string } };
      if (body.error?.message) msg = body.error.message;
    } catch {
      // Abaikan parse error body non-JSON
    }
    throw new Error(msg);
  }

  const { access_token } = (await tokenRes.json()) as { access_token: string };
  await discordSdk.commands.authenticate({ access_token });

  return { mode: "live", accessToken: access_token };
}

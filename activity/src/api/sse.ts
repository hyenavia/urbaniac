import type { AuthSession } from "./discordAuth";
import type { RealtimeEvent } from "./types";

export type StreamStatus = "connected" | "reconnecting" | "offline";

export function connectEventStream(
  session: AuthSession,
  onEvent: (ev: RealtimeEvent) => void,
  onStatusChange: (status: StreamStatus) => void,
): () => void {
  if (session.mode === "mock") {
    onStatusChange("connected");
    return () => {};
  }

  const controller = new AbortController();
  let stopped = false;
  let delayMs = 2000;

  const loop = async () => {
    while (!stopped) {
      try {
        onStatusChange("reconnecting");
        const res = await fetch("/api/events", {
          headers: {
            Authorization: `Bearer ${session.accessToken}`,
          },
          signal: controller.signal,
        });
        if (!res.ok || !res.body) {
          throw new Error(`SSE HTTP ${res.status}`);
        }
        onStatusChange("connected");
        delayMs = 2000;

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buf = "";

        while (!stopped) {
          const { value, done } = await reader.read();
          if (done) break;
          buf += decoder.decode(value, { stream: true });

          let sep = buf.indexOf("\n\n");
          while (sep !== -1) {
            const frame = buf.slice(0, sep);
            buf = buf.slice(sep + 2);
            const dataLines = frame
              .split(/\r?\n/)
              .filter((l) => l.startsWith("data:"))
              .map((l) => l.slice(5).trim());
            if (dataLines.length > 0) {
              const raw = dataLines.join("\n");
              if (raw && raw !== "ping") {
                try {
                  const parsed = JSON.parse(raw) as RealtimeEvent;
                  onEvent(parsed);
                } catch {
                  // Ignore non-JSON frames
                }
              }
            }
            sep = buf.indexOf("\n\n");
          }
        }
      } catch {
        if (stopped) break;
        onStatusChange("reconnecting");
      }

      if (stopped) break;
      await new Promise((r) => setTimeout(r, delayMs));
      delayMs = Math.min(10000, delayMs * 2);
    }
  };

  void loop();

  return () => {
    stopped = true;
    controller.abort();
    onStatusChange("offline");
  };
}

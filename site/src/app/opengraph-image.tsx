import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { ImageResponse } from "next/og";

/** Required for `output: "export"` static generation. */
export const dynamic = "force-static";

export const alt =
  "Onca — Solana tools for ZeroClaw agents. The agent proposes. You dispose.";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

async function loadFont(name: string) {
  return readFile(join(process.cwd(), "src/app/fonts", name));
}

/**
 * Premium OG card: soft light field (matches the live site), Instrument Sans,
 * claw mark, two-tone headline. Built at compile time for static export.
 */
export default async function OpenGraphImage() {
  const [font400, font500, font600] = await Promise.all([
    loadFont("instrument-sans-400.woff"),
    loadFont("instrument-sans-500.woff"),
    loadFont("instrument-sans-600.woff"),
  ]);

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          position: "relative",
          background: "#f4f5f7",
          fontFamily: "Instrument Sans",
          overflow: "hidden",
        }}
      >
        {/* Soft blue light field — upper right, directional */}
        <div
          style={{
            position: "absolute",
            top: -120,
            right: -80,
            width: 720,
            height: 720,
            borderRadius: 9999,
            background:
              "radial-gradient(circle at center, rgba(110,168,255,0.28) 0%, rgba(110,168,255,0.08) 42%, rgba(244,245,247,0) 70%)",
            display: "flex",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: -160,
            left: -100,
            width: 520,
            height: 520,
            borderRadius: 9999,
            background:
              "radial-gradient(circle at center, rgba(61,111,191,0.1) 0%, rgba(244,245,247,0) 68%)",
            display: "flex",
          }}
        />

        {/* Frame */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            justifyContent: "space-between",
            width: "100%",
            height: "100%",
            padding: "56px 64px 48px",
            position: "relative",
          }}
        >
          {/* Top bar */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
              {/* Claw mark */}
              <svg
                width="36"
                height="36"
                viewBox="0 0 32 32"
                fill="none"
                style={{ display: "flex" }}
              >
                <path
                  d="M9 6.5C12 12.5 12 19 10 25.5"
                  stroke="#3d6fbf"
                  strokeWidth="2.6"
                  strokeLinecap="round"
                />
                <path
                  d="M16 5.5C19.2 12.5 19.2 20 16 26.5"
                  stroke="#3d6fbf"
                  strokeWidth="2.6"
                  strokeLinecap="round"
                />
                <path
                  d="M23 6.5C26 12.5 26 19 22 25.5"
                  stroke="#3d6fbf"
                  strokeWidth="2.6"
                  strokeLinecap="round"
                />
              </svg>
              <span
                style={{
                  fontSize: 32,
                  fontWeight: 600,
                  letterSpacing: "-0.03em",
                  color: "#0c0c0e",
                }}
              >
                Onca
              </span>
            </div>
            <span
              style={{
                fontSize: 20,
                fontWeight: 500,
                color: "#8b8b96",
                letterSpacing: "-0.01em",
              }}
            >
              onca.run
            </span>
          </div>

          {/* Headline block */}
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 22,
              maxWidth: 920,
            }}
          >
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 0,
                fontSize: 72,
                fontWeight: 600,
                lineHeight: 1.05,
                letterSpacing: "-0.035em",
                color: "#0c0c0e",
              }}
            >
              <span>The agent proposes.</span>
              <span style={{ color: "#3d6fbf" }}>You dispose.</span>
            </div>
            <span
              style={{
                fontSize: 26,
                fontWeight: 400,
                lineHeight: 1.45,
                color: "#5c5c66",
                maxWidth: 640,
                letterSpacing: "-0.01em",
              }}
            >
              Solana tools for ZeroClaw agents. Read the chain, or build a
              request a person signs. Never a key that can spend.
            </span>
          </div>

          {/* Bottom meta row */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 18,
            }}
          >
            {["T0 read", "T1 build", "wasm32-wasip2"].map((label) => (
              <div
                key={label}
                style={{
                  display: "flex",
                  alignItems: "center",
                  padding: "10px 16px",
                  borderRadius: 999,
                  background: "rgba(12,12,14,0.05)",
                  border: "1px solid rgba(12,12,14,0.06)",
                  fontSize: 18,
                  fontWeight: 500,
                  color: "#5c5c66",
                  letterSpacing: "-0.01em",
                }}
              >
                {label}
              </div>
            ))}
          </div>
        </div>
      </div>
    ),
    {
      ...size,
      fonts: [
        { name: "Instrument Sans", data: font400, weight: 400, style: "normal" },
        { name: "Instrument Sans", data: font500, weight: 500, style: "normal" },
        { name: "Instrument Sans", data: font600, weight: 600, style: "normal" },
      ],
    }
  );
}

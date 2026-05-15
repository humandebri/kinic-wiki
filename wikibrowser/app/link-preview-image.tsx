import { readFile } from "node:fs/promises";
import { ImageResponse } from "next/og";

export const LINK_PREVIEW_ALT = "Kinic Wiki Database Dashboard";
export const LINK_PREVIEW_SIZE = {
  width: 1200,
  height: 630
};
export const LINK_PREVIEW_CONTENT_TYPE = "image/png";

export async function renderLinkPreviewImage() {
  const logoSrc = await kinicLogoDataUrl();
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          background: "#f8f8f8",
          color: "#161616",
          fontFamily: "Arial, Helvetica, sans-serif"
        }}
      >
        <div
          style={{
            width: "100%",
            height: "100%",
            display: "flex",
            padding: 72,
            border: "1px solid #d8d8d8"
          }}
        >
          <div
            style={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              justifyContent: "space-between"
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 22 }}>
              <div
                style={{
                  width: 72,
                  height: 72,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  borderRadius: 16,
                  background: "#ffffff",
                  boxShadow: "0 10px 28px rgba(0, 0, 0, 0.10)",
                  overflow: "hidden"
                }}
              >
                <img
                  alt=""
                  height={72}
                  src={logoSrc}
                  style={{
                    width: 72,
                    height: 72,
                    objectFit: "cover"
                  }}
                  width={72}
                />
              </div>
              <div style={{ display: "flex", flexDirection: "column" }}>
                <div style={{ color: "#5c5c5c", fontSize: 24, fontWeight: 700 }}>Kinic Wiki</div>
                <div style={{ color: "#ff2686", fontSize: 20, fontWeight: 700 }}>Canister database dashboard</div>
              </div>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
              <div style={{ display: "flex", fontSize: 74, fontWeight: 800, lineHeight: 1.02, maxWidth: 900 }}>
                Browse, search, edit, and manage wiki databases.
              </div>
              <div style={{ display: "flex", color: "#4b4b4b", fontSize: 30, lineHeight: 1.35, maxWidth: 820 }}>
                A focused browser and operator UI for Kinic Wiki canisters.
              </div>
            </div>
            <div style={{ display: "flex", gap: 12, color: "#5c5c5c", fontSize: 22, fontWeight: 700 }}>
              <span>/Wiki</span>
              <span>/Sources</span>
              <span>Access</span>
              <span>Query</span>
            </div>
          </div>
        </div>
      </div>
    ),
    LINK_PREVIEW_SIZE
  );
}

async function kinicLogoDataUrl(): Promise<string> {
  const logo = await readFile(new URL("./icon.png", import.meta.url));
  return `data:image/png;base64,${logo.toString("base64")}`;
}

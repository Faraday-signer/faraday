import { ImageResponse } from "next/og";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
// @ts-expect-error — wawoff2 has no types
import { decompress } from "wawoff2";

export const alt = "Faraday — Air-gapped Solana signer";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function Image() {
  const woff2 = await readFile(
    join(process.cwd(), "public", "fonts", "DepartureMono-Regular.woff2")
  );
  const fontData = Buffer.from((await decompress(woff2)) as Uint8Array);

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          backgroundColor: "#f7f6f1",
          backgroundImage: [
            // Primary 100px grid (stronger lines)
            "linear-gradient(to right, rgba(23,23,23,0.14) 1px, transparent 1px)",
            "linear-gradient(to bottom, rgba(23,23,23,0.14) 1px, transparent 1px)",
            // Sub 20px grid (lighter)
            "linear-gradient(to right, rgba(23,23,23,0.06) 1px, transparent 1px)",
            "linear-gradient(to bottom, rgba(23,23,23,0.06) 1px, transparent 1px)",
          ].join(", "),
          backgroundSize: "100px 100px, 100px 100px, 20px 20px, 20px 20px",
          padding: "80px",
          fontFamily: "Departure Mono",
        }}
      >
        {/* Wordmark + finder pattern row */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
          }}
        >
          <svg
            viewBox="0 0 879 186"
            width="380"
            height="80"
            fill="#1AF8FF"
            style={{ filter: "drop-shadow(3px 3px 0 #717171)" }}
          >
            <path d="M757.438 54.0006H770.322V92.5085H757.438V54.0006ZM808.974 54.0006H821.859V92.5085H808.974V54.0006ZM783.206 118.18H796.09V143.852H783.206V118.18ZM783.206 143.852V156.688H757.438V143.852H783.206ZM796.09 92.5085H808.974V118.18H796.09V92.5085ZM770.322 92.5085H783.206V118.18H770.322V92.5085Z" />
            <path d="M667.198 92.5085H680.083V118.18H667.198V92.5085ZM680.083 118.18H705.851V131.016H680.083V118.18ZM705.851 105.344H718.735V92.5085H680.083V79.6725H718.735V66.8366H731.619V131.016H718.735V118.18H705.851V105.344ZM680.083 54.0006H718.735V66.8366H680.083V54.0006Z" />
            <path d="M628.496 28.3287H641.38V131.016H628.496V118.18H615.611V105.344H628.496V79.6725H615.611V66.8366H628.496V28.3287ZM615.611 118.18V131.016H589.843V118.18H615.611ZM589.843 118.18H576.959V66.8366H589.843V118.18ZM615.611 66.8366H589.843V54.0006H615.611V66.8366Z" />
            <path d="M486.72 92.5085H499.604V118.18H486.72V92.5085ZM499.604 118.18H525.372V131.016H499.604V118.18ZM525.372 105.344H538.256V92.5085H499.604V79.6725H538.256V66.8366H551.14V131.016H538.256V118.18H525.372V105.344ZM499.604 54.0006H538.256V66.8366H499.604V54.0006Z" />
            <path d="M422.248 54.0006V66.8366H435.133V79.6725H422.248V118.18H448.017V131.016H396.48V118.18H409.364V66.8366H396.48V54.0006H422.248ZM460.901 54.0006V66.8366H435.133V54.0006H460.901Z" />
            <path d="M306.241 92.5085H319.125V118.18H306.241V92.5085ZM319.125 118.18H344.893V131.016H319.125V118.18ZM344.893 105.344H357.777V92.5085H319.125V79.6725H357.777V66.8366H370.661V131.016H357.777V118.18H344.893V105.344ZM319.125 54.0006H357.777V66.8366H319.125V54.0006Z" />
            <path d="M216.001 28.3287H280.422V41.1647H228.885V66.8366H267.538V79.6725H228.885V131.016H216.001V28.3287Z" />
            <rect x="63" y="28" width="51.5" height="51.5" />
            <rect x="114.5" y="79.5" width="51.5" height="51.5" />
            <rect x="140.736" y="28" width="25.2642" height="25.2642" />
            <rect x="63" y="105.736" width="25.2642" height="25.2642" />
          </svg>
          {/* QR finder pattern — matches the landing page corner mark */}
          <svg
            viewBox="0 0 7 7"
            width="80"
            height="80"
            fill="#171717"
            shapeRendering="crispEdges"
          >
            <rect x="0" y="0" width="7" height="1" />
            <rect x="0" y="6" width="7" height="1" />
            <rect x="0" y="1" width="1" height="5" />
            <rect x="6" y="1" width="1" height="5" />
            <rect x="2" y="2" width="3" height="3" />
          </svg>
        </div>

        {/* Hero text */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            marginTop: 60,
          }}
        >
          <span style={{ fontSize: 64, color: "#424242", lineHeight: 1.1 }}>
            Sign Solana Transactions
          </span>
          <span
            style={{
              fontSize: 96,
              color: "#1AF8FF",
              lineHeight: 1,
              marginTop: 8,
              textShadow: "3px 3px 0 #717171",
            }}
          >
            without trusting
          </span>
          <span
            style={{
              fontSize: 64,
              color: "#424242",
              lineHeight: 1,
              marginTop: 8,
            }}
          >
            your computer
          </span>
        </div>

        {/* Caption */}
        <div
          style={{
            marginTop: 56,
            display: "flex",
            fontSize: 22,
            color: "#717171",
            letterSpacing: 4,
            textTransform: "uppercase",
          }}
        >
          Air-gapped · Memory-resident keys · Open-source
        </div>
      </div>
    ),
    {
      ...size,
      fonts: [
        {
          name: "Departure Mono",
          data: fontData,
          weight: 400,
          style: "normal",
        },
      ],
    }
  );
}

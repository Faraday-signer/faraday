import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

const DESCRIPTION =
  "Faraday is an open-source, air-gapped hardware signer for Solana. Your private keys are generated on a device with no antennas and never leave it. Transactions cross the gap as QR codes. No Wi-Fi, no Bluetooth, no Faraday servers.";

export const metadata: Metadata = {
  metadataBase: new URL("https://faraday.to"),
  title: "Faraday · Air-gapped Solana signer",
  description: DESCRIPTION,
  openGraph: {
    title: "Faraday · Air-gapped Solana signer",
    description: DESCRIPTION,
    type: "website",
    siteName: "Faraday",
    url: "https://faraday.to",
  },
  twitter: {
    card: "summary_large_image",
    title: "Faraday · Air-gapped Solana signer",
    description:
      "Sign Solana transactions without trusting your computer. Keys never touch a network; transactions cross the air gap by QR.",
    creator: "@faradaysigner",
  },
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={`${inter.variable} h-full antialiased`}>
      <body className="min-h-full">{children}</body>
    </html>
  );
}

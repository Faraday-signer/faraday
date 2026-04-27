import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Faraday — Air-gapped Solana signer",
  description:
    "Sign Solana transactions without trusting your computer. A pocket-sized hardware signer with memory-resident keys.",
  openGraph: {
    title: "Faraday — Air-gapped Solana signer",
    description:
      "Sign Solana transactions without trusting your computer. A pocket-sized hardware signer with memory-resident keys.",
    type: "website",
    siteName: "Faraday",
  },
  twitter: {
    card: "summary_large_image",
    title: "Faraday — Air-gapped Solana signer",
    description:
      "Sign Solana transactions without trusting your computer.",
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

# Faraday Playground

Devnet test harness for the Faraday browser extension. Exercises Wallet Standard
`connect` and `signTransaction` end-to-end, from dapp → extension → Faraday
device → back to dapp → broadcast.

## Stack

React · Vite · TypeScript · Tailwind v4 · shadcn/ui · `@wallet-standard/app` · `@solana/web3.js`.

## Run

```bash
cd playground
npm install
npm run dev
```

Opens at <http://localhost:4173>.

The extension must be loaded in the same Chrome profile (from
`extension/.output/chrome-mv3` or `chrome-mv3-dev`) for the playground to detect
it as a Wallet Standard wallet.

## Flow

1. Pair a Solana pubkey in the Faraday extension popup.
2. Open this page.
3. **Connect** — Faraday shows up in the wallet picker automatically.
4. **Airdrop 1 SOL** — devnet top-up.
5. **Sign + send transfer** — Faraday window opens with the unsigned QR; scan it
   on your Faraday device, scan the signed response back, and the playground
   broadcasts the signed transaction to devnet.

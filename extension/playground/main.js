import { getWallets } from "https://esm.sh/@wallet-standard/app@1.1.0";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction
} from "https://esm.sh/@solana/web3.js@1.98.2";

const RPC_URL = "https://api.devnet.solana.com";
const CHAIN = "solana:devnet";

const connection = new Connection(RPC_URL, "confirmed");
const walletsApi = getWallets();

const refreshWalletsBtn = document.getElementById("refreshWalletsBtn");
const walletSelect = document.getElementById("walletSelect");
const connectBtn = document.getElementById("connectBtn");
const disconnectBtn = document.getElementById("disconnectBtn");
const walletInfo = document.getElementById("walletInfo");
const recipientInput = document.getElementById("recipientInput");
const amountInput = document.getElementById("amountInput");
const airdropBtn = document.getElementById("airdropBtn");
const signAndSendBtn = document.getElementById("signAndSendBtn");
const logEl = document.getElementById("log");

const state = {
  wallets: [],
  wallet: null,
  account: null
};

function shortAddress(address) {
  if (address.length <= 12) {
    return address;
  }
  return `${address.slice(0, 4)}...${address.slice(-4)}`;
}

function log(message) {
  const ts = new Date().toLocaleTimeString();
  logEl.textContent = `[${ts}] ${message}\n${logEl.textContent}`;
}

function logError(context, error) {
  const msg = error instanceof Error ? error.message : String(error);
  const stack = error instanceof Error && error.stack ? `\n${error.stack}` : "";
  log(`${context}: ${msg}${stack}`);
}

function setBusy(busy) {
  connectBtn.disabled = busy;
  disconnectBtn.disabled = busy;
  airdropBtn.disabled = busy;
  signAndSendBtn.disabled = busy;
  refreshWalletsBtn.disabled = busy;
}

function updateWalletInfo() {
  if (!state.wallet || !state.account) {
    walletInfo.textContent = "Not connected.";
    return;
  }

  walletInfo.textContent = `Connected: ${state.wallet.name} | ${state.account.address}`;
}

function supportsFaradayFlow(wallet) {
  const chains = Array.isArray(wallet.chains) ? wallet.chains : [];
  const hasSolanaChain = chains.some((chain) => chain.startsWith("solana:"));
  const hasConnect = Boolean(wallet.features?.["standard:connect"]);
  const hasSignTransaction = Boolean(wallet.features?.["solana:signTransaction"]);
  return hasSolanaChain && hasConnect && hasSignTransaction;
}

function refreshWallets() {
  state.wallets = walletsApi.get().filter(supportsFaradayFlow);

  walletSelect.innerHTML = "";
  if (state.wallets.length === 0) {
    const option = document.createElement("option");
    option.textContent = "No Wallet Standard wallets found";
    option.value = "";
    walletSelect.appendChild(option);
    log("No Wallet Standard wallets detected on this page.");
    return;
  }

  state.wallets.forEach((wallet, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = wallet.name;
    walletSelect.appendChild(option);
  });

  log(`Detected wallets: ${state.wallets.map((wallet) => wallet.name).join(", ")}`);
}

async function connectWallet() {
  if (state.wallets.length === 0) {
    throw new Error("No compatible wallet found.");
  }

  const selectedIndex = Number(walletSelect.value || 0);
  const wallet = state.wallets[selectedIndex];
  if (!wallet) {
    throw new Error("Select a wallet first.");
  }

  const connectFeature = wallet.features["standard:connect"];
  const result = await connectFeature.connect();
  const account = result.accounts?.[0] || wallet.accounts?.[0];

  if (!account) {
    throw new Error("Connected wallet did not return any account.");
  }

  state.wallet = wallet;
  state.account = account;
  updateWalletInfo();
  log(`Connected ${wallet.name} (${shortAddress(account.address)})`);
}

async function disconnectWallet() {
  if (!state.wallet) {
    return;
  }

  const disconnectFeature = state.wallet.features["standard:disconnect"];
  if (disconnectFeature?.disconnect) {
    await disconnectFeature.disconnect();
  }

  state.wallet = null;
  state.account = null;
  updateWalletInfo();
  log("Disconnected wallet.");
}

function ensureConnected() {
  if (!state.wallet || !state.account) {
    throw new Error("Connect wallet first.");
  }
}

async function requestAirdrop() {
  ensureConnected();
  const pubkey = new PublicKey(state.account.address);

  log(`Requesting devnet airdrop for ${state.account.address}...`);
  const signature = await connection.requestAirdrop(pubkey, LAMPORTS_PER_SOL);
  await connection.confirmTransaction(signature, "confirmed");
  log(`Airdrop confirmed: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
}

async function signAndSendTransfer() {
  ensureConnected();

  const fromAddress = state.account.address;
  const toAddress = recipientInput.value.trim() || fromAddress;
  const amountSol = Number(amountInput.value);

  if (!Number.isFinite(amountSol) || amountSol <= 0) {
    throw new Error("Amount must be > 0.");
  }

  const lamports = Math.floor(amountSol * LAMPORTS_PER_SOL);
  const latest = await connection.getLatestBlockhash("confirmed");

  const tx = new Transaction({
    feePayer: new PublicKey(fromAddress),
    recentBlockhash: latest.blockhash
  }).add(
    SystemProgram.transfer({
      fromPubkey: new PublicKey(fromAddress),
      toPubkey: new PublicKey(toAddress),
      lamports
    })
  );

  const unsignedBytes = tx.serialize({
    requireAllSignatures: false,
    verifySignatures: false
  });

  log(
    `Requesting signature for ${amountSol} SOL transfer to ${shortAddress(toAddress)}. ` +
      "Scan the unsigned QR in the Faraday window."
  );

  const signFeature = state.wallet.features["solana:signTransaction"];
  const outputs = await signFeature.signTransaction({
    account: state.account,
    chain: CHAIN,
    transaction: unsignedBytes
  });

  const signedBytes = outputs?.[0]?.signedTransaction;
  if (!signedBytes) {
    throw new Error("Wallet did not return signed transaction bytes.");
  }

  log("Signed payload received. Broadcasting to devnet...");
  const signature = await connection.sendRawTransaction(signedBytes, {
    skipPreflight: false
  });

  await connection.confirmTransaction(
    {
      signature,
      blockhash: latest.blockhash,
      lastValidBlockHeight: latest.lastValidBlockHeight
    },
    "confirmed"
  );

  log(`Transaction confirmed: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
}

refreshWalletsBtn.addEventListener("click", () => {
  refreshWallets();
});

connectBtn.addEventListener("click", async () => {
  setBusy(true);
  try {
    await connectWallet();
  } catch (error) {
    logError("Connect failed", error);
  } finally {
    setBusy(false);
  }
});

disconnectBtn.addEventListener("click", async () => {
  setBusy(true);
  try {
    await disconnectWallet();
  } catch (error) {
    logError("Disconnect failed", error);
  } finally {
    setBusy(false);
  }
});

airdropBtn.addEventListener("click", async () => {
  setBusy(true);
  try {
    await requestAirdrop();
  } catch (error) {
    logError("Airdrop failed", error);
  } finally {
    setBusy(false);
  }
});

signAndSendBtn.addEventListener("click", async () => {
  setBusy(true);
  try {
    await signAndSendTransfer();
  } catch (error) {
    logError("Sign+send failed", error);
  } finally {
    setBusy(false);
  }
});

const offRegister = walletsApi.on("register", refreshWallets);
const offUnregister = walletsApi.on("unregister", refreshWallets);

window.addEventListener("beforeunload", () => {
  offRegister();
  offUnregister();
});

refreshWallets();
updateWalletInfo();
log("Playground loaded. Click 'Refresh Wallets' if needed.");

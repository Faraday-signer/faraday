export type SignSessionStatus = "pending" | "completed" | "canceled" | "failed";

export interface ExtensionState {
  pairedPubkey: string | null;
  approvedOrigins: string[];
}

export interface SignSession {
  id: string;
  origin: string;
  kind: "tx" | "message";
  txBase64?: string;
  messageBase64?: string;
  messageQrBase64?: string;
  expectedPubkey: string;
  status: SignSessionStatus;
  createdAt: number;
  expiresAt: number;
  signedTxBase64?: string;
  signatureHex?: string;
  error?: string;
}

export type RuntimeRequest =
  | { type: "faraday:get-state" }
  | { type: "faraday:set-paired-pubkey"; pubkey: string }
  | { type: "faraday:clear-paired-pubkey" }
  | { type: "faraday:clear-approved-origins" }
  | { type: "faraday:approve-origin"; origin: string }
  | { type: "faraday:revoke-origin"; origin: string }
  | { type: "faraday:connect-check"; origin: string }
  | { type: "faraday:create-sign-session"; origin: string; txBase64: string }
  | { type: "faraday:create-sign-message-session"; origin: string; messageBase64: string }
  /// Sidepanel-originated session (Send flow). Skips the approved-origins
  /// check because the request comes from our own extension surface, not
  /// a dapp. Sender must be the extension itself (enforced in handler).
  | { type: "faraday:ext-create-sign-session"; txBase64: string }
  | { type: "faraday:open-sign-window"; origin: string; sessionId: string }
  /// Sidepanel-originated open-sign-window — no origin check.
  | { type: "faraday:ext-open-sign-window"; sessionId: string }
  | { type: "faraday:get-sign-session"; sessionId: string }
  | { type: "faraday:get-sign-result"; sessionId: string }
  | { type: "faraday:complete-sign-session"; sessionId: string; signedTxBase64: string }
  | { type: "faraday:complete-sign-message-session"; sessionId: string; signatureHex: string }
  | { type: "faraday:cancel-sign-session"; sessionId: string; reason?: string };

export type RuntimeResponse<T = unknown> =
  | {
      ok: true;
      data: T;
    }
  | {
      ok: false;
      error: string;
    };

export interface ConnectCheckResult {
  pairedPubkey: string | null;
  approved: boolean;
}

export interface CreateSignSessionResult {
  sessionId: string;
  signUrl: string;
}

export interface GetSignSessionResult {
  sessionId: string;
  origin: string;
  kind: "tx" | "message";
  txBase64?: string;
  messageQrBase64?: string;
  expectedPubkey: string;
  status: SignSessionStatus;
  error?: string;
}

export interface GetSignResult {
  kind: "tx" | "message";
  status: SignSessionStatus;
  signedTxBase64?: string;
  signatureHex?: string;
  error?: string;
}

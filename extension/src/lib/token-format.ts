export function shortMint(mint: string, edgeChars = 4): string {
  const minLength = edgeChars * 2 + 2;
  if (mint.length <= minLength) return mint;
  return `${mint.slice(0, edgeChars)}…${mint.slice(-edgeChars)}`;
}

export function formatTokenAmount(amount: number, decimals: number): string {
  if (amount === 0) return "0";
  if (amount < 0.000001) return amount.toExponential(2);
  const maxFrac = Math.min(decimals, amount < 1 ? 6 : 4);
  return amount.toLocaleString("en-US", { maximumFractionDigits: maxFrac });
}

export function formatTokenUsd(value: number): string {
  if (value < 0.01) return "<$0.01";
  return `$${value.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
}

export function formatPricePerToken(
  price: number,
  maxLargeFractionDigits = 2
): string {
  if (price >= 1) {
    return `$${price.toLocaleString("en-US", {
      maximumFractionDigits: maxLargeFractionDigits,
    })}`;
  }
  if (price >= 0.01) {
    return `$${price.toFixed(4)}`;
  }
  if (price > 0) {
    return `$${price.toExponential(2)}`;
  }
  return "—";
}

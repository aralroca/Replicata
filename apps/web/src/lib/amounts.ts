// Token amounts: integers in base units (USDC has 6 decimals). Never use floats for money.

export const USDC_DECIMALS = 6;

/** 1500000 → "1.50"; keeps at least 2 decimals, trims the rest of trailing zeros. */
export function formatAmount(base: number | bigint, decimals = USDC_DECIMALS): string {
  const value = BigInt(base);
  const sign = value < 0n ? "-" : "";
  const digits = (value < 0n ? -value : value).toString().padStart(decimals + 1, "0");
  const whole = digits.slice(0, digits.length - decimals);
  const fraction = digits
    .slice(digits.length - decimals)
    .replace(/0+$/, "")
    .padEnd(2, "0");

  return `${sign}${BigInt(whole).toLocaleString("en-US")}.${fraction}`;
}

export function formatUsdc(base: number | bigint, decimals = USDC_DECIMALS): string {
  return `${formatAmount(base, decimals)} USDC`;
}

export function formatPrice(base: number | bigint, decimals = USDC_DECIMALS): string {
  return `$${formatAmount(base, decimals)}`;
}

/** "1.5" → 1500000n. Returns null for anything that is not a non-negative decimal. */
export function parseAmount(text: string, decimals = USDC_DECIMALS): bigint | null {
  const match = /^(\d+)(?:\.(\d*))?$/.exec(text.trim());
  const fraction = match?.[2] ?? "";

  if (!match || fraction.length > decimals) {
    return null;
  }

  return BigInt(match[1]) * 10n ** BigInt(decimals) + BigInt(fraction.padEnd(decimals, "0") || 0);
}

/** 7000 bps → "70%". */
export function formatBps(bps: number): string {
  return `${(bps / 100).toFixed(bps % 100 === 0 ? 0 : 2)}%`;
}

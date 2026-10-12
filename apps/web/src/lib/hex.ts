const HEX_32 = /^[0-9a-fA-F]{64}$/;

export function isHex32(value: string): boolean {
  return HEX_32.test(value);
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** 64-char hex → 32 bytes (throws on anything else). */
export function hex32ToBytes(hex: string): Uint8Array {
  if (!isHex32(hex)) {
    throw new Error(`expected 64 hex chars, got "${hex}"`);
  }

  return Uint8Array.from(hex.match(/../g) ?? [], (pair) => parseInt(pair, 16));
}

/** `sha256:<hex>` image digest → its 32 raw bytes (on-chain `image_digest`). */
export function imageDigestBytes(digest: string): Uint8Array {
  return hex32ToBytes(digest.replace(/^sha256:/, ""));
}

export function shortHash(value: string, size = 6): string {
  return value.length <= size * 2 + 1 ? value : `${value.slice(0, size)}…${value.slice(-size)}`;
}

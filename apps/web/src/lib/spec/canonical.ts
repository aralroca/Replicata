// Canonical JSON (keys sorted bytewise, compact) and spec hash, identical to
// `replicata_core::JobSpec::canonical_json` / `spec_hash` (see vectors/core.json).
import type { JobSpec } from "@/lib/types";
import { bytesToHex } from "@/lib/hex";

type Json = null | boolean | number | bigint | string | Json[] | { [key: string]: Json };

const encoder = new TextEncoder();

/** Bytewise (UTF-8) comparison, like Rust's `BTreeMap<String, _>` ordering. */
export function compareUtf8(a: string, b: string): number {
  const left = encoder.encode(a);
  const right = encoder.encode(b);
  const index = left.findIndex((byte, i) => byte !== right[i]);

  if (index === -1) {
    return left.length - right.length;
  }

  return index >= right.length ? 1 : left[index] - right[index];
}

function canonicalObject(value: { [key: string]: Json }): string {
  const keys = Object.keys(value).sort(compareUtf8);
  const members = keys.map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`);

  return `{${members.join(",")}}`;
}

export function canonicalJson(value: Json): string {
  if (typeof value === "bigint") {
    return value.toString();
  }
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    return canonicalObject(value);
  }

  return JSON.stringify(value);
}

export function canonicalSpecJson(spec: JobSpec): string {
  return canonicalJson(spec as unknown as Json);
}

export async function sha256(bytes: Uint8Array): Promise<Uint8Array> {
  const digest = await crypto.subtle.digest("SHA-256", bytes as BufferSource);

  return new Uint8Array(digest);
}

export async function specHashBytes(spec: JobSpec): Promise<Uint8Array> {
  return sha256(encoder.encode(canonicalSpecJson(spec)));
}

export async function specHashHex(spec: JobSpec): Promise<string> {
  return bytesToHex(await specHashBytes(spec));
}

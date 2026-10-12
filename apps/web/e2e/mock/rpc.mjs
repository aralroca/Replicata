// The handful of Solana JSON-RPC methods the browser needs against the mock:
// blockhash (publish + x402), the USDC mint account (x402 reads decimals/program) and sends.
import { MINT } from "./fixtures.mjs";

const TOKEN_PROGRAM = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const BLOCKHASH = "EkSnNWid2cvwEVnVx9aBqawnmiCNiDgp3gUdkDPTKN1N";

/** SPL Token mint layout (82 bytes): authority option + key, supply, decimals, initialized, freeze option + key. */
function mintData(decimals) {
  const data = Buffer.alloc(82);

  data.writeUInt32LE(0, 0);
  data.writeUInt8(decimals, 44);
  data.writeUInt8(1, 45);

  return data.toString("base64");
}

const MINT_ACCOUNT = {
  data: [mintData(6), "base64"],
  executable: false,
  lamports: 1_461_600,
  owner: TOKEN_PROGRAM,
  rentEpoch: 0,
  space: 82,
};

const context = { apiVersion: "2.3.0", slot: 1 };

const METHODS = {
  getLatestBlockhash: () => ({ context, value: { blockhash: BLOCKHASH, lastValidBlockHeight: 1_000 } }),
  getAccountInfo: ([address]) => ({ context, value: address === MINT ? MINT_ACCOUNT : null }),
  getSignatureStatuses: ([signatures]) => ({ context, value: signatures.map(() => null) }),
  sendTransaction: () => "5".repeat(88),
};

export function rpc({ id, method, params = [] }) {
  const handler = METHODS[method];

  return handler
    ? { jsonrpc: "2.0", id, result: handler(params) }
    : { jsonrpc: "2.0", id, error: { code: -32601, message: `mock: ${method} not implemented` } };
}

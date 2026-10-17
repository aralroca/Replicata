// Browser init script: a Wallet Standard wallet ("E2E Wallet") that signs with a local Ed25519 key
// through WebCrypto. Config comes from `window.__E2E_WALLET__` (set by an earlier init script):
// { seed: number[32], publicKey: number[32], address: string, sendRpc?: string }.
// `window.__e2eWallet.rejectNext = true` makes the next signature request fail like a user rejection.
{
  const config = window.__E2E_WALLET__;
  const CHAINS = ["solana:devnet", "solana:localnet", "solana:testnet", "solana:mainnet"];
  const FEATURES = ["solana:signTransaction", "solana:signAndSendTransaction", "solana:signMessage"];
  const PKCS8_ED25519 = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
  ];
  const ICON =
    "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHJlY3Qgd2lkdGg9IjMyIiBoZWlnaHQ9IjMyIiBmaWxsPSIjNThhNmZmIi8+PC9zdmc+";
  const state = { connected: false, rejectNext: false, signed: 0, sent: [] };
  const listeners = new Set();
  const keyPromise = crypto.subtle.importKey(
    "pkcs8",
    new Uint8Array([...PKCS8_ED25519, ...config.seed]),
    { name: "Ed25519" },
    false,
    ["sign"]
  );
  const account = Object.freeze({
    address: config.address,
    publicKey: new Uint8Array(config.publicKey),
    chains: CHAINS,
    features: FEATURES,
    label: "E2E",
  });

  /** compact-u16 ("shortvec") at `offset` → { value, size }. */
  const shortVec = (bytes, offset) =>
    [0, 1, 2].reduce((acc, i) => (acc.done ? acc : step(acc, bytes[offset + i], i)), {
      value: 0,
      size: 0,
      done: false,
    });
  const step = (acc, byte, i) => ({ value: acc.value | ((byte & 0x7f) << (7 * i)), size: i + 1, done: !(byte & 0x80) });

  const sameKey = (bytes, start) => account.publicKey.every((byte, i) => bytes[start + i] === byte);

  function signerIndex(message) {
    const header = message[0] & 0x80 ? 1 : 0;
    const keys = shortVec(message, header + 3);
    const first = header + 3 + keys.size;
    const index = Array.from({ length: message[header] }, (_, i) => i).find((i) => sameKey(message, first + 32 * i));

    if (index === undefined) throw new Error("E2E wallet is not a signer of this transaction");

    return index;
  }

  async function signWire(wire) {
    const sigs = shortVec(wire, 0);
    const message = wire.slice(sigs.size + 64 * sigs.value);
    const signature = new Uint8Array(await crypto.subtle.sign("Ed25519", await keyPromise, message));
    const signed = new Uint8Array(wire);

    signed.set(signature, sigs.size + 64 * signerIndex(message));
    state.signed += 1;

    return signed;
  }

  function guard() {
    const rejected = state.rejectNext;

    state.rejectNext = false;
    if (rejected) throw Object.assign(new Error("User rejected the request."), { code: 4001 });
  }

  const JSON_HEADERS = { "Content-Type": "application/json" };

  // Like a real wallet: simulate at the dapp's blockhash commitment and surface RPC failures.
  async function rpcSend(signed) {
    const options = { encoding: "base64", preflightCommitment: "confirmed" };
    const params = [btoa(String.fromCharCode(...signed)), options];
    const body = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "sendTransaction", params });
    const { error } = await (await fetch(config.sendRpc, { method: "POST", headers: JSON_HEADERS, body })).json();

    if (error) throw new Error(error.message);
  }

  async function send(signed) {
    state.sent.push(Array.from(signed));
    if (config.sendRpc) await rpcSend(signed);
  }

  async function signAndSend({ transaction }) {
    const signed = await signWire(transaction);

    await send(signed);

    return { signature: signed.slice(1, 65) };
  }

  const emit = () => listeners.forEach((listener) => listener({ accounts: wallet.accounts }));

  async function connect() {
    state.connected = true;
    emit();

    return { accounts: wallet.accounts };
  }

  async function disconnect() {
    state.connected = false;
    emit();
  }

  function on(_event, listener) {
    listeners.add(listener);

    return () => listeners.delete(listener);
  }

  async function signTransaction(...inputs) {
    guard();

    return Promise.all(inputs.map(async ({ transaction }) => ({ signedTransaction: await signWire(transaction) })));
  }

  async function signAndSendTransaction(...inputs) {
    guard();

    return Promise.all(inputs.map(signAndSend));
  }

  async function signOne({ message }) {
    const signature = new Uint8Array(await crypto.subtle.sign("Ed25519", await keyPromise, message));

    return { signedMessage: message, signature };
  }

  const TX_VERSIONS = ["legacy", 0];
  const features = {
    "standard:connect": { version: "1.0.0", connect },
    "standard:disconnect": { version: "1.0.0", disconnect },
    "standard:events": { version: "1.0.0", on },
    "solana:signTransaction": { version: "1.0.0", supportedTransactionVersions: TX_VERSIONS, signTransaction },
    "solana:signAndSendTransaction": {
      version: "1.0.0",
      supportedTransactionVersions: TX_VERSIONS,
      signAndSendTransaction,
    },
    "solana:signMessage": { version: "1.0.0", signMessage: (...inputs) => Promise.all(inputs.map(signOne)) },
  };

  const wallet = {
    version: "1.0.0",
    name: "E2E Wallet",
    icon: ICON,
    chains: CHAINS,
    features,
    get accounts() {
      return state.connected ? [account] : [];
    },
  };

  const register = ({ register: add }) => add(wallet);

  window.__e2eWallet = state;
  window.addEventListener("wallet-standard:app-ready", ({ detail }) => register(detail));
  window.dispatchEvent(new CustomEvent("wallet-standard:register-wallet", { detail: register }));
}

// Tiny mock of the Replicata coordinator (docs/API.md) + the few Solana JSON-RPC calls the web
// app makes. Node built-ins only. Usage: `node e2e/mock/server.mjs` (PORT, default 8799).
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { gzipSync } from "node:zlib";

import * as data from "./fixtures.mjs";
import { rpc } from "./rpc.mjs";

const PORT = Number(process.env.PORT ?? 8799);
const ORIGIN = `http://localhost:${PORT}`;
const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "*",
  "Access-Control-Allow-Methods": "*",
  "Access-Control-Expose-Headers": "payment-required,payment-response,x-result-hash,content-disposition,retry-after",
};
const payments = [];

const b64 = (value) => Buffer.from(JSON.stringify(value)).toString("base64");

function send(res, status, body, headers = {}) {
  const isBuffer = Buffer.isBuffer(body);
  const type = isBuffer ? "application/gzip" : "application/json";

  res.writeHead(status, { ...CORS, "Content-Type": type, ...headers });
  res.end(isBuffer ? body : JSON.stringify(body));
}

const findExperiment = (id) => data.experiments().find((experiment) => experiment.id === id);

function deliver(res, id) {
  return send(res, 200, gzipSync(Buffer.from("verified result\n")), {
    "X-Result-Hash": "cb8420d7fe1f37f793a6a57a1f4d001a0bf6db2bf296f9046ca32e764fa29776",
    "Content-Disposition": `attachment; filename="${id}.tar.gz"`,
    "PAYMENT-RESPONSE": b64({ success: true, transaction: "mock-settlement", network: data.NETWORK }),
  });
}

/** What happens after the client sent PAYMENT-SIGNATURE, per scenario. */
const PAID_OUTCOMES = {
  [data.IDS.noFunds]: (res) => send(res, 402, { error: "insufficient_funds", faucet: "https://faucet.circle.com" }),
  [data.IDS.down]: (res) => send(res, 503, { error: "facilitator_unavailable" }, { "Retry-After": "5" }),
};

function sendPaid(res, id) {
  return (PAID_OUTCOMES[id] ?? deliver)(res, id);
}

function result(req, res, [id]) {
  const experiment = findExperiment(id);
  const signature = req.headers["payment-signature"];
  const required = () => data.paymentRequired(id, `${ORIGIN}/v1/experiments/${id}/result`);

  if (!experiment) return send(res, 404, { error: "not_found" });
  if (experiment.status !== "replicated") return send(res, 409, { error: "not_replicated" });
  if (!signature) return send(res, 402, { error: "payment_required" }, { "PAYMENT-REQUIRED": b64(required()) });
  payments.push(JSON.parse(Buffer.from(signature, "base64").toString()));

  return sendPaid(res, id);
}

const ROUTES = [
  ["GET", /^\/v1\/config$/, (req, res) => send(res, 200, data.config(`${ORIGIN}/rpc`))],
  ["GET", /^\/v1\/experiments$/, (req, res) => send(res, 200, data.experiments())],
  ["GET", /^\/v1\/experiments\/([^/]+)\/result$/, result],
  ["GET", /^\/v1\/experiments\/([^/]+)$/, (req, res, [id]) => send(res, ...experimentOr404(id))],
  ["GET", /^\/v1\/workers\/([^/]+)$/, (req, res, [pk]) => send(res, 200, data.worker(pk))],
  ["GET", /^\/v1\/authors\/([^/]+)$/, (req, res, [pk]) => send(res, 200, data.author(pk))],
  ["POST", /^\/v1\/specs$/, (req, res, _, body) => send(res, 201, { spec_hash: sha256(body) })],
  ["POST", /^\/rpc$/, (req, res, _, body) => send(res, 200, rpc(JSON.parse(body.toString())))],
  ["GET", /^\/__payments$/, (req, res) => send(res, 200, payments)],
];

function experimentOr404(id) {
  const experiment = findExperiment(id);

  return experiment ? [200, experiment] : [404, { error: "not_found" }];
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function route(req, res, body) {
  const path = new URL(req.url, ORIGIN).pathname;
  const match = ROUTES.map(([method, pattern, handler]) => [method, pattern.exec(path), handler]).find(
    ([method, found]) => method === req.method && found
  );

  if (req.method === "OPTIONS") return res.writeHead(204, CORS).end();
  if (!match) return send(res, 404, { error: "no_route" });

  return match[2](req, res, match[1].slice(1), body);
}

createServer((req, res) => {
  const chunks = [];

  req.on("data", (chunk) => chunks.push(chunk));
  req.on("end", () => route(req, res, Buffer.concat(chunks)));
}).listen(PORT, () => console.log(`mock coordinator on ${ORIGIN}`));

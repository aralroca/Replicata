// Generates the JS client of the Replicata program from the Anchor IDL.
// Usage: `pnpm gen` (writes apps/web/src/generated) · `pnpm gen:check` (fails on drift).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";
import { createFromRoot } from "codama";

const idlPath = fileURLToPath(
  new URL("./target/idl/replicata.json", import.meta.url)
);
const webPath = fileURLToPath(new URL("./apps/web", import.meta.url));
const idl = JSON.parse(readFileSync(idlPath, "utf8"));
const codama = createFromRoot(rootNodeFromAnchor(idl));

await codama.accept(
  renderVisitor(webPath, {
    generatedFolder: "src/generated",
    kitImportStrategy: "rootOnly",
    syncPackageJson: false,
  })
);

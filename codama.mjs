// Generates the JS client of the Replicata program from the Anchor IDL.
// Usage: `pnpm gen` (writes apps/web/src/generated) · `pnpm gen:check` (fails on drift).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";
import { createFromRoot, updateInstructionsVisitor } from "codama";

const idlPath = fileURLToPath(
  new URL("./idl/replicata.json", import.meta.url)
);
const webPath = fileURLToPath(new URL("./apps/web", import.meta.url));
const idl = JSON.parse(readFileSync(idlPath, "utf8"));
const codama = createFromRoot(rootNodeFromAnchor(idl));

// The `claim` instruction and the `Claim` account would both export `CLAIM_DISCRIMINATOR`.
// Renaming is client-side only: the instruction discriminator bytes come from the IDL.
codama.update(updateInstructionsVisitor({ claim: { name: "claimSlot" } }));

await codama.accept(
  renderVisitor(webPath, {
    generatedFolder: "src/generated",
    kitImportStrategy: "rootOnly",
    syncPackageJson: false,
  })
);

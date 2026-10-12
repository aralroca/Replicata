import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

// House style: declarative code, small files and functions, no `void`.
const houseStyle = {
  rules: {
    "no-void": "error",
    "max-lines": ["error", { max: 200, skipBlankLines: false, skipComments: false }],
    // 10-line bodies + signature + closing brace.
    "max-lines-per-function": ["error", { max: 12, skipBlankLines: true, skipComments: true }],
    "padding-line-between-statements": [
      "error",
      { blankLine: "always", prev: "*", next: "return" },
      { blankLine: "always", prev: ["const", "let", "var"], next: "*" },
      { blankLine: "any", prev: ["const", "let", "var"], next: ["const", "let", "var"] },
    ],
    "prefer-const": "error",
    "no-var": "error",
  },
};

export default defineConfig([
  ...nextVitals,
  ...nextTs,
  houseStyle,
  globalIgnores([
    ".next/**",
    ".next-e2e/**",
    ".vitest/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    "src/generated/**",
    "playwright-report/**",
    "test-results/**",
  ]),
]);

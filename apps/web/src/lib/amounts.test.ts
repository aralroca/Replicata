import { expect, test } from "vitest";

import { formatAmount, formatBps, formatPrice, formatUsdc, parseAmount } from "@/lib/amounts";

test.each([
  [0, "0.00"],
  [1, "0.000001"],
  [10_000, "0.01"],
  [1_500_000, "1.50"],
  [3_000_000, "3.00"],
  [1_234_567_890, "1,234.56789"],
  [18_446_744_073_709_551_615n, "18,446,744,073,709.551615"],
  [-2_500_000, "-2.50"],
])("formatAmount(%s) = %s", (base, text) => {
  expect(formatAmount(base)).toBe(text);
});

test("currency helpers", () => {
  expect(formatUsdc(3_000_000)).toBe("3.00 USDC");
  expect(formatPrice(10_000)).toBe("$0.01");
  expect(formatAmount(1_500, 3)).toBe("1.50");
});

test.each([
  ["1", 1_000_000n],
  ["1.5", 1_500_000n],
  [" 0.01 ", 10_000n],
  ["0.000001", 1n],
  ["3.", 3_000_000n],
  ["0.0000001", null],
  ["-1", null],
  ["1e3", null],
  ["", null],
  ["abc", null],
])("parseAmount(%j) = %s", (text, base) => {
  expect(parseAmount(text)).toBe(base);
});

test("formatBps", () => {
  expect(formatBps(7000)).toBe("70%");
  expect(formatBps(12_50)).toBe("12.50%");
});

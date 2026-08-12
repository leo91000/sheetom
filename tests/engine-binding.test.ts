import assert from "node:assert/strict";
import { test } from "vitest";

import {
  expectedEngineAbiIdentity,
  validateEngineBindingIdentity,
} from "../src/internal/engine-binding.js";

test("accepts an engine with the exact generated ABI identity", () => {
  assert.doesNotThrow(() => validateEngineBindingIdentity({
    engineAbiIdentity: () => JSON.stringify(expectedEngineAbiIdentity),
  }));
});

test("rejects every ABI identity field before the engine is used", () => {
  for (const [field, value] of [
    ["abiVersion", expectedEngineAbiIdentity.abiVersion + 1],
    ["sheetomVersion", "0.0.0-incompatible"],
    ["syntaxEngineSetSha256", "0".repeat(64)],
  ] as const) {
    const incompatible = { ...expectedEngineAbiIdentity, [field]: value };
    assert.throws(
      () => validateEngineBindingIdentity({
        engineAbiIdentity: () => JSON.stringify(incompatible),
      }),
      error => error instanceof Error
        && error.name === "SheetOMEngineBindingError"
        && Reflect.get(error, "code") === "SHEETOM_ENGINE_ABI_MISMATCH",
      field,
    );
  }
});

test("rejects an undecodable ABI identity as a protocol error", () => {
  assert.throws(
    () => validateEngineBindingIdentity({ engineAbiIdentity: () => "not-json" }),
    error => error instanceof Error
      && error.name === "SheetOMEngineBindingError"
      && Reflect.get(error, "code") === "SHEETOM_ENGINE_ABI_INVALID",
  );
});

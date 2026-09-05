import { test } from "node:test";
import assert from "node:assert/strict";
import { selectIds } from "./selection.ts";
const visible = ["one", "two", "three", "four"];
test("click toggles a selected page and replaces selection for a new page", () => {
  assert.deepEqual(selectIds(["two"], "two", visible, "two", false, false), []);
  assert.deepEqual(selectIds(["one"], "two", visible, "one", false, false), [
    "two",
  ]);
});
test("Ctrl toggles individual pages while retaining the others", () => {
  const current = ["one", "three"];
  assert.deepEqual(selectIds(current, "two", visible, "one", false, true), [
    "one",
    "three",
    "two",
  ]);
  assert.deepEqual(selectIds(current, "one", visible, "one", false, true), [
    "three",
  ]);
  assert.deepEqual(current, ["one", "three"]);
});
test("Shift selects visible ranges in both directions; Ctrl+Shift extends", () => {
  assert.deepEqual(selectIds(["four"], "two", visible, "four", true, false), [
    "two",
    "three",
    "four",
  ]);
  assert.deepEqual(selectIds(["one"], "three", visible, "two", true, true), [
    "one",
    "two",
    "three",
  ]);
});
test("a removed anchor or missing destination does not select unrelated pages", () => {
  assert.deepEqual(
    selectIds(["one"], "three", visible, "removed", true, false),
    ["three"],
  );
  assert.deepEqual(selectIds(["one"], "removed", visible, "two", true, false), [
    "one",
  ]);
});

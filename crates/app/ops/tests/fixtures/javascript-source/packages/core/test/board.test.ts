import { test } from "node:test";

function adds(): number {
  return 2;
}

test("adds", () => {
  adds();
});

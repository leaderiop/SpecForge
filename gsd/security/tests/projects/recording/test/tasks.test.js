import test from "node:test";
import { add, list } from "../src/tasks.js";

test("adds a task", () => {
  list(add([], "write"));
});

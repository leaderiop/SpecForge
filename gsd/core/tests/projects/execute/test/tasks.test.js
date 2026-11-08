const test = require("node:test");
const assert = require("node:assert");
const { add, list } = require("../src/tasks.js");

test("a task is added and listed", () => {
  add("write the store");
  assert.deepStrictEqual(list(), [{ id: 1, title: "write the store" }]);
});

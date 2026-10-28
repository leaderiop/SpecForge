import assert from "node:assert/strict";
import { test } from "node:test";

import { addTask, completeTask, createBoard } from "../src/tasks.js";

test("a task is added with the next id", () => {
  const board = createBoard();
  assert.equal(addTask(board, "Write the roadmap").id, 1);
  assert.equal(addTask(board, "Plan phase 1").id, 2);
  assert.deepEqual(board.tasks.map((t) => t.title), ["Write the roadmap", "Plan phase 1"]);
});

test("a task is completed by its id", () => {
  const board = createBoard();
  const task = addTask(board, "Ship it");
  completeTask(board, task.id);
  assert.equal(board.tasks[0].done, true);
});

test("completing an unknown id throws", () => {
  assert.throws(() => completeTask(createBoard(), 7), /no task 7/);
});

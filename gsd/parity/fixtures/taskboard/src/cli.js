#!/usr/bin/env node
// taskboard add <title> | taskboard done <id>

import { addTask, completeTask, createBoard } from "./tasks.js";

const board = createBoard();
const [command, ...rest] = process.argv.slice(2);

if (command === "add") {
  const task = addTask(board, rest.join(" "));
  console.log(`added ${task.id}: ${task.title}`);
} else if (command === "done") {
  const task = completeTask(board, Number(rest[0]));
  console.log(`done ${task.id}: ${task.title}`);
} else {
  console.error("usage: taskboard add <title> | taskboard done <id>");
  process.exitCode = 2;
}

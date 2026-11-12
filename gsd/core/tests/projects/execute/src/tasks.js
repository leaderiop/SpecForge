// An in-memory task store.
const tasks = [];

function add(title) {
  const task = { id: tasks.length + 1, title };
  tasks.push(task);
  return task;
}

function list() {
  return tasks.slice();
}

module.exports = { add, list };

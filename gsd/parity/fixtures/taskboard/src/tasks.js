// The task model: a board is a list of tasks with integer ids.

export function createBoard() {
  return { tasks: [], nextId: 1 };
}

export function addTask(board, title) {
  const task = { id: board.nextId, title, done: false };
  board.tasks.push(task);
  board.nextId += 1;
  return task;
}

export function completeTask(board, id) {
  const task = board.tasks.find((t) => t.id === id);
  if (!task) throw new Error(`no task ${id}`);
  task.done = true;
  return task;
}

// The persistence boundary for tasks (hexagonal "driven" port).

use "types/task"

port TaskRepository "Persistence boundary for tasks" {
  direction outbound
  category  "persistence/task"

  method create(cmd: CreateTaskCommand)   -> Result<Task, InvalidTitleError>
  method findById(id: string)             -> Result<Task, TaskNotFoundError>
  method complete(cmd: CompleteTaskCommand) -> Result<Task, TaskNotFoundError>
  method list(status: TaskStatus)         -> Result<Task[], never>
  method delete(id: string)               -> Result<void, TaskNotFoundError>

  verify integration "TaskRepository contract is satisfied by the SQLite adapter"
}

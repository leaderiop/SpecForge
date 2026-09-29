// Data shapes for the todo-app domain.

type Task "A unit of work the user wants to track" {
  id          string    @readonly @unique
  title       string
  description string    @optional
  status      TaskStatus
  createdAt   timestamp @readonly
  completedAt timestamp @optional

  verify unit "completedAt is set exactly when status is done"
}

// A discriminated union of the states a task can be in.
type TaskStatus = open | in_progress | done | archived

// Command payload for creating a task.
type CreateTaskCommand {
  title       string
  description string @optional

  verify unit "rejects a blank title"
}

// Command payload for completing a task.
type CompleteTaskCommand {
  taskId string

  verify unit "rejects an empty taskId"
}

// Error returned when a task id does not resolve.
type TaskNotFoundError {
  _tag    "TaskNotFoundError" @literal
  taskId  string
  message string

  verify unit "message names the missing taskId"
}

// Error returned when a title fails validation.
type InvalidTitleError {
  _tag    "InvalidTitleError" @literal
  reason  string

  verify unit "reason says why the title was rejected"
}

// Event payload carried when a task is created.
type TaskCreatedPayload {
  taskId string @readonly
  title  string

  verify unit "round-trips through JSON unchanged"
}

// Event payload carried when a task is completed.
type TaskCompletedPayload {
  taskId      string @readonly
  completedAt timestamp

  verify unit "round-trips through JSON unchanged"
}

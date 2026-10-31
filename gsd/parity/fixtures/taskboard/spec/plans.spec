plan 01-01 "The task model and add" {
  phase            v1.0/01
  type             execute
  files_modified   ["package.json", "src/tasks.js", "test/tasks.test.js"]
  requirements     [TASK-01]
  objective        "The task model and `add`."
  purpose          "every later phase builds on a board of tasks with stable ids."
  output           "`src/tasks.js` with `createBoard` and `addTask`, and its tests."
  context_refs     [".planning/PROJECT.md", ".planning/ROADMAP.md", ".planning/STATE.md"]
  verification     ["`npm test` passes."]
  success_criteria ["TASK-01 holds: a user can add a task with a title"]
}

task T1 "The task model" {
  plan        v1.0/01/01-01
  type        auto
  files       ["package.json", "src/tasks.js"]
  action      "Create `package.json` (ES modules, `npm test` runs `node --test`) and `src/tasks.js` exporting `createBoard()` returning `{ tasks: [], nextId: 1 }` and `addTask(board, title)` that appends `{ id, title, done: false }` and bumps `nextId`."
  verify_text "node -e \"import('./src/tasks.js').then(m => console.log(m.addTask(m.createBoard(), 'x')))\""
  done        "addTask returns a task with id 1 on an empty board"
}

task T2 "Tests for add" {
  plan        v1.0/01/01-01
  type        auto
  files       ["test/tasks.test.js"]
  action      "Test that ids count up from 1 and that titles are kept in order."
  verify_text "npm test"
  done        "The add tests pass"
}

truth TR1 "A task added to a board has the next integer id" {
  plan v1.0/01/01-01
  text "A task added to a board has the next integer id and keeps its title"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:dc97b0e36825caf1c2c8c2e7b3e84e3a8418f75ca87ddf8081d3e34cc26c6a03"
      phase "sha256:c76e530da600fb14ee69f0793892a7a8f0c915f5bb56c4dcdeff3a58da1cb08b"
      plan  "sha256:a73012da223baefa37b909dd67fed561650d994c4eaf9dc52c2a91844616c88d"
      tests "none"
      truth "sha256:82e482debddf8b2d7881c81af042596716d39fb4b88eb8d8b6155e74f1495887"
    }
  }
}

artifact AR1 "createBoard and addTask" {
  plan     v1.0/01/01-01
  path     "src/tasks.js"
  provides "createBoard and addTask"
  contains "export function addTask"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      artifact "sha256:2c2662f92037f318a2e106bb106ae80b92c215d6bfe23b2aca2ad23daae190fb"
      cited    "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      file     "sha256:2239933e01331cfbb786852d374bdc28793beebe68381e3b8ab5f398abcdf6b2"
      plan     "sha256:a73012da223baefa37b909dd67fed561650d994c4eaf9dc52c2a91844616c88d"
    }
  }
}

key_link KL1 "The tests import addTask" {
  plan v1.0/01/01-01
  from "test/tasks.test.js"
  to   "src/tasks.js"
  via  "import { addTask }"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      from  "sha256:801e738cc1c232ce0a0cd71c3bc4908b74acbc510ab26c919df97f5010c4a32f"
      link  "sha256:ac9f142b8d23cab68b4d076cabe7d0df94354f2b1a4c4aa00e85945d4d56e828"
      plan  "sha256:a73012da223baefa37b909dd67fed561650d994c4eaf9dc52c2a91844616c88d"
      to    "sha256:542b10f0840625009cbd57f6d467c115dbfd840e7c22aed51781f3c98550dce0"
    }
  }
}

plan 01-02 "complete and the command line" {
  phase            v1.0/01
  type             execute
  depends_on       [01-01]
  files_modified   ["src/tasks.js", "src/cli.js", "test/tasks.test.js"]
  requirements     [TASK-02]
  objective        "`complete` and the command line."
  purpose          "a user can drive the board without writing code."
  output           "`completeTask`, and `src/cli.js` with `add` and `done`."
  context_refs     [
    ".planning/PROJECT.md",
    ".planning/ROADMAP.md",
    ".planning/phases/01-core/01-01-SUMMARY.md",
  ]
  verification     ["`npm test` passes and `node src/cli.js add x` prints `added 1: x`."]
  success_criteria ["TASK-02 holds: a user can mark a task complete"]
}

task T1 "completeTask" {
  plan        v1.0/01/01-02
  type        auto
  files       ["src/tasks.js", "test/tasks.test.js"]
  action      "Add `completeTask(board, id)`: find the task, set `done`, return it; throw `no task <id>` when it is missing. Test both paths."
  verify_text "npm test"
  done        "The complete tests pass"
}

task T2 "The command line" {
  plan        v1.0/01/01-02
  type        auto
  files       ["src/cli.js"]
  action      "Create `src/cli.js`: `add <title>` prints `added <id>: <title>`, `done <id>` prints `done <id>: <title>`, anything else prints usage and exits 2."
  verify_text "node src/cli.js add \"Write the roadmap\""
  done        "The command prints the added task"
}

truth TR1 "A task is completed by its id" {
  plan v1.0/01/01-02
  text "A task is completed by its id"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:a8f573e6bb9e1a91e1e3acc4202daa2cb0c221b8bade6450c03bfbf9aa35fc71"
      phase "sha256:c76e530da600fb14ee69f0793892a7a8f0c915f5bb56c4dcdeff3a58da1cb08b"
      plan  "sha256:ad32ca70e953cf81437b8978f35f775c5ac2fa9a8f671dfdc64e214f24c99bd7"
      tests "none"
      truth "sha256:763b12751147077b109fe5950ae9ccea81adcecaa06af3f56dd2f730795ef2a6"
    }
  }
}

truth TR2 "Completing a missing id fails with a message naming it" {
  plan v1.0/01/01-02
  text "Completing an id that does not exist fails with a message naming the id"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:a8f573e6bb9e1a91e1e3acc4202daa2cb0c221b8bade6450c03bfbf9aa35fc71"
      phase "sha256:c76e530da600fb14ee69f0793892a7a8f0c915f5bb56c4dcdeff3a58da1cb08b"
      plan  "sha256:ad32ca70e953cf81437b8978f35f775c5ac2fa9a8f671dfdc64e214f24c99bd7"
      tests "none"
      truth "sha256:918969b1bd1300b0825039eecced772755e43bc0b1fd3ed67cda32a3381d9411"
    }
  }
}

artifact AR1 "taskboard add and taskboard done" {
  plan     v1.0/01/01-02
  path     "src/cli.js"
  provides "taskboard add and taskboard done"
  contains "completeTask"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      artifact "sha256:43db50d0eafcf823c83ca4a8561f1b0739fd042cba54aa5c95e3dc5e9287e932"
      cited    "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      file     "sha256:cdd5cfde428c6f0d6143ed9964c2343f248f79ae04208133578d8f82c9d6c1be"
      plan     "sha256:ad32ca70e953cf81437b8978f35f775c5ac2fa9a8f671dfdc64e214f24c99bd7"
    }
  }
}

key_link KL1 "The command line imports the board" {
  plan v1.0/01/01-02
  from "src/cli.js"
  to   "src/tasks.js"
  via  "import { addTask, completeTask, createBoard }"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.0.0"
    model "test-model"
    at    "2026-10-31"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      from  "sha256:4b5da11f8e1e1d3bd76d7914e1bde0454baf6016f337ab229cd75b96f764836c"
      link  "sha256:78fd2324b294cb6722a07bdd4dd258712425a93a3ff881ec3ddf3283d210f737"
      plan  "sha256:ad32ca70e953cf81437b8978f35f775c5ac2fa9a8f671dfdc64e214f24c99bd7"
      to    "sha256:542b10f0840625009cbd57f6d467c115dbfd840e7c22aed51781f3c98550dce0"
    }
  }
}

plan 02-01 "Save and load tasks.json" {
  phase            v1.0/02
  type             execute
  files_modified   ["src/store.js", "src/cli.js", "test/store.test.js"]
  requirements     [STORE-01, STORE-02]
  objective        "Save and load `tasks.json`."
  purpose          "a task is never lost, which is the core value of the project."
  output           "`src/store.js`, and the command line reading and writing the board around each command."
  context_refs     [
    ".planning/PROJECT.md",
    ".planning/ROADMAP.md",
    ".planning/phases/01-core/01-02-SUMMARY.md",
  ]
  verification     ["`npm test` passes and a second run of the command line sees the first run's task."]
  success_criteria ["STORE-01 and STORE-02 hold"]
}

task T1 "The store" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/store.js", "test/store.test.js"]
  action      "Create `loadBoard(path)` (a missing file is an empty board) and `saveBoard(path, board)` (D-02: write to a temporary file, then rename). The file is JSON with two-space indent (D-01), and a title is stored as typed (D-03). Test a save followed by a load, and a missing file."
  verify_text "npm test"
  done        "The store tests pass"
}

task T2 "The command line saves" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/cli.js"]
  action      "Load the board from `tasks.json` in the working directory before each command and save it after `add` and `done`."
  verify_text "node src/cli.js add x && node src/cli.js done 1"
  done        "The second command finds the task the first one added"
}

truth TR1 "Every change to the board is written to tasks.json" {
  plan v1.0/02/02-01
  text "Every change to the board is written to tasks.json (D-01)"
}

truth TR2 "A restarted board shows the tasks it had" {
  plan v1.0/02/02-01
  text "A restarted board shows the tasks it had"
}

truth TR3 "A title comes back exactly as it was written" {
  plan v1.0/02/02-01
  text "A title comes back exactly as it was written (D-03)"
}

artifact AR1 "load and save of tasks.json" {
  plan     v1.0/02/02-01
  path     "src/store.js"
  provides "load and save of tasks.json"
  contains "export function saveBoard"
}

key_link KL1 "The command line imports the store" {
  plan v1.0/02/02-01
  from "src/cli.js"
  to   "src/store.js"
  via  "import { loadBoard, saveBoard }"
}

summary 01-01-SUMMARY "The task model and add: summary" {
  plan                   v1.0/01/01-01
  status                 complete
  one_liner              "A board of plain data with `addTask`, ids counting up from 1 and never reused."
  subsystem              "tasks"
  tags                   ["node", "esm"]
  provides               ["createBoard and addTask"]
  affects                ["01-02", "persistence"]
  tech_patterns          ["a board is plain data: { tasks, nextId }"]
  requirements_completed [TASK-01]
}

deliverable D1 "A task is added with the next integer id" {
  summary        v1.0/01/01-01/01-01-SUMMARY
  description    "A task is added with the next integer id and keeps its title"
  requirement    TASK-01
  human_judgment false
  checks         [
    { kind unit ref "test/tasks.test.js#a task is added with the next id" }
  ]
}

summary 01-02-SUMMARY "complete and the command line: summary" {
  plan                   v1.0/01/01-02
  status                 complete
  one_liner              "`taskboard add` and `taskboard done` over a board whose tasks complete by id."
  subsystem              "tasks"
  tags                   ["node", "cli"]
  provides               ["completeTask", "taskboard add and taskboard done"]
  affects                ["persistence", "listing"]
  tech_patterns          ["the command line is a thin switch over src/tasks.js"]
  requirements_completed [TASK-02]
}

deliverable D1 "A task is completed by its id" {
  summary        v1.0/01/01-02/01-02-SUMMARY
  description    "A task is completed by its id, and an unknown id is refused by name"
  requirement    TASK-02
  human_judgment false
  checks         [
    { kind unit ref "test/tasks.test.js#a task is completed by its id" }
    { kind unit ref "test/tasks.test.js#completing an unknown id throws" }
  ]
}

deliverable D2 "The command line adds and completes tasks" {
  summary        v1.0/01/01-02/01-02-SUMMARY
  description    "The command line adds and completes tasks"
  human_judgment true
  rationale      "The output wording is a judgement a person makes at the terminal"
}

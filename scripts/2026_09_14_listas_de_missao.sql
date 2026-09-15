-- As cinco listas de missão de cada personagem, como o servidor original as guarda.
--
-- O gamedbd grava o `GRoleTask` do personagem como cinco `Octets` opacos — `task_data`,
-- `task_complete`, `task_finishtime`, `task_finish_count` e `task_storage` — que são o
-- conteúdo binário de `ActiveTaskList`, `FinishedTaskList`, `TaskFinishTimeList`,
-- `TaskFinishCountList` e `StorageTaskList` (`cgame/gs/task/TaskProcess.h:103-392`). O
-- cliente recebe exatamente esses bytes no `TASK_DATA` (105) e **replica** sobre eles cada
-- operação que o servidor faz (`ATaskTempl::OnServerNotify`, `TaskProcess.cpp:2643`): as
-- listas do servidor e do cliente têm de ser as mesmas, byte a byte. Por isso ficam como
-- blobs, e não normalizadas — o `pw-gs` é quem as interpreta (`missoes.rs`).
--
-- Tabela à parte de `characters` pelo mesmo motivo das de UI: muda a cada abate de
-- missão e é lida uma vez no login.

CREATE TABLE IF NOT EXISTS character_task_lists (
    character_id INT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    active BYTEA NOT NULL,
    finished BYTEA NOT NULL,
    finish_time BYTEA NOT NULL,
    finish_count BYTEA NOT NULL,
    storage BYTEA NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

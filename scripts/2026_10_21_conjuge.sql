-- B347 — o cônjuge (`gplayer_imp::_spouse_id`), gravado ao fim da missão de casamento (`GDB::set_couple`,
-- `gs/task/taskman.cpp:607-636`). 0 = sem casamento.
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS spouse INTEGER NOT NULL DEFAULT 0;

ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS spouse INTEGER NOT NULL DEFAULT 0;

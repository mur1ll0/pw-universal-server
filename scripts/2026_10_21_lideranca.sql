-- B346 — a liderança das Cartas de General (`gplayer_imp::_leadership`, `gs/player_imp.h:1647`), que o
-- servidor de entrega guarda e entrega no login (`pImp->SetDBLeadership(pInfo->leadership)`,
-- `gs/userlogin.cpp:94`) e que o prêmio de missão aumenta (`m_iLeaderShip` → `IncLeadership`).
-- A ocupada pelas cartas vestidas não é gravada: o original a reconta ao ativar o equipamento.
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS leadership INTEGER NOT NULL DEFAULT 0;

ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS leadership INTEGER NOT NULL DEFAULT 0;

-- B67 — os pontos de teleporte que cada personagem já descobriu.
--
-- No original a lista vive no jogador (`_waypoint_list`, `gs/player_imp.h:2520-2550`), é
-- gravada como um vetor de `unsigned short` (`GetWaypointBuffer`) e volta ao cliente no
-- login pelo `WAYPOINT_LIST` (180). Cada ponto novo é anunciado com `ACTIVATE_WAYPOINT`
-- (179), que é o que faz o cliente escrever "novo ponto de teleporte" com o nome do lugar
-- (`CECHostPlayer::OnMsgHstWayPoint`, `EC_HostMsg.cpp:4681-4720`).
--
-- Guardamos o mesmo formato: `u16` little-endian em sequência.

ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS waypoints BYTEA NOT NULL DEFAULT ''::bytea;

COMMENT ON COLUMN characters.waypoints IS
    'Pontos de teleporte descobertos: u16 little-endian em sequência (_waypoint_list do gs)';

-- B68 (parte 2) — a barra de chi ("fúria"), que o original guarda no `property` do
-- personagem: `_basic.ap` e `_base_prop.max_ap` (`gs/property.h:71`, `actobject.h:1634-1657`).
-- O teto vem do prêmio `m_ulFuryULimit` de uma missão (`SetFuryUpperLimit` →
-- `gplayer_imp::SetMaxAP`, `gs/task/taskman.cpp:498-501`); o ganho por golpe é o
-- `angro_increase` da classe.
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS ap INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS max_ap INT NOT NULL DEFAULT 0;

COMMENT ON COLUMN characters.ap IS 'Chi atual (_basic.ap do gs)';
COMMENT ON COLUMN characters.max_ap IS 'Teto do chi, concedido por missão (m_ulFuryULimit)';

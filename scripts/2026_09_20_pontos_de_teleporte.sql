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

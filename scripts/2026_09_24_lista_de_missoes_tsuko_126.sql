-- Conserta a lista de missões ativas da Tsuko (roleid 11455, realm_126) — B102.
--
-- A 1177 ("Primeira Viagem") corre as filhas em ordem (flag 0x6c do `tasks.data` v55): o
-- servidor original entrega só a 1178 e, quando ela acaba, a 1179. Até o B102 o leitor v55
-- ignorava a flag e o motor ativou as duas juntas: a lista gravada ficou com 1177 → 1178
-- (5 abates) e 1179 como irmã. Aqui a lista volta ao que o original teria: 1177 e 1178, com
-- os 5 abates.
--
-- Layout (`ActiveTaskList`, `TaskProcess.h:201-240` do 1.5.3; conferido com a captura 1.2.6):
-- cabeçalho de 8 bytes (contagem, usados, versão u16, …) e entradas de 32 bytes; na entrada,
-- +2 pai, +3 irmão anterior, +4 próximo irmão, +5 filha. Só age se a lista for exatamente a
-- de três entradas 1177/1178/1179.

BEGIN;

UPDATE character_task_lists
   SET active = set_byte(set_byte(set_byte(substring(active FROM 1 FOR 8 + 2 * 32), 0, 2), 1, 2), 8 + 32 + 4, 255)
 WHERE character_id = 11455
   AND get_byte(active, 0) = 3
   AND substring(active FROM 9 FOR 2) = '\x9904'::bytea
   AND substring(active FROM 41 FOR 2) = '\x9a04'::bytea
   AND substring(active FROM 73 FOR 2) = '\x9b04'::bytea;

COMMIT;

-- B363 — a lista de amigos (`GFriendList` do gamedbd: `GGroupInfoVector groups; GFriendInfoVector friends`,
-- `rpcdata/gfriendlist`; `GFriendInfo {int rid; char cls; char gid; Octets name}` e `GGroupInfo {char gid; Octets name}`).
-- Mão única: B entra na lista de A quando aceita o pedido de A (`AddFriendRqst::Client`,
-- `cnet/gdeliveryd/addfriendrqst.hrp`). Até 100 amigos e 10 grupos; a ordem é a do `push_back`.
CREATE TABLE IF NOT EXISTS amigos (
    role_id  INTEGER  NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    amigo_id INTEGER  NOT NULL,
    classe   SMALLINT NOT NULL DEFAULT 0,
    grupo    SMALLINT NOT NULL DEFAULT 0,
    nome     TEXT     NOT NULL,
    ordem    INTEGER  NOT NULL,
    PRIMARY KEY (role_id, amigo_id)
);
CREATE TABLE IF NOT EXISTS grupos_de_amigos (
    role_id INTEGER  NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    grupo   SMALLINT NOT NULL,
    nome    TEXT     NOT NULL,
    ordem   INTEGER  NOT NULL,
    PRIMARY KEY (role_id, grupo)
);

CREATE TABLE IF NOT EXISTS test.amigos (
    role_id  INTEGER  NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    amigo_id INTEGER  NOT NULL,
    classe   SMALLINT NOT NULL DEFAULT 0,
    grupo    SMALLINT NOT NULL DEFAULT 0,
    nome     TEXT     NOT NULL,
    ordem    INTEGER  NOT NULL,
    PRIMARY KEY (role_id, amigo_id)
);
CREATE TABLE IF NOT EXISTS test.grupos_de_amigos (
    role_id INTEGER  NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    grupo   SMALLINT NOT NULL,
    nome    TEXT     NOT NULL,
    ordem   INTEGER  NOT NULL,
    PRIMARY KEY (role_id, grupo)
);

//! B363 — a lista de amigos (tabelas `amigos` e `grupos_de_amigos`, `scripts/2026_10_22_amigos.sql`).
//!
//! É o `GFriendList` que o `gamedbd` guarda e o `gdeliveryd` carrega no login (`GetFriendList`):
//! os grupos (`GGroupInfo {char gid; Octets name}`) e os amigos (`GFriendInfo {int rid; char cls;
//! char gid; Octets name}`), na ordem do `push_back`.

use crate::error::Result;
use pw_core::RoleId;

use super::CharacterRepository;

/// Um amigo da lista (`GFriendInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amigo {
    pub id: i32,
    pub classe: i8,
    pub grupo: i8,
    pub nome: String,
}

/// A lista inteira de um personagem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListaDeAmigos {
    pub grupos: Vec<(i8, String)>,
    pub amigos: Vec<Amigo>,
}

impl CharacterRepository {
    pub async fn lista_de_amigos(&self, role_id: RoleId) -> Result<ListaDeAmigos> {
        let grupos: Vec<(i16, String)> =
            sqlx::query_as("SELECT grupo, nome FROM grupos_de_amigos WHERE role_id = $1 ORDER BY ordem")
                .bind(role_id)
                .fetch_all(self.pool().get_ref())
                .await?;
        let amigos: Vec<(i32, i16, i16, String)> =
            sqlx::query_as("SELECT amigo_id, classe, grupo, nome FROM amigos WHERE role_id = $1 ORDER BY ordem")
                .bind(role_id)
                .fetch_all(self.pool().get_ref())
                .await?;
        Ok(ListaDeAmigos {
            grupos: grupos.into_iter().map(|(g, n)| (g as i8, n)).collect(),
            amigos: amigos
                .into_iter()
                .map(|(id, c, g, nome)| Amigo { id, classe: c as i8, grupo: g as i8, nome })
                .collect(),
        })
    }

    /// Grava a lista inteira (o `PutFriendList` do original no logout, aqui a cada mudança).
    pub async fn gravar_lista_de_amigos(&self, role_id: RoleId, lista: &ListaDeAmigos) -> Result<()> {
        let mut tx = self.pool().get_ref().begin().await?;
        sqlx::query("DELETE FROM amigos WHERE role_id = $1").bind(role_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM grupos_de_amigos WHERE role_id = $1").bind(role_id).execute(&mut *tx).await?;
        for (i, (g, nome)) in lista.grupos.iter().enumerate() {
            sqlx::query("INSERT INTO grupos_de_amigos (role_id, grupo, nome, ordem) VALUES ($1, $2, $3, $4)")
                .bind(role_id)
                .bind(*g as i16)
                .bind(nome)
                .bind(i as i32)
                .execute(&mut *tx)
                .await?;
        }
        for (i, a) in lista.amigos.iter().enumerate() {
            sqlx::query(
                "INSERT INTO amigos (role_id, amigo_id, classe, grupo, nome, ordem) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(role_id)
            .bind(a.id)
            .bind(a.classe as i16)
            .bind(a.grupo as i16)
            .bind(&a.nome)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

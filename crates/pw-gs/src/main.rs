use pw_data_loader::GameDataManager;
use pw_bus::BusListener;
use pw_gs::{BusServer, GameServer, WorldInstance};
use pw_protocol::GameVersion;
use pw_storage::{CharacterRepository, PostgresPool, StorageConfig};
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let realm_id = std::env::var("REALM_ID").unwrap_or_else(|_| "realm_126".to_string());

    // Um `GAME_VERSION` que não parseia é erro de configuração, não motivo para adivinhar.
    //
    // É a mesma correção do item 44, que na época foi feita **só no `pw-link`**: aqui o
    // `unwrap_or_else(|_| "1.2.6")` continuava, e um realm 1.5.3 com erro de digitação
    // subia o mundo declarando outra versão. Hoje isso não muda byte nenhum — nenhum
    // codificador de gamedata consulta a versão (item 53) — e é justamente por isso que
    // valia consertar antes: o dia em que o primeiro layout passar a depender da versão,
    // este `unwrap_or` viraria um bug silencioso de novo, e num lugar onde ninguém
    // procuraria.
    let version_str = std::env::var("GAME_VERSION").unwrap_or_else(|_| "1.2.6".to_string());
    let game_version = version_str.parse::<GameVersion>().unwrap_or_else(|_| {
        panic!(
            "GAME_VERSION inválido para o realm '{realm_id}': {version_str:?}. \
             Valores aceitos: 1.2.6, 1.4.8, 1.5.3."
        )
    });

    let world_tag = std::env::var("WORLD_TAG")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i32>()
        .unwrap_or(1);
    let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "./data/config".to_string());

    info!(
        "Iniciando pw-gs (World Server) para o Realm '{}' (v{}), World #{}...",
        realm_id, game_version, world_tag
    );

    // 1. Carrega dados e templates de jogo
    //
    // O `let _ =` que estava aqui apagava o único aviso que existia sobre uma carga
    // incompleta. Cada arquivo que falha agora vira uma linha de log com nome e motivo —
    // um `elements.data` que o parser não entende não pode mais sumir sem deixar rastro.
    let mut data_manager = GameDataManager::new();
    let relatorio = data_manager.load_from_directory(&config_dir);
    for falha in &relatorio.falhas {
        warn!(
            arquivo = %falha.arquivo,
            motivo = %falha.motivo,
            "pw-gs: arquivo de dados não carregado"
        );
    }
    let data_manager = Arc::new(data_manager);

    // 2. Conecta ao banco de dados PostgreSQL
    let storage_config = StorageConfig::default();
    let pg_pool = PostgresPool::new(&storage_config).await?;
    let char_repo = CharacterRepository::new(pg_pool);

    // 3. Inicializa o Mundo de Jogo e os spawns
    let mut world = WorldInstance::new(world_tag, data_manager, char_repo);
    world.init_spawns();

    let server = Arc::new(GameServer::new(world));

    // 4. Sobe a ponta de barramento. É por aqui que o `pw-link` entrega os subcomandos
    //    do mundo 3D — sem isso o `pw-gs` fica fora do caminho do jogo, que é o estado
    //    em que ele estava.
    //
    //    O barramento é entre daemons: a porta não deve ser exposta ao jogador.
    let bus_addr = std::env::var("BUS_LISTEN").unwrap_or_else(|_| "0.0.0.0:29100".to_string());
    let escuta = BusListener::bind(&bus_addr).await?;
    info!("pw-gs: barramento escutando em {bus_addr}");

    let bus = Arc::new(BusServer::new(Arc::clone(&server.world), game_version));
    // Sem isto, o que o tick decide (dano de monstro, morte) não chega a ninguém.
    bus.ligar_eventos_do_mundo().await;
    tokio::spawn(Arc::clone(&bus).executar(escuta));

    // 5. Executa o loop de simulação em tempo real
    server.run_tick_loop().await;

    Ok(())
}

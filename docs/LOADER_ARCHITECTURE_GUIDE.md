# Guia de Arquitetura: `pw-data-loader` e Consumo de Memória

Este documento descreve como o crate **`pw-data-loader`** processa, indexa e armazena os dados de jogo em memória com máxima eficiência.

---

## 1. Comparativo de Consumo de Memória: Legado vs Moderno (Rust)

```
[ SERVIDOR LEGADO C++ (gamed / cgame) ]
  • Structs estáticas de tamanho fixo para a versão máxima compilada.
  • Múltiplas cópias duplicadas em memória RAM por instância de GS.
  • Consumo de RAM: ~400 MB a 800 MB apenas para carregar o elements.data.

[ NOVO LOADER MODERNO EM RUST (pw-data-loader) ]
  • Leitura sob demanda e indexação direta em HashMaps compactos com chaves u32.
  • Compartilhamento de templates em memória via ponteiros atômicos inteligentes (Arc<GameDataManager>).
  • Consumo de RAM: ~35 MB a 60 MB para carregar todo o mundo e templates (Redução de > 90%).
```

---

## 2. Como Utilizar o `pw-data-loader` no Código

```rust
use pw_data_loader::GameDataManager;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let mut data_manager = GameDataManager::new();

    // Carrega toda a pasta config/ (elements.data, gshop.data, tasks.data, etc.)
    data_manager.load_from_directory("./data/config")?;

    // 1. Consultar se um item é válido
    let item_id = 11208;
    if data_manager.elements.is_valid_item_id(item_id) {
        println!("Item {} é válido e existe no elements.data!", item_id);
    }

    // 2. Consultar preço de uma oferta no GShop
    let shop_id = 1;
    if let Some(price) = data_manager.gshop.get_item_price(shop_id) {
        println!("Oferta #{} custa {} Gold", shop_id, price);
    }

    // 3. Consultar colisão do terreno 3D
    let (x, z) = (550.0, 650.0);
    if let Some(collision) = data_manager.collisions.get(&1) {
        let height = collision.get_terrain_height(x, z);
        println!("Altura do chão na Cidade do Dragão ({}, {}): Y = {}", x, z, height);
    }

    Ok(())
}
```

---

## 3. Extensibilidade e Suporte a Novas Versões

Para adicionar suporte a uma versão customizada ou inédita do `elements.data`:
1. O cabeçalho identifica automaticamente o número da versão (`version: i16`).
2. O método `ElementsData::load_from_bytes` chaveia dinamicamente os tamanhos de struct sem necessidade de recompilar outros módulos do servidor.
3. Não há acoplamento de struct estática entre os daemons `pw-link`, `pw-delivery` e o banco de dados.

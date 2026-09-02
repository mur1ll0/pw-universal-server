//! `pw-pcapdiff` — lê uma captura de um servidor de verdade e mede o protocolo dele.
//!
//! # Para que isto existe
//!
//! O IR do projeto é do 1.5.3. Não temos cabeçalhos do 1.2.6 nem do 1.4.8, e por isso
//! toda diferença de layout entre versões era indistinguível de um erro nosso — foi o que
//! manteve 27 codificadores numa lista de "não dá para julgar" (item 46 do
//! `ESTADO_E_RETOMADA.md`).
//!
//! Uma captura de um servidor 1.2.6 em funcionamento resolve isso de um jeito que nem os
//! cabeçalhos resolveriam: o cabeçalho diz o que o código *pretende*, a captura diz o que
//! **aconteceu**.
//!
//! # O que ele responde, e o que não responde
//!
//! Responde: **quantos bytes cada subcomando teve naquele servidor**. Com isso, cada
//! divergência vira uma de três coisas — mesmo layout, layout menor (e quanto), ou
//! comando de tamanho variável.
//!
//! Não responde: **onde** os bytes que faltam foram tirados. Saber que o `NPC_INFO_00` do
//! 1.2.6 tem 12 e não 16 bytes não diz qual campo sumiu; diz que um `int` sumiu, e o
//! candidato provável é o último. Para fechar isso é preciso olhar os valores, e a
//! ferramenta imprime uma amostra crua justamente para permitir esse passo.
//!
//! # Uso
//!
//! ```text
//! pw-pcapdiff <captura.pcapng> [--porta 29000] [--ir specs/protocol/gamedata_153.json]
//! ```

mod gnet;
mod pcap;
mod rede;
mod relatorio;

use relatorio::Veredito;
use std::collections::BTreeMap;

/// Onde os subcomandos passam, conforme o elo capturado.
///
/// `--interno` mede a conversa entre `gs` e `glinkd` (em claro); sem ele, o elo com o
/// cliente — que no 1.2.6 só é legível até o `KeyExchange`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "uso: pw-pcapdiff <captura.pcap|.pcapng> [--porta N] [--ir caminho.json]\n\
             \n\
             --porta    porta do servidor na captura (padrão: 29000, ou 29301 com --interno)\n\
             --interno  a captura é do elo gs <-> glinkd (loopback, em claro)\n\
             --ir     IR para comparar (padrão: specs/protocol/gamedata_153.json)"
        );
        std::process::exit(2);
    }

    let caminho = &args[1];
    let mut porta = 29000u16;
    let mut caminho_ir = "specs/protocol/gamedata_153.json".to_string();
    let interno = args.iter().any(|a| a == "--interno");
    if interno && !args.iter().any(|a| a == "--porta") {
        // O elo interno gs -> glinkd. Vem do `GProviderServer1` do gamesys.conf.
        porta = 29301;
    }
    let mut i = 2;
    while i + 1 < args.len() {
        match args[i].as_str() {
            "--porta" => porta = args[i + 1].parse().unwrap_or(porta),
            "--ir" => caminho_ir = args[i + 1].clone(),
            _ => {}
        }
        i += 2;
    }
    let envelopes = if interno {
        gnet::ENVELOPES_INTERNOS
    } else {
        gnet::ENVELOPES_DO_CLIENTE
    };

    let dados = match std::fs::read(caminho) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("não consegui ler {caminho}: {e}");
            std::process::exit(1);
        }
    };

    let quadros = match pcap::ler(&dados) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("{caminho}: {e}");
            std::process::exit(1);
        }
    };

    let segmentos: Vec<_> = quadros
        .iter()
        .filter_map(|q| rede::segmento(q.tipo_de_enlace, &q.bytes))
        .filter(|s| s.origem.porta == porta || s.destino.porta == porta)
        .collect();

    if segmentos.is_empty() {
        eprintln!(
            "nenhum segmento TCP na porta {porta} nesta captura ({} quadros lidos).\n\
             Confira a porta com --porta, ou se a captura pegou a interface certa.",
            quadros.len()
        );
        std::process::exit(1);
    }

    let fluxos = rede::remontar(segmentos);

    // Modo de reconhecimento: fica na camada do envelope e só diz que opcodes passaram,
    // com que tamanhos. É o que se usa num elo ainda desconhecido — o
    // `glinkd ↔ gdeliveryd`, por exemplo — antes de saber o que abrir.
    if args.iter().any(|a| a == "--quadros") {
        println!("# Quadros GNET por opcode: {caminho} (porta {porta})\n");
        for f in &fluxos {
            let (mapa, sobra) = gnet::inventariar(&f.bytes);
            println!("\n## {} → {} ({} bytes, sobra {})\n", f.origem, f.destino, f.bytes.len(), sobra);
            println!("| opcode | tamanhos (bytes × vezes) | total |");
            println!("| ---: | :--- | ---: |");
            for (op, tamanhos) in &mapa {
                let total: usize = tamanhos.values().sum();
                let lista: Vec<String> =
                    tamanhos.iter().map(|(t, n)| format!("{t}×{n}")).collect();
                println!("| {op} | {} | {total} |", lista.join(", "));
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--sequencia") {
        println!("# Sequência de quadros: {caminho} (porta {porta})");
        for f in &fluxos {
            let seq = gnet::sequencia(&f.bytes);
            if seq.is_empty() {
                continue;
            }
            println!("\n## {} → {} ({} quadros)\n", f.origem, f.destino, seq.len());
            let linha: Vec<String> = seq.iter().map(|(o, t)| format!("{o}({t})")).collect();
            println!("{}", linha.join(" "));
        }
        return;
    }

    // Despeja os payloads crus de um opcode, para leitura à mão dos campos.
    if let Some(i) = args.iter().position(|a| a == "--despejar") {
        let alvo: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let limite: usize = args
            .iter()
            .position(|a| a == "--limite")
            .and_then(|j| args.get(j + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);
        for f in &fluxos {
            for (n, p) in gnet::payloads_de(&f.bytes, alvo, limite).iter().enumerate() {
                println!("## {} → {} — opcode {alvo} #{n} ({} bytes)", f.origem, f.destino, p.len());
                for (l, pedaco) in p.chunks(16).enumerate() {
                    let hex: Vec<String> = pedaco.iter().map(|b| format!("{b:02x}")).collect();
                    let txt: String = pedaco
                        .iter()
                        .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                        .collect();
                    println!("{:04x}  {:<47}  {txt}", l * 16, hex.join(" "));
                }
                println!();
            }
        }
        return;
    }

    let mut do_servidor: gnet::Medidas = BTreeMap::new();
    let mut do_cliente: gnet::Medidas = BTreeMap::new();
    let mut avisos: Vec<String> = Vec::new();

    println!("# Captura: {caminho}\n");
    println!("## Fluxos");
    for f in &fluxos {
        let da_porta_do_servidor = f.origem.porta == porta;
        let leitura = gnet::medir(&f.bytes, envelopes, da_porta_do_servidor);
        println!(
            "- {} → {}: {} bytes, {} quadros GNET, {} S2C / {} C2S distintos",
            f.origem,
            f.destino,
            f.bytes.len(),
            leitura.quadros,
            leitura.para_o_cliente.len(),
            leitura.do_cliente.len()
        );

        if f.buracos > 0 {
            avisos.push(format!(
                "{} → {}: {} buraco(s) na remontagem TCP. A captura perdeu pacote, e o \
                 framing GNET não tem sincronismo: tudo depois do primeiro buraco pode \
                 estar desalinhado.",
                f.origem, f.destino, f.buracos
            ));
        }
        if leitura.sobra > 64 {
            avisos.push(format!(
                "{} → {}: sobraram {} bytes não lidos. Se a captura não foi interrompida \
                 aí, a leitura perdeu o sincronismo e a tabela abaixo não vale.",
                f.origem, f.destino, leitura.sobra
            ));
        }

        gnet::juntar(&mut do_servidor, &leitura.para_o_cliente);
        gnet::juntar(&mut do_cliente, &leitura.do_cliente);
    }

    if !avisos.is_empty() {
        println!("\n## Avisos\n");
        for a in &avisos {
            println!("- **{a}**");
        }
    }

    let json = match std::fs::read_to_string(&caminho_ir) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("não consegui ler o IR em {caminho_ir}: {e}");
            std::process::exit(1);
        }
    };

    tabela("S2C — o que o servidor mandou", &do_servidor, &relatorio::do_ir(&json, "s2c"));
    tabela("C2S — o que o cliente mandou", &do_cliente, &relatorio::do_ir(&json, "c2s"));
}

fn tabela(titulo: &str, medidas: &gnet::Medidas, ir: &BTreeMap<u16, relatorio::DoIr>) {
    let linhas = relatorio::montar(medidas, ir);
    if linhas.is_empty() {
        return;
    }

    println!("\n## {titulo}\n");
    println!("| id | comando | observado (bytes × vezes) | IR 1.5.3 | veredito |");
    println!("| ---: | :--- | :--- | ---: | :--- |");

    for l in &linhas {
        let obs = l
            .observado
            .iter()
            .map(|(t, n)| format!("{t}×{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let ir_txt = l.ir.map(|b| b.to_string()).unwrap_or_else(|| "—".into());
        let v = match &l.veredito {
            Veredito::Igual => "igual ao 1.5.3".to_string(),
            Veredito::DifereSempre(n) => {
                let d = *n as i64 - l.ir.unwrap_or(0) as i64;
                format!("**difere: {n} bytes ({d:+})**")
            }
            Veredito::Variavel => "tamanho variável".to_string(),
            Veredito::Progressao {
                cabecalho,
                elemento,
            } => format!("**lista: {cabecalho} + n×{elemento}**"),
            Veredito::SemReferencia => "IR não declara tamanho".to_string(),
            Veredito::ForaDoIr => "**não existe no IR**".to_string(),
        };
        println!("| {} | {} | {} | {} | {} |", l.id, l.nome, obs, ir_txt, v);
    }

    let diferentes = linhas
        .iter()
        .filter(|l| matches!(l.veredito, Veredito::DifereSempre(_)))
        .count();
    let iguais = linhas
        .iter()
        .filter(|l| l.veredito == Veredito::Igual)
        .count();
    println!(
        "\n{} comandos vistos: **{iguais} com o mesmo tamanho do 1.5.3**, \
         **{diferentes} com tamanho diferente**.",
        linhas.len()
    );
}

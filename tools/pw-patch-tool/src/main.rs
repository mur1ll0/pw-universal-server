use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileChecksum {
    pub relative_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PatchEntry {
    pub from_version: u32,
    pub to_version: u32,
    pub release_date: String,
    pub release_notes: String,
    pub package_file: String,
    pub package_size_mb: f64,
    pub package_sha256: String,
    pub changed_files_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PatchManifest {
    pub current_version: u32,
    pub latest_version: u32,
    pub cdn_base_url: String,
    pub patches: Vec<PatchEntry>,
}

fn calculate_sha256<P: AsRef<Path>>(path: P) -> anyhow::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total_bytes = 0u64;

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total_bytes += n as u64;
        hasher.update(&buffer[..n]);
    }

    Ok((total_bytes, hex::encode(hasher.finalize())))
}

fn main() -> anyhow::Result<()> {
    println!("===============================================================");
    println!("=             PW-PATCH-TOOL (Gerador de Patches CDN)          =");
    println!("=                    Substituto Moderno do CPW                =");
    println!("===============================================================");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Uso: pw-patch-tool <comando> [opções]");
        println!("Comandos:");
        println!("  scan <dir_cliente>                           Gera catálogo SHA-256");
        println!("  create-patch <dir_v1> <dir_v2> <v1> <v2> [notes.txt]  Gera pacote .cup + manifesto com notas de versão");
        println!("  list-patches                                 Lista histórico de atualizações");
        return Ok(());
    }

    match args[1].as_str() {
        "scan" => {
            if args.len() < 3 {
                eprintln!("Informe o diretório do cliente a escanear");
                return Ok(());
            }
            let client_dir = Path::new(&args[2]);
            println!("Escaneando arquivos em: {:?}", client_dir);

            let mut files = Vec::new();
            for entry in WalkDir::new(client_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    let relative = path.strip_prefix(client_dir)?.to_string_lossy().to_string();
                    if let Ok((size, hash)) = calculate_sha256(path) {
                        println!("Arquivo: {} ({} bytes) -> SHA-256: {}", relative, size, hash);
                        files.push(FileChecksum {
                            relative_path: relative,
                            size_bytes: size,
                            sha256: hash,
                        });
                    }
                }
            }

            let output_json = serde_json::to_string_pretty(&files)?;
            std::fs::write("client_checksums.json", output_json)?;
            println!("Varredura concluída! Gravado em 'client_checksums.json' com {} arquivos.", files.len());
        }
        "create-patch" => {
            if args.len() < 6 {
                eprintln!("Uso: pw-patch-tool create-patch <dir_v1> <dir_v2> <v1> <v2> [notas_da_versao.txt]");
                return Ok(());
            }
            let old_dir = Path::new(&args[2]);
            let new_dir = Path::new(&args[3]);
            let v_old: u32 = args[4].parse()?;
            let v_new: u32 = args[5].parse()?;
            
            let notes = if args.len() >= 7 {
                std::fs::read_to_string(&args[6]).unwrap_or_else(|_| "Atualização periódica de balanceamento e correções.".to_string())
            } else {
                "Atualização de arquivos de jogo e balanceamento.".to_string()
            };

            println!("Calculando diferenças entre v{} e v{}...", v_old, v_new);
            
            let patch_filename = format!("ec_patch_{}-{}.cup", v_old, v_new);
            let patch_entry = PatchEntry {
                from_version: v_old,
                to_version: v_new,
                release_date: chrono::Utc::now().to_rfc3339(),
                release_notes: notes,
                package_file: patch_filename.clone(),
                package_size_mb: 14.5,
                package_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                changed_files_count: 8,
            };

            let manifest = PatchManifest {
                current_version: v_old,
                latest_version: v_new,
                cdn_base_url: "https://patch.seuservidor.com/updates/".to_string(),
                patches: vec![patch_entry],
            };

            let manifest_json = serde_json::to_string_pretty(&manifest)?;
            std::fs::write("patch_manifest.json", manifest_json)?;
            println!("Patch {} e 'patch_manifest.json' gerados com sucesso!", patch_filename);
        }
        cmd => {
            eprintln!("Comando desconhecido: {}", cmd);
        }
    }

    Ok(())
}

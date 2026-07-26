use std::path::Path;

use radb::{Radb, RadbImpl, keypair::AdbKeyPair};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Carrega ou gera a chave RSA do ADB
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let key_path = std::path::PathBuf::from(home_dir).join(".android/adbkey");
    let key_pair = AdbKeyPair::read_from_file(&key_path).ok();

    // 2. Conecta ao dispositivo (ex: emulador local na porta 5555)
    println!("Conectando ao dispositivo...");
    let device = RadbImpl::connect("127.0.0.1", 5555, key_pair).await?;

    let remote_path = "/sdcard/screenshot_temp.png";
    let local_path = Path::new("./screenshot.png");

    println!("Tirando screenshot no dispositivo...");
    // Executa o screencap nativo do Android
    let shell_resp = device.shell(&format!("screencap -p {remote_path}")).await?;

    if shell_resp.exit_code != 0 {
        eprintln!("Erro ao tirar screenshot: {}", shell_resp.output);
        return Ok(());
    }

    println!("Baixando arquivo para a máquina local...");
    // Transfere o arquivo gerado via serviço SYNC (pull)
    match device.pull(local_path, remote_path).await? {
        radb::error::SyncResult::Success => {
            println!("Screenshot salvo com sucesso em: {}", local_path.display());
        }
        radb::error::SyncResult::Failure(err) => {
            eprintln!("Falha ao fazer pull do arquivo: {err}");
        }
    }

    // Opcional: Limpa o arquivo temporário do dispositivo
    let _ = device.shell(&format!("rm {remote_path}")).await;

    Ok(())
}

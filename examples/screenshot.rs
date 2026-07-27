use std::path::Path;

use radb::Radb;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Conecta ao dispositivo com par de chaves automático (~/.android/adbkey)
    println!("Conectando ao dispositivo...");
    let device = radb::connect("127.0.0.1", 5555).await?;

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

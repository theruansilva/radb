use radb::server::AdbServer;
use radb::{KeyCode, Radb, RadbTouchExt, SwipeDirection};
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("======================================================");
    println!("   RADB - Demonstração Completa de Simulações de Touch");
    println!("======================================================");

    // 1. Descobrir ou conectar ao dispositivo ADB / emulador
    let server = AdbServer::default_local();
    if server.is_running().await {
        if let Ok(devices) = server.list_devices().await {
            println!("\n[+] Dispositivos ADB Server encontrados:");
            for dev in &devices {
                println!("    - Serial: {}, Estado: {}", dev.serial, dev.state);
            }
        }
    }

    println!("\n[+] Conectando ao emulador/dispositivo Android (127.0.0.1:5555)...");
    let radb = match radb::connect("127.0.0.1", 5555).await {
        Ok(client) => client,
        Err(_) => match radb::discover("127.0.0.1").await? {
            Some(client) => client,
            None => {
                eprintln!("[!] Erro: Nenhum emulador ativo encontrado nas portas 5555..5683.");
                return Ok(());
            }
        },
    };

    println!("[✓] Conectado com sucesso!\n");

    // =========================================================================
    // 1. TOQUE SIMPLES (Single Tap)
    // =========================================================================
    println!("--- 1. Toque Simples (Single Tap) ---");
    let x = 500;
    let y = 1000;
    println!("Simulando toque na coordenada ({x}, {y})...");
    let res = radb.tap(x, y).await?;
    println!("Resultado: exit_code={}", res.exit_code);
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 2. TOQUE DUPLO (Double Tap)
    // =========================================================================
    println!("\n--- 2. Toque Duplo (Double Tap) ---");
    println!("Simulando toque duplo na coordenada ({x}, {y}) com intervalo de 150ms...");
    radb.double_tap(x, y, 150).await?;
    println!("Toque duplo executado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 3. TOQUE LONGO / MANTER PRESSIONADO (Long Press)
    // =========================================================================
    println!("\n--- 3. Toque Longo / Press & Hold (Long Press) ---");
    println!("Simulando toque longo (1.5 segundos) na coordenada ({x}, {y})...");
    radb.long_press(x, y, 1500).await?;
    println!("Toque longo executado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 4. ARRASTE RÁPIDO / FLING (Fast Swipe / Scroll)
    // =========================================================================
    println!("\n--- 4. Arraste Rápido / Fling (Swipe Rápido) ---");
    println!("Simulando swipe rápido de (500, 1500) para (500, 500) em 250ms...");
    radb.swipe(500, 1500, 500, 500, Some(250)).await?;
    println!("Arraste rápido executado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 5. ARRASTE LENTO / DESLOCAMENTO DE ELEMENTO (Slow Drag)
    // =========================================================================
    println!("\n--- 5. Arraste Lento (Drag & Move) ---");
    println!("Simulando movimento lento de item de (200, 800) para (800, 800) em 2000ms...");
    radb.swipe(200, 800, 800, 800, Some(2000)).await?;
    println!("Arraste lento executado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 6. ARRASTES DIRECIONAIS (Directional Swipes: Up, Down, Left, Right)
    // =========================================================================
    println!("\n--- 6. Arrastes Direcionais (Up, Down, Left, Right) ---");
    let center_x = 500;
    let center_y = 1000;
    let distance = 400;

    println!("a) Arraste para CIMA (Scroll para baixo)...");
    radb.swipe_directional(center_x, center_y, distance, SwipeDirection::Up, 400)
        .await?;
    sleep(Duration::from_millis(500)).await;

    println!("b) Arraste para BAIXO (Scroll para cima)...");
    radb.swipe_directional(center_x, center_y, distance, SwipeDirection::Down, 400)
        .await?;
    sleep(Duration::from_millis(500)).await;

    println!("c) Arraste para ESQUERDA (Próxima página/Card)...");
    radb.swipe_directional(center_x, center_y, distance, SwipeDirection::Left, 400)
        .await?;
    sleep(Duration::from_millis(500)).await;

    println!("d) Arraste para DIREITA (Página/Card anterior)...");
    radb.swipe_directional(center_x, center_y, distance, SwipeDirection::Right, 400)
        .await?;
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 7. ARRASTAR E SOLTAR (Drag and Drop - Android 10+)
    // =========================================================================
    println!("\n--- 7. Arrastar e Soltar (Drag and Drop) ---");
    println!("Simulando draganddrop de (300, 600) para (700, 1200)...");
    let res = radb.drag_and_drop(300, 600, 700, 1200, Some(1000)).await?;
    println!("Drag & drop finalizado: exit_code={}", res.exit_code);
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 8. DESENHO DE PADRÃO / GESTO MULTI-PONTO (Pattern Gesture / Continuous Path)
    // =========================================================================
    println!("\n--- 8. Desenho de Padrão / Gesto Multi-Ponto ---");
    let pattern_points = vec![(200, 600), (800, 600), (800, 1200), (200, 1200), (200, 600)];
    println!("Desenhando um quadrado na tela conectando 5 pontos...");
    radb.draw_path(&pattern_points, 300).await?;
    println!("Desenho de padrão finalizado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 9. TOQUE EM BOTÕES DE NAVEGAÇÃO E SISTEMA (System Navigation Keys)
    // =========================================================================
    println!("\n--- 9. Botões do Sistema / Teclas de Navegação ---");
    println!("a) Pressionando botão VOLTAR (Back)...");
    radb.press_key(KeyCode::BACK).await?;
    sleep(Duration::from_millis(600)).await;

    println!("b) Pressionando botão INÍCIO (Home)...");
    radb.press_key(KeyCode::HOME).await?;
    sleep(Duration::from_millis(600)).await;

    println!("c) Pressionando botão RECENTES (App Switch)...");
    radb.press_key(KeyCode::APP_SWITCH).await?;
    sleep(Duration::from_millis(600)).await;

    // Retorna para a tela principal
    radb.press_key(KeyCode::HOME).await?;
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 10. TOQUE EM CAMPO DE TEXTO E DIGITAÇÃO (Focus Touch & Text Input)
    // =========================================================================
    println!("\n--- 10. Toque em Campo de Texto e Digitação ---");
    println!("Tocando no centro da tela (para focar campo) e digitando texto...");
    radb.tap(500, 800).await?;
    sleep(Duration::from_millis(300)).await;
    radb.type_text("Simulacao de Toques RADB").await?;
    println!("Texto enviado!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 11. TRACKBALL / ROLAGEM DE PONTEIRO (Trackball Roll)
    // =========================================================================
    println!("\n--- 11. Simulação de Rolagem de Esfera / Trackball ---");
    println!("Rolando ponteiro (dx=10, dy=-5)...");
    radb.roll(10, -5).await?;
    println!("Rolagem concluída!");
    sleep(Duration::from_millis(800)).await;

    // =========================================================================
    // 12. TOQUE DE BAIXO NÍVEL VIA SENDEVENT (Low-Level Kernel Touch Injection)
    // =========================================================================
    println!("\n--- 12. Toque de Baixo Nível via Eventos Kernel (sendevent) ---");
    println!("Verificando se o utilitário getevent / sendevent está disponível...");
    let getevent_check = radb.shell("getevent -p 2>&1").await?;
    if !getevent_check.output.is_empty() {
        println!("Dispositivo suporta getevent/sendevent! Exemplo de comandos sendevent:");
        println!("  - Evento Touch Down:  sendevent /dev/input/eventX 3 57 <tracking_id>");
        println!("  - Evento Posição X:   sendevent /dev/input/eventX 3 53 <x>");
        println!("  - Evento Posição Y:   sendevent /dev/input/eventX 3 54 <y>");
        println!("  - Evento Sincronizar: sendevent /dev/input/eventX 0 0 0");
    }

    println!("\n======================================================");
    println!("   Todas as simulações de toques foram concluídas!   ");
    println!("======================================================");

    Ok(())
}

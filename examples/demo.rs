use radb::Radb;
use radb::server::AdbServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== RADB Rust ADB Client Demo ===");

    // 1. List devices via ADB server if available
    let server = AdbServer::default_local();
    if server.is_running().await {
        match server.list_devices().await {
            Ok(devices) => {
                println!("\nADB Server devices (port 5037):");
                if devices.is_empty() {
                    println!("  (No devices listed by adb server)");
                } else {
                    for dev in &devices {
                        println!("  - Serial: {}, State: {}", dev.serial, dev.state);
                    }
                }
            }
            Err(e) => println!("ADB server error: {e}"),
        }
    }

    // 2. Connect to emulator on port 5555 (or auto-discover) using automatic keypair
    println!("\nAttempting connection to 127.0.0.1:5555...");
    let radb = match radb::connect("127.0.0.1", 5555).await {
        Ok(client) => client,
        Err(e) => {
            println!("Could not connect directly to 5555: {e}");
            println!("Searching for active emulator (5555..5683)...");
            match radb::discover("127.0.0.1").await? {
                Some(client) => client,
                None => {
                    eprintln!("Error: No emulator found on ports 5555..5683.");
                    return Ok(());
                }
            }
        }
    };

    println!("Connected successfully!");

    // 4. Run shell commands
    println!("\n--- Executing Shell Commands ---");

    let response = radb.shell("getprop ro.product.model").await?;
    println!("Device Model: {}", response.output.trim());

    let response = radb.shell("uname -a").await?;
    println!("Kernel: {}", response.output.trim());

    let response = radb.shell("echo 'Hello from radb in Rust!'").await?;
    println!("Shell Echo: {}", response.output.trim());
    println!("Exit Code: {}", response.exit_code);

    println!("\nDemo finished successfully!");
    Ok(())
}

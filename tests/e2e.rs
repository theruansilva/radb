use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

use radb::{KeyCode, Radb, RadbImpl, RadbTouchExt};

fn get_emulator_path() -> String {
    if let Ok(home) = std::env::var("HOME") {
        let p = format!("{home}/Android/Sdk/emulator/emulator");
        if std::path::Path::new(&p).exists() {
            return p;
        }
    }
    "emulator".to_string()
}

fn list_avds() -> Vec<String> {
    let emulator = get_emulator_path();
    let output = Command::new(emulator)
        .arg("-list-avds")
        .output()
        .expect("Failed to run emulator -list-avds");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

async fn ensure_emulator() -> Result<RadbImpl, Box<dyn std::error::Error>> {
    if let Ok(client) = radb::connect("127.0.0.1", 5555).await {
        return Ok(client);
    }

    let avds = list_avds();
    let avd_name = avds.first().expect("No AVD found in emulator -list-avds");

    let emulator = get_emulator_path();
    let _ = Command::new(emulator)
        .args(["-avd", avd_name, "-no-window", "-no-audio"])
        .spawn()?;

    for _ in 0..60 {
        sleep(Duration::from_secs(2)).await;
        if let Ok(client) = radb::connect("127.0.0.1", 5555).await {
            if let Ok(resp) = client.shell("getprop sys.boot_completed").await {
                if resp.output.trim() == "1" {
                    return Ok(client);
                }
            }
        }
    }
    Err("Timeout waiting for emulator boot".into())
}

#[tokio::test]
#[ignore = "requires Android emulator"]
async fn test_e2e_demo_shell() -> Result<(), Box<dyn std::error::Error>> {
    let client = ensure_emulator().await?;
    let res = client.shell("getprop ro.product.model").await?;
    assert_eq!(res.exit_code, 0);
    assert!(!res.output.trim().is_empty());
    Ok(())
}

#[tokio::test]
#[ignore = "requires Android emulator"]
async fn test_e2e_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    let client = ensure_emulator().await?;
    let remote_path = "/sdcard/screenshot_e2e.png";
    let local_path = std::path::Path::new("./screenshot_e2e.png");

    let shell_resp = client.shell(&format!("screencap -p {remote_path}")).await?;
    assert_eq!(shell_resp.exit_code, 0);

    let sync_res = client.pull(local_path, remote_path).await?;
    assert!(matches!(sync_res, radb::error::SyncResult::Success));
    assert!(local_path.exists());

    let _ = std::fs::remove_file(local_path);
    let _ = client.shell(&format!("rm {remote_path}")).await;
    Ok(())
}

#[tokio::test]
#[ignore = "requires Android emulator"]
async fn test_e2e_touch_simulation() -> Result<(), Box<dyn std::error::Error>> {
    let client = ensure_emulator().await?;
    assert_eq!(client.tap(500, 1000).await?.exit_code, 0);
    assert_eq!(
        client.swipe(100, 500, 100, 100, Some(200)).await?.exit_code,
        0
    );
    client.press_key(KeyCode::HOME).await?;
    Ok(())
}

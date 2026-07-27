<div align="center">

# RADB — Rust Android Debug Bridge

[![Crates.io](https://img.shields.io/badge/crates.io-v1.0.0-blue.svg)](https://crates.io/crates/radb-core)
[![Documentation](https://docs.rs/radb-core/badge.svg)](https://docs.rs/radb-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**RADB** (`radb`) is a 100% pure-Rust, asynchronous library (built on the `tokio` ecosystem) for high-performance communication with the **Android Debug Bridge (ADB)** protocol.

</div>

With `radb`, you can connect directly to ADB daemons on Android devices or emulators via TCP (e.g., port `5555`), execute shell commands, simulate complex touch gestures, transfer files (`push`/`pull`), install/uninstall APKs, interact with the local ADB server, and much more — all without relying on an external `adb` binary at runtime.
---

## 🚀 Features

- **Direct TCP Connection**: Native communication with the device ADB port (e.g., `127.0.0.1:5555`), supporting multiplexed data channels (streams).
- **Automatic RSA Authentication**: Transparently generates and loads RSA key pairs from the standard Android directory (`~/.android/adbkey`), signing authentication challenges from the daemon.
- **Emulator Auto-Discovery**: Automatically scans and connects across standard active emulator port ranges (`5555..5683`).
- **Shell Execution (v1 & v2)**: Full support for the `Shell v2` protocol (with separated `stdout`, `stderr`, and `exit_code` capture).
- **Rich Touch & Gesture Simulation (`RadbTouchExt`)**:
  - `tap(x, y)`: Single coordinate tap.
  - `double_tap(x, y, delay_ms)`: Double tap with customizable interval.
  - `long_press(x, y, duration_ms)`: Touch and hold gesture.
  - `swipe(x1, y1, x2, y2, duration)`: Configurable swipe or drag.
  - `drag_and_drop(x1, y1, x2, y2)`: Drag and drop gesture (Android 10+).
  - `swipe_directional(...)`: Direction-oriented swipe (`Up`, `Down`, `Left`, `Right`).
  - `draw_path(...)`: Continuous multi-point drawing along path `[(x1, y1), (x2, y2), ...]`.
  - `press_key(...)`: Hardware/software key event injection (`HOME`, `BACK`, `POWER`, etc.).
  - `type_text(...)`: Typing text into focused input fields.
- **File Synchronization (`sync`)**:
  - Fast file transfer from local filesystem to remote device (`push`).
  - File download from device to local environment (`pull`).
  - In-memory byte buffer transfer (`push_bytes`, `pull_bytes`).
- **Application Management**: APK package installation (`install`) and uninstallation (`uninstall`).
- **Root Privilege Control**: Switch the remote ADB daemon to root mode (`root`) or unroot mode (`unroot`).
- **Port Forwarding (`TcpForwarder`)**: Forward local host TCP ports directly to target device ports.
- **Local ADB Server (`AdbServer`)**: Query the local host ADB server daemon (port `5037`) and list connected devices.

---

## 📦 Installation

Add `radb-core` to your `Cargo.toml`:

```toml
[dependencies]
radb-core = "1.0.0"
tokio = { version = "1.43", features = ["full"] }
```

Or import it with the `radb` alias:

```toml
[dependencies]
radb = { package = "radb-core", version = "1.0.0" }
tokio = { version = "1.43", features = ["full"] }
```

---

## 💻 Usage Examples

### 1. Basic Connection & Shell Execution

```rust,no_run
use radb::Radb;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect directly to local device on port 5555 using ~/.android/adbkey
    let device = radb::connect("127.0.0.1", 5555).await?;

    // Execute a shell command
    let response = device.shell("getprop ro.product.model").await?;

    println!("Device Model: {}", response.output.trim());
    println!("Exit Code: {}", response.exit_code);

    Ok(())
}
```

### 2. Automatic Emulator Discovery

```rust,no_run
use radb::Radb;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Discover the first active emulator on port range 5555..5683
    if let Some(device) = radb::discover("127.0.0.1").await? {
        let resp = device.shell("uname -a").await?;
        println!("Kernel: {}", resp.output.trim());
    } else {
        println!("No active emulator was found.");
    }

    Ok(())
}
```

### 3. Touch & Gesture Simulation (`RadbTouchExt`)

```rust,no_run
use radb::{Radb, RadbTouchExt, KeyCode, SwipeDirection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = radb::connect("127.0.0.1", 5555).await?;

    // Tap at coordinates (500, 1000)
    device.tap(500, 1000).await?;

    // Long press for 1 second
    device.long_press(500, 1000, 1000).await?;

    // Swipe upward
    device.swipe_directional(500, 1000, 400, SwipeDirection::Up, 300).await?;

    // Press the HOME key
    device.press_key(KeyCode::HOME).await?;

    // Type text into focused input
    device.type_text("Hello World!").await?;

    Ok(())
}
```

### 4. File Sync (`push` and `pull`)

```rust,no_run
use std::path::Path;
use radb::{Radb, error::SyncResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = radb::connect("127.0.0.1", 5555).await?;

    // Take a screenshot on the remote device
    device.shell("screencap -p /sdcard/screenshot.png").await?;

    // Download the screenshot to local machine (pull)
    let local_path = Path::new("./screenshot.png");
    match device.pull(local_path, "/sdcard/screenshot.png").await? {
        SyncResult::Success => println!("Screenshot downloaded successfully!"),
        SyncResult::Failure(err) => eprintln!("Download error: {err}"),
    }

    // Upload a local file to the device (push)
    let src_file = Path::new("./my_file.txt");
    device.push(src_file, "/sdcard/my_file.txt", 0o644, 0).await?;

    Ok(())
}
```

### 5. Installing & Uninstalling APKs

```rust,no_run
use std::path::Path;
use radb::{Radb, error::{InstallResult, UninstallResult}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = radb::connect("127.0.0.1", 5555).await?;

    // Install an APK with reinstall option (-r)
    let apk = Path::new("./app-release.apk");
    match device.install(apk, &["-r"]).await? {
        InstallResult::Success => println!("App installed successfully!"),
        InstallResult::Failure(err) => eprintln!("Installation failed: {err}"),
    }

    // Uninstall app by package name
    match device.uninstall("com.example.myapp").await? {
        UninstallResult::Success => println!("App uninstalled successfully!"),
        UninstallResult::Failure { reason, exit_code } => {
            eprintln!("Error ({exit_code}): {reason}");
        }
    }

    Ok(())
}
```

### 6. TCP Port Forwarding (`TcpForwarder`)

```rust,no_run
use std::sync::Arc;
use radb::forwarding::TcpForwarder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Arc::new(radb::connect("127.0.0.1", 5555).await?);

    // Forward host port 8080 to device port 8080
    let forwarder = TcpForwarder::start(device.clone(), 8080, 8080).await?;
    println!("Forwarding connections on 127.0.0.1:{}", forwarder.local_port);

    // Forwarder remains active while `forwarder` instance is in scope
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    Ok(())
}
```

### 7. Interacting with Local ADB Server (`AdbServer`)

```rust,no_run
use radb::server::AdbServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AdbServer::default_local();

    if server.is_running().await {
        let devices = server.list_devices().await?;
        println!("Devices listed by local ADB server:");
        for dev in devices {
            println!(" - Serial: {}, State: {}", dev.serial, dev.state);
        }
    } else {
        println!("Local ADB server (port 5037) is not running.");
    }

    Ok(())
}
```

---

## 🏗️ Module Architecture

| Module | Description |
|---|---|
| `radb::Radb` | Primary trait for high-level ADB operations. |
| `radb::RadbImpl` | Default implementation for native TCP ADB connections. |
| `radb::touch` | Extension trait `RadbTouchExt` for touch, gesture, and input event simulation. |
| `radb::sync` | Implementation of the ADB Sync service for file transfers (`push`/`pull`). |
| `radb::shell` | Shell command execution and response parsing for `shell` v1 and v2. |
| `radb::keypair` | Management and signing of RSA ADB authentication keypairs (`~/.android/adbkey`). |
| `radb::forwarding` | TCP port forwarding utility (`TcpForwarder`). |
| `radb::server` | Client for communicating with the local host ADB Server daemon (port 5037). |
| `radb::connection` | Low-level TCP connection management, handshake, and authentication. |
| `radb::stream` | Multiplexed ADB data channel handle (`AdbStream`). |
| `radb::error` | Custom error types (`AdbError`) and result aliases. |

---

## 🛠️ Running Examples & Tests

Run the general demo example:

```bash
cargo run --example demo
```

Run the touch & gesture simulation example:

```bash
cargo run --example touch_simulation
```

Run the screenshot capture example:

```bash
cargo run --example screenshot
```

Build and execute unit tests:

```bash
cargo test
```

---

## 📄 License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.

---

## 👤 Author

Developed by **Ruan Silva** (<progmruansilva@gmail.com>).

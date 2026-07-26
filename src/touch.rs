use crate::Radb;
use crate::error::Result;
use crate::shell::AdbShellResponse;
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

/// Direction for swipe operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Keycodes commonly used for system key events.
pub struct KeyCode;

impl KeyCode {
    pub const HOME: &'static str = "KEYCODE_HOME";
    pub const BACK: &'static str = "KEYCODE_BACK";
    pub const APP_SWITCH: &'static str = "KEYCODE_APP_SWITCH";
    pub const POWER: &'static str = "KEYCODE_POWER";
    pub const VOLUME_UP: &'static str = "KEYCODE_VOLUME_UP";
    pub const VOLUME_DOWN: &'static str = "KEYCODE_VOLUME_DOWN";
    pub const ENTER: &'static str = "KEYCODE_ENTER";
    pub const TAB: &'static str = "KEYCODE_TAB";
    pub const SPACE: &'static str = "KEYCODE_SPACE";
    pub const DEL: &'static str = "KEYCODE_DEL";
}

/// Extension trait for `Radb` providing rich touch, swipe, gesture, and input simulation.
#[async_trait]
pub trait RadbTouchExt {
    /// Perform a single tap touch at coordinate (x, y).
    async fn tap(&self, x: u32, y: u32) -> Result<AdbShellResponse>;

    /// Perform a double tap at coordinate (x, y) with a delay (in ms) between taps.
    async fn double_tap(&self, x: u32, y: u32, delay_ms: u64) -> Result<AdbShellResponse>;

    /// Perform a long press (touch and hold) at coordinate (x, y) for `duration_ms`.
    async fn long_press(&self, x: u32, y: u32, duration_ms: u32) -> Result<AdbShellResponse>;

    /// Perform a swipe/drag touch gesture from (start_x, start_y) to (end_x, end_y).
    /// `duration_ms` specifies how long the swipe takes (e.g., 300ms for fast fling, 1500ms for drag).
    async fn swipe(
        &self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        duration_ms: Option<u32>,
    ) -> Result<AdbShellResponse>;

    /// Perform a drag and drop gesture from (start_x, start_y) to (end_x, end_y) (Android 10+).
    async fn drag_and_drop(
        &self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        duration_ms: Option<u32>,
    ) -> Result<AdbShellResponse>;

    /// Perform a directional swipe relative to a starting point (e.g. center of screen).
    async fn swipe_directional(
        &self,
        center_x: u32,
        center_y: u32,
        distance: u32,
        direction: SwipeDirection,
        duration_ms: u32,
    ) -> Result<AdbShellResponse>;

    /// Draw/drag along a multi-point continuous path `[(x1, y1), (x2, y2), ...]`.
    async fn draw_path(&self, points: &[(u32, u32)], step_duration_ms: u32) -> Result<()>;

    /// Send a key event simulation (e.g., "KEYCODE_BACK", "KEYCODE_HOME", "4", "3").
    async fn press_key(&self, keycode: &str) -> Result<AdbShellResponse>;

    /// Type text into a currently focused input field.
    async fn type_text(&self, text: &str) -> Result<AdbShellResponse>;

    /// Roll trackball pointer by (dx, dy).
    async fn roll(&self, dx: i32, dy: i32) -> Result<AdbShellResponse>;
}

#[async_trait]
impl<T: Radb + ?Sized> RadbTouchExt for T {
    async fn tap(&self, x: u32, y: u32) -> Result<AdbShellResponse> {
        self.shell(&format!("input tap {x} {y}")).await
    }

    async fn double_tap(&self, x: u32, y: u32, delay_ms: u64) -> Result<AdbShellResponse> {
        let first = self.tap(x, y).await?;
        sleep(Duration::from_millis(delay_ms)).await;
        self.tap(x, y).await?;
        Ok(first)
    }

    async fn long_press(&self, x: u32, y: u32, duration_ms: u32) -> Result<AdbShellResponse> {
        // A long press in Android adb shell input is executed as a swipe with identical start & end coordinates over duration
        self.swipe(x, y, x, y, Some(duration_ms)).await
    }

    async fn swipe(
        &self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        duration_ms: Option<u32>,
    ) -> Result<AdbShellResponse> {
        let cmd = match duration_ms {
            Some(dur) => format!("input swipe {start_x} {start_y} {end_x} {end_y} {dur}"),
            None => format!("input swipe {start_x} {start_y} {end_x} {end_y}"),
        };
        self.shell(&cmd).await
    }

    async fn drag_and_drop(
        &self,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        duration_ms: Option<u32>,
    ) -> Result<AdbShellResponse> {
        let cmd = match duration_ms {
            Some(dur) => format!("input draganddrop {start_x} {start_y} {end_x} {end_y} {dur}"),
            None => format!("input draganddrop {start_x} {start_y} {end_x} {end_y}"),
        };
        self.shell(&cmd).await
    }

    async fn swipe_directional(
        &self,
        center_x: u32,
        center_y: u32,
        distance: u32,
        direction: SwipeDirection,
        duration_ms: u32,
    ) -> Result<AdbShellResponse> {
        let (end_x, end_y) = match direction {
            SwipeDirection::Up => (center_x, center_y.saturating_sub(distance)),
            SwipeDirection::Down => (center_x, center_y + distance),
            SwipeDirection::Left => (center_x.saturating_sub(distance), center_y),
            SwipeDirection::Right => (center_x + distance, center_y),
        };
        self.swipe(center_x, center_y, end_x, end_y, Some(duration_ms))
            .await
    }

    async fn draw_path(&self, points: &[(u32, u32)], step_duration_ms: u32) -> Result<()> {
        if points.len() < 2 {
            return Ok(());
        }
        for window in points.windows(2) {
            let (x1, y1) = window[0];
            let (x2, y2) = window[1];
            self.swipe(x1, y1, x2, y2, Some(step_duration_ms)).await?;
        }
        Ok(())
    }

    async fn press_key(&self, keycode: &str) -> Result<AdbShellResponse> {
        self.shell(&format!("input keyevent {keycode}")).await
    }

    async fn type_text(&self, text: &str) -> Result<AdbShellResponse> {
        // Replace spaces with %s for adb input text standard
        let escaped = text.replace(' ', "%s");
        self.shell(&format!("input text '{escaped}'")).await
    }

    async fn roll(&self, dx: i32, dy: i32) -> Result<AdbShellResponse> {
        self.shell(&format!("input roll {dx} {dy}")).await
    }
}

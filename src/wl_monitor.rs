use std::collections::HashMap;

use wayland_client::backend::ObjectId;

#[derive(Default)]
pub struct WlResolution {
    pub height: i32,
    pub width: i32,
}

#[derive(Default)]
pub struct WlPosition {
    pub x: i32,
    pub y: i32,
}

pub struct WlMonitorMode {
    mode_id: ObjectId,
    head_id: ObjectId,
    refresh_rate: i32,
    resolution: WlResolution,
}

impl Default for WlMonitorMode {
    fn default() -> Self {
        WlMonitorMode {
            mode_id: ObjectId::null(),
            head_id: ObjectId::null(),
            refresh_rate: 0,
            resolution: WlResolution::default(),
        }
    }
}

pub struct WlMonitor {
    head_id: ObjectId,
    name: String,
    modes: Vec<WlMonitorMode>,
    resolution: WlResolution,
    position: WlPosition,
    enabled: bool,
}

impl Default for WlMonitor {
    fn default() -> Self {
        WlMonitor {
            head_id: ObjectId::null(),
            name: String::new(),
            modes: Vec::new(),
            resolution: WlResolution::default(),
            position: WlPosition::default(),
            enabled: false,
        }
    }
}

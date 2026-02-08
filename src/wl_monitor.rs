use wayland_client::{WEnum, backend::ObjectId, protocol::wl_output::Transform};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1,
    zwlr_output_mode_v1::ZwlrOutputModeV1,
};

#[derive(Default, Clone)]
pub struct WlResolution {
    pub height: i32,
    pub width: i32,
}

#[derive(Default, Clone)]
pub struct WlPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone)]
pub struct WlMonitorMode {
    pub mode_id: ObjectId,
    pub head_id: ObjectId,
    pub refresh_rate: i32,
    pub resolution: WlResolution,
    pub preferred: bool,
    pub proxy: ZwlrOutputModeV1,
}

#[derive(Clone)]
pub struct WlMonitor {
    pub head_id: ObjectId,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial_number: String,
    pub modes: Vec<WlMonitorMode>,
    pub resolution: WlResolution,
    pub position: WlPosition,
    pub scale: f64,
    pub enabled: bool,
    pub current_mode: Option<ZwlrOutputModeV1>,
    pub transform: WEnum<Transform>,
    pub head: ZwlrOutputHeadV1,
    pub changed: bool,
}

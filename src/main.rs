// Hi, this is xantarius I just want to make you guys aware of this code first of all for me it was
// very complex to get this even working. The only tip I can give you guys is to read the xml file
// that I have added in the comments, so you can basically undersatnd each event of the interface
//
// second thing, you need to understand how objects working in the wayland, and how request, event
// model works here, get those concepts clear and read the xml file, then it will be easy for you to go through this code
// otherwise honestly nothing will make sense here trust me

use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Receiver, mpsc::Sender, mpsc::channel},
};

use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, WEnum,
    backend::ObjectId,
    protocol::{wl_output::Transform, wl_registry},
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1, zwlr_output_manager_v1,
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

#[derive(Debug)]
struct Mode {
    id: ObjectId,
    monitor_id: ObjectId,
    mhz: i32,
    height: i32,
    width: i32,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            id: ObjectId::null(),
            monitor_id: ObjectId::null(),
            mhz: Default::default(),
            height: Default::default(),
            width: Default::default(),
        }
    }
}

#[derive(Debug)]
struct Monitor {
    id: ObjectId,
    name: String,
    modes: Vec<Mode>,
    enabled: bool,
    scale: f64,
    position_x: i32,
    position_y: i32,
    mode: Option<ZwlrOutputModeV1>,
    transform: WEnum<Transform>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self {
            id: ObjectId::null(),
            name: String::new(),
            modes: Vec::new(),
            enabled: false,
            scale: 1.0,
            position_x: 0,
            mode: None,
            position_y: 0,
            transform: WEnum::Value(Transform::Normal),
        }
    }
}

#[derive(Debug)]
struct WlMonitorResolution {
    height: i32,
    width: i32,
}

#[derive(Debug)]
struct WlMonitorPosition {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct WlMonitorMode {
    id: ObjectId,
    monitor_id: ObjectId,
    refresh_rate: i32,
    resolution: WlMonitorResolution,
}

#[derive(Debug)]
struct WlMonitor {
    id: ObjectId,
    name: String,
    enabled: bool,
    refresh_rate: i32,
    resolution: WlMonitorResolution,
    position: WlMonitorPosition,
    mode: Vec<WlMonitorMode>,
}

#[derive(Debug)]
enum EventEmit {
    Initiate(WlMonitor),
}

#[derive(Debug)]
struct AppState {
    emit: Sender<EventEmit>,
    monitor_hash_map: HashMap<ObjectId, Monitor>,
    mode_monitor_hash_map: HashMap<ObjectId, ObjectId>,
}

// Protocol: https://gitlab.freedesktop.org/wayland/wayland/-/blob/main/protocol/wayland.xml#L71
impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        _: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == zwlr_output_manager_v1::ZwlrOutputManagerV1::interface().name
        {
            registry.bind::<zwlr_output_manager_v1::ZwlrOutputManagerV1, _, _>(
                name,
                version,
                qh,
                (),
            );
        }
    }
}

// Protocol: https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-output-management-unstable-v1.xml#L46
impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                state.monitor_hash_map.insert(
                    head.id(),
                    Monitor {
                        id: head.id(),
                        ..Default::default()
                    },
                );
            }
            zwlr_output_manager_v1::Event::Done { serial: _ } => {
                for monitor in state.monitor_hash_map.values() {
                    let active_mode = monitor
                        .mode
                        .as_ref()
                        .and_then(|m| monitor.modes.iter().find(|mode| mode.id == m.id()));

                    let (active_refresh_rate, active_resolution) = active_mode
                        .map(|m| {
                            (
                                m.mhz,
                                WlMonitorResolution {
                                    height: m.height,
                                    width: m.width,
                                },
                            )
                        })
                        .unwrap_or((
                            0,
                            WlMonitorResolution {
                                height: 0,
                                width: 0,
                            },
                        ));

                    let wl_modes: Vec<WlMonitorMode> = monitor
                        .modes
                        .iter()
                        .map(|m| WlMonitorMode {
                            id: m.id.clone(),
                            monitor_id: m.monitor_id.clone(),
                            refresh_rate: m.mhz,
                            resolution: WlMonitorResolution {
                                height: m.height,
                                width: m.width,
                            },
                        })
                        .collect();

                    let wl_monitor = WlMonitor {
                        id: monitor.id.clone(),
                        name: monitor.name.clone(),
                        enabled: monitor.enabled,
                        refresh_rate: active_refresh_rate,
                        resolution: active_resolution,
                        position: WlMonitorPosition {
                            x: monitor.position_x,
                            y: monitor.position_y,
                        },
                        mode: wl_modes,
                    };

                    let _ = state.emit.send(EventEmit::Initiate(wl_monitor));
                }
            }
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &wayland_client::QueueHandle<Self>,
    ) -> Arc<dyn wayland_client::backend::ObjectData> {
        if opcode == 0 {
            qh.make_data::<zwlr_output_head_v1::ZwlrOutputHeadV1, _>(())
        } else {
            unreachable!("unknown opcode for zwlr_output_manager_v1")
        }
    }
}

// Protocol: https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-output-management-unstable-v1.xml#L96
impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for AppState {
    fn event(
        state: &mut Self,
        head: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: <zwlr_output_head_v1::ZwlrOutputHeadV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        let head_id = head.id();
        let Some(monitor) = state.monitor_hash_map.get_mut(&head_id) else {
            return;
        };

        if let zwlr_output_head_v1::Event::Mode { mode } = &event {
            state.mode_monitor_hash_map.insert(mode.id(), head_id);
            monitor.modes.push(Mode {
                id: mode.id(),
                monitor_id: monitor.id.clone(),
                ..Default::default()
            });
            return;
        }

        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                monitor.name = name;
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => {
                monitor.mode = Some(mode);
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                monitor.enabled = enabled != 0;
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                monitor.scale = scale;
            }
            zwlr_output_head_v1::Event::Transform { transform } => {
                monitor.transform = transform;
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                monitor.position_x = x;
                monitor.position_y = y;
            }
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &wayland_client::QueueHandle<Self>,
    ) -> Arc<dyn wayland_client::backend::ObjectData> {
        if opcode == 3 {
            qh.make_data::<zwlr_output_mode_v1::ZwlrOutputModeV1, _>(())
        } else {
            unreachable!("unknown opcode for zwlr_output_head_v1")
        }
    }
}

// Protocol: https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-output-management-unstable-v1.xml#L250
impl Dispatch<zwlr_output_mode_v1::ZwlrOutputModeV1, ()> for AppState {
    fn event(
        state: &mut Self,
        mode_obj: &zwlr_output_mode_v1::ZwlrOutputModeV1,
        event: <zwlr_output_mode_v1::ZwlrOutputModeV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        let mode_id = mode_obj.id();
        let Some(monitor) = get_monitor_by_mode_id(state, &mode_id) else {
            return;
        };
        let Some(mode) = monitor.modes.iter_mut().find(|m| m.id == mode_id) else {
            return;
        };
        match event {
            zwlr_output_mode_v1::Event::Size { height, width } => {
                mode.height = height;
                mode.width = width;
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                mode.mhz = refresh / 1000;
            }
            _ => {}
        }
    }
}

fn get_monitor_by_mode_id<'a>(
    state: &'a mut AppState,
    mode_id: &'a ObjectId,
) -> Option<&'a mut Monitor> {
    let Some(monitor_id) = state.mode_monitor_hash_map.get_mut(mode_id) else {
        println!("couldn't find mode with the mode object id {}", mode_id);
        return None;
    };
    let Some(monitor) = state.monitor_hash_map.get_mut(monitor_id) else {
        println!("couldn't find monitor with the mode object id {}", mode_id);
        return None;
    };
    Some(monitor)
}

fn main() {
    let (tx, rx): (Sender<EventEmit>, Receiver<EventEmit>) = channel();

    let mut state = AppState {
        emit: tx,
        monitor_hash_map: HashMap::new(),
        mode_monitor_hash_map: HashMap::new(),
    };

    let conn = Connection::connect_to_env().expect("error: failed to connect to wayland server");
    let display_object = conn.display();
    let mut event_queue: EventQueue<AppState> = conn.new_event_queue();
    let queue_handler = event_queue.handle();
    display_object.get_registry(&queue_handler, ());
    event_queue
        .roundtrip(&mut state)
        .expect("error: failed to start the event queue roundtrip");
    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .expect("error: failed to start the dispacth pending event");

        while let Ok(event) = rx.try_recv() {
            match event {
                EventEmit::Initiate(monitor) => {
                    println!("\n=== Monitor: {} ===", monitor.name);
                    println!("ID: {}", monitor.id);
                    println!("Enabled: {}", monitor.enabled);
                    println!("Position: ({}, {})", monitor.position.x, monitor.position.y);
                    println!(
                        "Active: {}x{} @ {}Hz",
                        monitor.resolution.width, monitor.resolution.height, monitor.refresh_rate
                    );
                    println!("Available modes:");
                    for mode in &monitor.mode {
                        println!(
                            "  - {}x{} @ {}Hz",
                            mode.resolution.width, mode.resolution.height, mode.refresh_rate
                        );
                    }
                }
            }
        }
    }
}

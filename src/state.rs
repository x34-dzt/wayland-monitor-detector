use std::{
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender},
    },
};

use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    backend::ObjectId,
    protocol::{wl_output::Transform, wl_registry},
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

use crate::wl_monitor::{WlMonitor, WlMonitorMode, WlPosition, WlResolution};

pub enum WlMonitorEvent {
    InitialState(Vec<WlMonitor>),
    Changed(WlMonitor),
    Removed { id: ObjectId, name: String },
}

enum WlMonitorAction {
    Toggle,
    SwitchMode,
}

pub struct WlMonitorManager {
    conn: Connection,
    emitter: SyncSender<WlMonitorEvent>,
    monitors: HashMap<ObjectId, WlMonitor>,
    mode_monitor: HashMap<ObjectId, ObjectId>,
    controller: Receiver<WlMonitorAction>,
    zwlr_manager: Option<ZwlrOutputManagerV1>,
    serial: Option<u32>,
    initialized: bool,
}

enum WlMonitorManagerError {
    ConnectionError(String),
    EventQueueError(String),
}

impl WlMonitorManager {
    fn new_connection(
        emitter: SyncSender<WlMonitorEvent>,
        controller: Receiver<WlMonitorAction>,
    ) -> Result<(Self, EventQueue<Self>), WlMonitorManagerError> {
        let conn = Connection::connect_to_env().map_err(|e| {
            WlMonitorManagerError::ConnectionError(e.to_string())
        })?;

        let display_object = conn.display();
        let event_queue: EventQueue<WlMonitorManager> = conn.new_event_queue();
        let queue_handler = event_queue.handle();
        display_object.get_registry(&queue_handler, ());

        let state = WlMonitorManager {
            conn,
            emitter,
            monitors: HashMap::new(),
            mode_monitor: HashMap::new(),
            controller,
            zwlr_manager: None,
            serial: None,
            initialized: false,
        };

        Ok((state, event_queue))
    }

    fn run(
        &mut self,
        mut eq: EventQueue<Self>,
    ) -> Result<(), WlMonitorManagerError> {
        loop {
            eq.blocking_dispatch(self).map_err(|e| {
                WlMonitorManagerError::EventQueueError(e.to_string())
            })?;
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WlMonitorManager {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == ZwlrOutputManagerV1::interface().name
        {
            let bound = registry.bind::<ZwlrOutputManagerV1, _, _>(name, version, qh, ());
            state.zwlr_manager = Some(bound);
        }
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for WlMonitorManager {
    fn event(
        state: &mut Self,
        _: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                state.monitors.insert(
                    head.id(),
                    WlMonitor {
                        head_id: head.id(),
                        name: String::new(),
                        description: String::new(),
                        make: String::new(),
                        model: String::new(),
                        serial_number: String::new(),
                        modes: Vec::new(),
                        resolution: WlResolution::default(),
                        position: WlPosition::default(),
                        scale: 1.0,
                        enabled: false,
                        current_mode: None,
                        head,
                        dirty: false,
                    },
                );
            }
            zwlr_output_manager_v1::Event::Done { serial } => {
                state.serial = Some(serial);
                if !state.initialized {
                    state.initialized = true;
                    let monitors = state.monitors.values().cloned().collect();
                    let _ = state.emitter.send(WlMonitorEvent::InitialState(monitors));
                }
            }
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &QueueHandle<Self>,
    ) -> Arc<dyn wayland_client::backend::ObjectData> {
        if opcode == 0 {
            qh.make_data::<ZwlrOutputHeadV1, _>(())
        } else {
            unreachable!()
        }
    }
}

impl Dispatch<ZwlrOutputHeadV1, ()> for WlMonitorManager {
    fn event(
        state: &mut Self,
        head: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let head_id = head.id();

        if let zwlr_output_head_v1::Event::Finished = &event {
            if let Some(monitor) = state.monitors.remove(&head_id) {
                let _ = state.emitter.send(WlMonitorEvent::Removed {
                    id: monitor.head_id,
                    name: monitor.name,
                });
            }
            return;
        }

        let Some(monitor) = state.monitors.get_mut(&head_id) else {
            return;
        };

        if let zwlr_output_head_v1::Event::Mode { mode } = &event {
            state.mode_monitor.insert(mode.id(), head_id);
            monitor.modes.push(WlMonitorMode {
                mode_id: mode.id(),
                head_id: monitor.head_id.clone(),
                refresh_rate: 0,
                resolution: WlResolution::default(),
                preferred: false,
                proxy: mode.clone(),
            });
            return;
        }

        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                monitor.name = name;
            }
            zwlr_output_head_v1::Event::Description { description } => {
                monitor.description = description;
            }
            zwlr_output_head_v1::Event::Make { make } => {
                monitor.make = make;
            }
            zwlr_output_head_v1::Event::Model { model } => {
                monitor.model = model;
            }
            zwlr_output_head_v1::Event::SerialNumber { serial_number } => {
                monitor.serial_number = serial_number;
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                monitor.enabled = enabled != 0;
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => {
                monitor.current_mode = Some(mode);
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                monitor.position = WlPosition { x, y };
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                monitor.scale = scale;
            }
            _ => {}
        }

        if state.initialized {
            monitor.dirty = true;
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &QueueHandle<Self>,
    ) -> Arc<dyn wayland_client::backend::ObjectData> {
        if opcode == 3 {
            qh.make_data::<ZwlrOutputModeV1, _>(())
        } else {
            unreachable!()
        }
    }
}

impl Dispatch<ZwlrOutputModeV1, ()> for WlMonitorManager {
    fn event(
        state: &mut Self,
        mode_obj: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mode_id = mode_obj.id();
        let Some(monitor_id) = state.mode_monitor.get(&mode_id) else {
            return;
        };
        let Some(monitor) = state.monitors.get_mut(monitor_id) else {
            return;
        };
        let Some(mode) = monitor.modes.iter_mut().find(|m| m.mode_id == mode_id) else {
            return;
        };
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                mode.resolution = WlResolution { width, height };
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                mode.refresh_rate = refresh / 1000;
            }
            zwlr_output_mode_v1::Event::Preferred => {
                mode.preferred = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ()> for WlMonitorManager {
    fn event(
        _: &mut Self,
        _: &ZwlrOutputConfigurationV1,
        _event: zwlr_output_configuration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // TODO: handle config result when implementing toggle/switch
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for WlMonitorManager {
    fn event(
        _: &mut Self,
        _: &ZwlrOutputConfigurationHeadV1,
        _event: zwlr_output_configuration_head_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

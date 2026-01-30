// Hi, this is xantarius I just want to make you guys aware of this code first of all for me it was
// very complex to get this even working. The only tip I can give you guys is to read the xml file
// that I have added in the comments, so you can basically undersatnd each event of the interface
//
// second thing, you need to understand how objects working in the wayland, and how request, event
// model works here, get those concepts clear and read the xml file, then it will be easy for you to go through this code
// otherwise honestly nothing will make sense here trust me

use std::{collections::HashMap, sync::Arc};

use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, WEnum,
    backend::ObjectId,
    protocol::{wl_output::Transform, wl_registry},
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1, zwlr_output_manager_v1, zwlr_output_mode_v1,
};

#[derive(Debug, Default)]
struct Mode {
    mhz: i32,
    height: i32,
    width: i32,
}

#[derive(Debug)]
struct Monitor {
    name: String,
    modes: Vec<Mode>,
    enabled: bool,
    scale: f64,
    position_x: i32,
    position_y: i32,
    transform: WEnum<Transform>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self {
            name: String::new(),
            modes: Vec::new(),
            enabled: false,
            scale: 1.0,
            position_x: 0,
            position_y: 0,
            transform: WEnum::Value(Transform::Normal),
        }
    }
}

#[derive(Debug)]
struct AppState {
    // Monitors keyed by their ZwlrOutputHeadV1 ObjectId
    monitors: HashMap<ObjectId, Monitor>,
    // Maps ZwlrOutputModeV1 ObjectId -> ZwlrOutputHeadV1 ObjectId (parent)
    mode_to_head: HashMap<ObjectId, ObjectId>,
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
                state.monitors.insert(head.id(), Monitor::default());
            }
            zwlr_output_manager_v1::Event::Done { serial: _ } => {
                let monitors: Vec<&Monitor> = state.monitors.values().collect();
                println!("{:#?}", monitors);
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

        if let zwlr_output_head_v1::Event::Mode { mode } = &event {
            state.mode_to_head.insert(mode.id(), head_id.clone());
            if let Some(monitor) = state.monitors.get_mut(&head_id) {
                monitor.modes.push(Mode::default());
            }
            return;
        }

        let Some(monitor) = state.monitors.get_mut(&head_id) else {
            return;
        };

        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                monitor.name = name;
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
        let Some(head_id) = state.mode_to_head.get(&mode_id) else {
            return;
        };
        let Some(monitor) = state.monitors.get_mut(head_id) else {
            return;
        };
        let Some(mode) = monitor.modes.last_mut() else {
            return;
        };

        match event {
            zwlr_output_mode_v1::Event::Size { height, width } => {
                mode.height = height;
                mode.width = width;
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                mode.mhz = refresh;
            }
            _ => {}
        }
    }
}

fn main() {
    let mut state = AppState {
        monitors: HashMap::new(),
        mode_to_head: HashMap::new(),
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
    }
}

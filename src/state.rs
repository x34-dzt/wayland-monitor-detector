use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, SyncSender},
};

use wayland_client::{
    Connection, Dispatch, EventQueue, backend::ObjectId, protocol::wl_registry,
};

use crate::wl_monitor::WlMonitor;

enum WlMonitorEvent {
    InitialState(Vec<WlMonitor>),
}

enum WlMonitorAction {
    Toggle,
    SwitchMode,
}

pub struct WlMonitorManager {
    conn: Connection,
    emitter: SyncSender<WlMonitor>,
    monitors: HashMap<ObjectId, WlMonitor>,
    mode_monitor: HashMap<ObjectId, ObjectId>,
    controller: Receiver<WlMonitorAction>,
}

enum WlMonitorManagerError {
    ConnectionError(String),
    EventQueueError(String),
}

impl WlMonitorManager {
    fn new_connection(
        self,
        emitter: SyncSender<WlMonitor>,
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
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        todo!()
    }
}

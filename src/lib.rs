//! Wayland Monitor Detector Library
//!
//! A library for detecting and monitoring Wayland display outputs using the
//! wlr-output-management protocol.
//!
//! # Example
//!
//! ```no_run
//! use wl_monitor_detector::{MonitorDetector, MonitorEvent};
//!
//! let (detector, receiver) = MonitorDetector::new().unwrap();
//!
//! std::thread::spawn(move || detector.run());
//!
//! while let Ok(event) = receiver.recv() {
//!     if let MonitorEvent::Detected(monitor) = event {
//!         println!("{}: {}x{}", monitor.name,
//!             monitor.resolution.width, monitor.resolution.height);
//!     }
//! }
//! ```

mod internal;

use std::sync::mpsc::{self, Receiver, Sender};

pub use wayland_client::backend::ObjectId;
use wayland_client::{Connection, EventQueue};

use internal::AppState;

#[derive(Debug, Clone)]
pub struct WlMonitorResolution {
    pub height: i32,
    pub width: i32,
}

#[derive(Debug, Clone)]
pub struct WlMonitorPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct WlMonitorMode {
    pub id: ObjectId,
    pub monitor_id: ObjectId,
    pub refresh_rate: i32,
    pub resolution: WlMonitorResolution,
}

/// Information about a detected monitor.
#[derive(Debug, Clone)]
pub struct WlMonitor {
    pub id: ObjectId,
    pub name: String,
    pub enabled: bool,
    pub refresh_rate: i32,
    pub resolution: WlMonitorResolution,
    pub position: WlMonitorPosition,
    pub modes: Vec<WlMonitorMode>,
}

#[derive(Debug)]
pub enum Error {
    /// Event queue failed to run
    EventQueueError(String),
    /// Connection error with the wayland server
    ConnectionFailed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EventQueueError(msg) => {
                write!(f, "Event queue error: {}", msg)
            }
            Error::ConnectionFailed(msg) => {
                write!(f, "Connection failed error: {}", msg)
            }
        }
    }
}

/// Events emitted by the monitor detector.
#[derive(Debug)]
pub enum MonitorEvent {
    /// Emitted once and provides all the information for the current monitors avaialble to the
    /// wayland
    InitialState(WlMonitor),
}

#[derive(Debug)]
pub struct MonitorReceiver {
    rx: Receiver<MonitorEvent>,
}

impl MonitorReceiver {
    pub fn recv(&self) -> Result<MonitorEvent, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn try_recv(&self) -> Result<MonitorEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}

/// The main monitor detector that connects to Wayland and listens for output events.
#[derive(Debug)]
pub struct MonitorDetector {
    state: AppState,
    event_queue: EventQueue<AppState>,
}

impl MonitorDetector {
    /// Creates a new detector and receiver pair.
    pub fn new() -> Result<(Self, MonitorReceiver), Error> {
        let (tx, rx): (Sender<MonitorEvent>, Receiver<MonitorEvent>) =
            mpsc::channel();

        let state = AppState::new(tx);

        let conn = Connection::connect_to_env()
            .map_err(|e| Error::ConnectionFailed(e.to_string()))?;

        let display_object = conn.display();
        let event_queue: EventQueue<AppState> = conn.new_event_queue();
        let queue_handler = event_queue.handle();
        display_object.get_registry(&queue_handler, ());

        let mut detector = Self { state, event_queue };

        detector
            .event_queue
            .roundtrip(&mut detector.state)
            .map_err(|e| Error::EventQueueError(e.to_string()))?;

        let receiver = MonitorReceiver { rx };

        Ok((detector, receiver))
    }

    /// Runs the event loop (blocking). Call this in a separate thread.
    pub fn run(mut self) -> Result<(), Error> {
        loop {
            self.event_queue
                .blocking_dispatch(&mut self.state)
                .map_err(|e| Error::EventQueueError(e.to_string()))?;
        }
    }
}

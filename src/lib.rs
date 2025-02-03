#![feature(once_cell_get_mut)]
#![feature(debug_closure_helpers)]
#![no_std]

mod game;

extern crate alloc;
#[macro_use]
extern crate playdate as pd;

use core::cell::OnceCell;
use core::ptr::NonNull;
use bevy_app::App;
use bevy_playdate::DefaultPlugins;
use pd::display::Display;
use pd::sys::EventLoopCtrl;
use pd::sys::ffi::{PDSystemEvent, PlaydateAPI};
use bevy_playdate::event::SystemEvent;
use pd::system::update::{Update, UpdateCtrl};

/// Game state
struct State {
    app: App,
}

impl State {
    fn new() -> Self {
        let mut app = App::new();
        
        app.add_plugins(DefaultPlugins)
            .add_plugins(game::GamePlugin);

        Self { app }
    }

    /// System event handler
    fn event(&'static mut self, event: SystemEvent) -> EventLoopCtrl {
        match event {
            // Initial setup
            SystemEvent::Init => {
                // Set FPS to 30
                Display::Default().set_refresh_rate(50.0);

                // Register our update handler that defined below
                self.set_update_handler();

                println!("Game init complete");
            }
            // TODO: React to other events
            e => {
                self.app.world_mut().trigger(e);
            }
        }
        EventLoopCtrl::Continue
    }
}

impl Update for State {
    /// Updates the state
    fn update(&mut self) -> UpdateCtrl {
        // clear(Color::WHITE);

        // self.app.update();
        // self.app.run_system
        self.app.update();

        UpdateCtrl::Continue
    }
}

/// Entry point
#[no_mangle]
pub fn event_handler(
    _api: NonNull<PlaydateAPI>,
    event: PDSystemEvent,
    sim_key_code: u32,
) -> EventLoopCtrl {
    // Unsafe static storage for our state.
    // Usually it's safe because there's only one thread.
    let event = SystemEvent::from_event(event, sim_key_code);

    pub static mut STATE: OnceCell<State> = OnceCell::new();

    // Call state.event
    unsafe { STATE.get_mut_or_init(State::new).event(event) }
}


// Needed for debug build, absolutely optional
ll_symbols!();

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
use pd::sys::ffi::{LCDColor, PDSystemEvent, PlaydateAPI};
use bevy_playdate::event::SystemEvent;
use pd::graphics::bitmap::LCDColorConst;
use pd::graphics::draw_line;
use pd::sys::log::println;
use pd::system;
use pd::system::System;
use pd::system::update::{Update, UpdateCtrl};

/// Game state
struct State {
    app: App,
}

impl State {
    fn new() -> Self {
        let mut app = App::new();
        // Set FPS to 30
        Display::Default().set_refresh_rate(50.0);
        
        // app.add_plugins(DefaultPlugins)
        //     .add_plugins(game::GamePlugin);

        Self { app }
        // Self {}
    }

    /// System event handler
    fn event(&'static mut self, event: SystemEvent) -> EventLoopCtrl {
        match event {
            // Initial setup
            // SystemEvent::Init => {
            // 
            //     // Register our update handler that defined below
            //     // self.set_update_handler();
            //     
            // 
            //     // println!("Game init complete");
            // }
            // TODO: React to other events
            e => {
                // println!("test {:?}", event);
                // self.app.world_mut().trigger(e);
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
        // self.app.update();
        draw_line(0, 0, 100, 200, 10, LCDColor::BLACK);

        UpdateCtrl::Continue
    }
}


/// Entry point
#[no_mangle]
fn event_handler(api: NonNull<PlaydateAPI>, event: PDSystemEvent, _: u32) -> EventLoopCtrl {
    // SAFETY: nothing else had any chance to tamper with the PlaydateAPI so we can create OpaqueAPI
    // from it
    // let api = unsafe { OpaqueAPI::new(api) };

    match event {
        PDSystemEvent::kEventInit => {}
        // TODO: Handle other events here if we care about them
        _ => return EventLoopCtrl::Continue,
    }

    // SAFETY: bevydate_time::init gurantees that returned function pointer will give Durations
    // whenever called it is called, representing amount of time that has passed since program start
    // unsafe { Instant::set_elapsed(bevydate_time::init(api)) };

    let mut app = App::new();
    

    Display::Default().set_refresh_rate(50.);

    // Create cached end-points that we using every update
    let system = system::System::Cached();

    // Register update handler
    // Just to draw current playback position
    system.set_update_callback_boxed(
        move |_| {
            app.update();

            system.draw_fps(0, 0);

            UpdateCtrl::Continue
        },
        (),
    );

    EventLoopCtrl::Continue
}
// pub fn event_handler(
//     _api: NonNull<PlaydateAPI>,
//     event: PDSystemEvent,
//     sim_key_code: u32,
// ) -> EventLoopCtrl {
//     // Unsafe static storage for our state.
//     // Usually it's safe because there's only one thread.
//     let event = SystemEvent::from_event(event, sim_key_code);
//     
//     if event == SystemEvent::Init {
//         let mut state = State::new();
//         
//         System::Default().set_update_callback_boxed(
//             move |_| {
//                 state.update();
//                 
//                 UpdateCtrl::Continue
//             },
//             ()
//         )
//     }
//     
//     EventLoopCtrl::Continue
// 
//     // pub static mut STATE: OnceCell<State> = OnceCell::new();
//     
//     
//     // App::new();
//     // System::Cached().set_update_callback_boxed()
// 
//     // Call state.event
//     // #[allow(static_mut_refs)]
//     // unsafe { STATE.get_mut_or_init(State::new).event(event) }
// }


// Needed for debug build, absolutely optional
ll_symbols!();

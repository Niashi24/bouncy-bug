#![feature(once_cell_get_mut)]
#![feature(debug_closure_helpers)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate playdate as pd;
mod game;

use bevy_app::App;
use core::ptr::NonNull;
use pd::display::Display;
use pd::graphics::bitmap::LCDColorConst;
use pd::sys::ffi::{PDSystemEvent, PlaydateAPI};
use pd::sys::EventLoopCtrl;
use pd::system;
use pd::system::update::{Update, UpdateCtrl};

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


// Needed for debug build, absolutely optional
ll_symbols!();

use crate::debug::in_debug;
use alloc::format;
use bevy_app::{App, First, FixedUpdate, Plugin};
use bevy_ecs::prelude::{IntoSystemConfigs, ResMut, Resource};
use bevy_ecs::system::Res;
use bevy_platform_support::time::Instant;
use bevy_time::{Time, TimePlugin};
use core::cell::UnsafeCell;
use core::ffi::c_uint;
use core::time::Duration;
use playdate::graphics::bitmap::LCDColorConst;
use playdate::graphics::fill_rect;
use playdate::graphics::text::draw_text;
use playdate::sys::ffi::LCDColor;
use playdate::system::api::Cache;
use playdate::system::System;
use playdate::api;

pub struct PDTimePlugin;

impl Plugin for PDTimePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TimePlugin);
        unsafe {
            Instant::set_elapsed(init());
        }
        app
            .init_resource::<RunningTimer>()
            .add_systems(First, RunningTimer::update_system);
        app.add_systems(FixedUpdate, debug_time.run_if(in_debug));
    }
}

#[derive(Resource)]
pub struct RunningTimer {
    start_time: Duration,
    system: System<Cache>,
}

impl Default for RunningTimer {
    fn default() -> Self {
        Self {
            start_time: Duration::ZERO,
            system: System::Cached(),
        }
    }
}

impl RunningTimer {
    pub fn update_system(mut this: ResMut<Self>) {
        this.update();
    }
    
    pub fn update(&mut self) {
        self.start_time = self.system.current_time();
    }
    
    pub fn time_in_frame(&self) -> Duration {
        self.system.current_time() - self.start_time
    }
}

fn debug_time(time: Res<Time>, running: Res<RunningTimer>) {
    // println!("here3");
    fill_rect(0, 16, 60, 48, LCDColor::WHITE);
    draw_text(format!("t: {:.2}", time.delta_secs()), 0, 16).unwrap();
    draw_text(format!("p: {:.2}", System::Default().elapsed_time_secs()), 0, 32).unwrap();
    draw_text(format!("r: {:.2}", running.time_in_frame().as_secs_f32()), 0, 48).unwrap();
}

static GET_CURRENT_TIME: CurrentTimeFn = CurrentTimeFn(UnsafeCell::new(None));

struct CurrentTimeFn(UnsafeCell<Option<unsafe extern "C" fn() -> c_uint>>);

// SAFETY: Playdate is single threaded so data will never be synced between different threads
unsafe impl Sync for CurrentTimeFn {}

fn get_elapsed() -> Duration {
    // SAFETY: We are guranteed by [`init`] that we will never be called until
    // [`GET_CURRENT_TIME`] is initialized into a valid and correct function pointer.
    unsafe {
        let get_elapsed_ms = (*(GET_CURRENT_TIME.0.get())).unwrap_unchecked();
        Duration::from_millis(get_elapsed_ms().into())
    }
}

pub fn init() -> fn() -> Duration {
    
    let current_time_fn = api!(system).getCurrentTimeMilliseconds.expect("getCurrentTimeMilliseconds");
    // SAFETY: we are the only ones accessing [`GET_CURRENT_TIME`] since it's defined in this
    // module to be only accessed by this function and [`get_elapsed`]
    unsafe {
        *GET_CURRENT_TIME.0.get() = Some(current_time_fn);
    }

    get_elapsed
}

use crate::debug::in_debug;
use crate::sprite::SpriteSystemSet;
use alloc::format;
use bevy_app::{App, First, Plugin, PostUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, ResMut, Resource};
use bevy_ecs::system::Res;
use bevy_platform::time::Instant;
use bevy_time::{Time, TimePlugin};
use core::time::Duration;
use playdate::graphics::bitmap::LCDColorConst;
use playdate::graphics::fill_rect;
use playdate::graphics::text::draw_text;
use playdate::sys::ffi::LCDColor;
use playdate::system::System;
use playdate::system::api::Cache;

/// Whatever you do, do NOT call reset_elapsed_time.
/// Use the utilities from [`bevy_time`], such as [`bevy_time::Timer`]
pub struct PDTimePlugin;

impl Plugin for PDTimePlugin {
    fn build(&self, app: &mut App) {
        unsafe {
            Instant::set_elapsed(init());
        }
        app.add_plugins(TimePlugin);
        app.init_resource::<RunningTimer>()
            .add_systems(First, RunningTimer::update_system);
        app.add_systems(
            PostUpdate,
            debug_time.run_if(in_debug).after(SpriteSystemSet),
        );
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
        self.start_time = self.system.elapsed_time();
    }

    pub fn time_in_frame(&self) -> Duration {
        self.system.elapsed_time() - self.start_time
    }
}

fn debug_time(time: Res<Time>, running: Res<RunningTimer>) {
    // println!("here3");
    fill_rect(0, 16, 60, 48, LCDColor::WHITE);
    draw_text(format!("t: {:.2}", time.delta_secs()), 0, 16).unwrap();
    draw_text(
        format!("p: {:.2}", System::Default().elapsed_time_secs()),
        0,
        32,
    )
    .unwrap();
    draw_text(
        format!("r: {:.2}", running.time_in_frame().as_secs_f32()),
        0,
        48,
    )
    .unwrap();
}

pub fn init() -> fn() -> Duration {
    System::Default().reset_elapsed_time();

    fn get_elapsed() -> Duration {
        System::Default().elapsed_time()
    }

    get_elapsed
}

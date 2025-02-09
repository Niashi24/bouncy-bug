use bevy_app::{App, First, Plugin};
use bevy_ecs::prelude::{ResMut, Resource};
use core::time::Duration;
use playdate::system::api::Cache;
use playdate::system::System;

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>().add_systems(First, advance_time);
    }
}

#[derive(Resource)]
pub struct Time {
    now: Duration,
    delta: Duration,
    pub pd_time: System<Cache>,
}

impl Time {
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn elapsed(&self) -> Duration {
        self.pd_time.current_time()
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed().as_secs_f32()
    }
}

impl Default for Time {
    fn default() -> Self {
        let sys = System::Default();

        sys.reset_elapsed_time();

        Self {
            now: Duration::ZERO,
            delta: Duration::ZERO,
            pd_time: System::Cached(),
        }
    }
}

pub fn advance_time(mut time: ResMut<Time>) {
    let sys = System::Default();
    let dur = sys.elapsed_time();
    time.now += dur;
    time.delta = dur;

    sys.reset_elapsed_time();
}

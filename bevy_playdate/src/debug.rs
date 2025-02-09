use crate::event::SystemEvent;
use alloc::collections::VecDeque;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::observer::Trigger;
use bevy_ecs::prelude::{IntoSystemConfigs, Resource};
use bevy_ecs::system::{Res, ResMut};
use playdate::api;
use playdate::sprite::draw_sprites;
use playdate::sys::ffi::LCDColor;
use playdate::system::System;

#[macro_export]
macro_rules! dbg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `eprintln!`
    // will be malformed.
    () => {
        playdate::println!("[{}:{}:{}]", file!(), line!(), column!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                playdate::println!("[{}:{}:{}] {} = {:#?}",
                    file!(), line!(), column!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Debug>()
            .add_observer(toggle_debug_system)
            .add_systems(
                PostUpdate,
                draw_fps_top_left.after(draw_sprites).run_if(in_debug),
            );
    }
}

pub fn in_debug(debug: Res<Debug>) -> bool {
    debug.enabled
}

pub fn draw_fps_top_left() {
    System::Default().draw_fps(0, 0);
}

pub fn toggle_debug_system(trigger: Trigger<SystemEvent>, mut debug: ResMut<Debug>) {
    const BACKTICK: u32 = 96;
    if matches!(*trigger.event(), SystemEvent::KeyPressed(BACKTICK)) {
        debug.toggle_enabled();
    }
}

// pub fn debug_print

#[derive(Resource, Default)]
pub struct Debug {
    pub enabled: bool,
    command_queue: VecDeque<DebugCommand>,
}

impl Debug {
    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }
}

enum DebugCommand {
    Line {
        start: (i32, i32),
        end: (i32, i32),
        line_width: i32,
        color: LCDColor,
    },
    Circle {
        center: (i32, i32),
        radius: i32,
        line_width: i32,
        color: LCDColor,
        filled: bool,
    },
}

impl Debug {
    pub fn line(&mut self, start: (i32, i32), end: (i32, i32), line_width: i32, color: LCDColor) {
        self.command_queue.push_back(DebugCommand::Line {
            start,
            end,
            line_width,
            color,
        });
    }

    pub fn circle(
        &mut self,
        center: (i32, i32),
        radius: i32,
        line_width: i32,
        color: LCDColor,
        filled: bool,
    ) {
        self.command_queue.push_back(DebugCommand::Circle {
            center,
            radius,
            line_width,
            color,
            filled,
        });
    }

    pub fn draw(&mut self) {
        for command in self.command_queue.drain(..) {
            match command {
                DebugCommand::Line {
                    start,
                    end,
                    line_width,
                    color,
                } => unsafe {
                    api!(graphics).drawLine.unwrap()(
                        start.0, start.1, end.0, end.1, line_width, color,
                    );
                },
                DebugCommand::Circle {
                    center,
                    radius,
                    line_width,
                    color,
                    filled,
                } => {
                    if filled {
                        unsafe {
                            api!(graphics).fillEllipse.unwrap()(
                                center.0,
                                center.1,
                                radius * 2,
                                radius * 2,
                                0.0,
                                0.0,
                                color,
                            );
                        }
                    } else {
                        unsafe {
                            api!(graphics).drawEllipse.unwrap()(
                                center.0,
                                center.1,
                                radius * 2,
                                radius * 2,
                                line_width,
                                0.0,
                                0.0,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}

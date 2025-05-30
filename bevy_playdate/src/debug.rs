use crate::input::PlaydateButton;
use crate::jobs::Jobs;
use crate::sprite::Sprite;
use crate::time::RunningTimer;
use alloc::collections::VecDeque;
use bevy_app::{App, Last, Plugin, PostUpdate};
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource};
use bevy_ecs::system::{Query, Res, ResMut};
use bevy_input::ButtonInput;
use bevy_math::IVec2;
use core::time::Duration;
use playdate::graphics::bitmap::LCDColorConst;
use playdate::graphics::{Graphics, draw_line, fill_rect};
use playdate::sprite::draw_sprites;
use playdate::sys::ffi::LCDColor;
use playdate::system::System;
use playdate::{api, println};
use crate::view::DrawOffset;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Debug>().add_systems(
            PostUpdate,
            (
                toggle_debug_system,
                draw_fps_top_left.after(draw_sprites),
                // (debug_sprite)
                //     .after(draw_sprites)
                //     .run_if(in_debug),
            )
                .chain(),
        );
        app.add_plugins(FpsLinesPlugin);
    }
}

pub fn in_debug(debug: Res<Debug>) -> bool {
    debug.enabled
}

#[allow(dead_code)]
fn debug_sprite(sprite: Query<&Sprite>) {
    let graphics = Graphics::Cached();
    for spr in sprite.iter() {
        // let (x, y) = spr.position();
        let rect = spr.bounds();
        graphics.draw_rect(
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
            LCDColor::XOR,
        );
    }
}

pub fn draw_fps_top_left() {
    fill_rect(0, 0, 15, 12, LCDColor::WHITE);
    System::Default().draw_fps(0, 0);
}

pub fn toggle_debug_system(input: Res<ButtonInput<PlaydateButton>>, mut debug: ResMut<Debug>) {
    use PlaydateButton as PDB;
    const DEBUG_COMBO: [PDB; 3] = [PDB::Up, PDB::Left, PDB::B];
    if input.all_pressed(DEBUG_COMBO) && input.any_just_pressed(DEBUG_COMBO) {
        debug.toggle_enabled();
    }
}

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

pub struct FpsLinesPlugin;

impl Plugin for FpsLinesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FpsLines>().add_systems(
            Last,
            (
                FpsLines::push_frame_system,
                FpsLines::draw_system.run_if(in_debug),
            )
                .chain()
                .after(Jobs::run_jobs_system),
        );
    }
}

#[derive(Resource)]
pub struct FpsLines {
    frames: VecDeque<Duration>,
    max_frames: usize,
    display_scale: f32,
}

impl Default for FpsLines {
    fn default() -> Self {
        Self {
            frames: VecDeque::with_capacity(50),
            max_frames: 50,
            display_scale: 2000.0,
        }
    }
}

impl FpsLines {
    pub fn push(&mut self, delta: Duration) {
        if self.frames.len() >= self.max_frames {
            self.frames.pop_front();
        }

        if delta > Duration::from_millis(20) {
            println!(
                "spike: {:.2}ms ({:.1} frame(s) lost)",
                delta.as_secs_f32() * 1000.0,
                delta.as_secs_f32() / 0.02
            );
        }

        self.frames.push_back(delta);
    }

    pub fn draw(&self, bottom_right: IVec2) {
        let Some(max_frame) = self.frames.iter().max() else { return; };
        let height = (max_frame.as_secs_f32() * self.display_scale) as i32;
        fill_rect(
            bottom_right.x - self.frames.len() as i32,
            bottom_right.y - height,
            self.frames.len() as i32,
            height,
            LCDColor::WHITE,
        );
        
        let mut x = bottom_right.x;
        for frame in &self.frames {
            let height = (frame.as_secs_f32() * self.display_scale) as i32;
            draw_line(
                x,
                bottom_right.y,
                x,
                bottom_right.y - height,
                1,
                LCDColor::BLACK,
            );

            x -= 1;
        }
    }

    pub fn push_frame_system(mut fps: ResMut<Self>, timer: Res<RunningTimer>) {
        fps.push(timer.time_in_frame());
    }

    pub fn draw_system(fps: Res<Self>, draw_offset: Res<DrawOffset>) {
        fps.draw(draw_offset.bottom_right());
    }
}

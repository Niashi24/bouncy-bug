use crate::tiled::collision::{Collision, TileLayerCollision};
use crate::tiled::{JobCommandsExt, MapLoader, SpriteLoader, SpriteTableLoader};
use alloc::string::{String, ToString};
use alloc::{format, vec};
use bevy_app::{App, Last, Plugin, PostUpdate, Startup, Update};
use bevy_ecs::prelude::{
    Children, Commands, Component, Entity, In, IntoScheduleConfigs, Name, Query, Res, ResMut,
    Single, With,
};
use bevy_input::ButtonInput;
use bevy_math::{Rot2, Vec2};
use bevy_playdate::debug::in_debug;
use bevy_playdate::input::{CrankInput, PlaydateButton};
use bevy_playdate::jobs::{JobHandle, JobStatusRef, Jobs, JobsScheduler, WorkResult};
use bevy_playdate::sprite::Sprite;
use bevy_playdate::time::RunningTimer;
use bevy_playdate::transform::{GlobalTransform, Transform};
use bevy_playdate::view::Camera;
use bevy_time::Time;
use parry2d::query::ShapeCastOptions;
use pd::graphics::api::Cache;
use pd::graphics::color::{Color, LCDColorConst};
use pd::graphics::text::draw_text;
use pd::graphics::{fill_rect, Graphics, LineCapStyle};
use pd::sprite::draw_sprites;
use pd::sys::ffi::LCDColor;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, test_spawn_job)
            .add_systems(Update, move_camera)
            .add_systems(
                Last,
                (control_job, display_job)
                    .chain()
                    .after(Jobs::run_jobs_system),
            )
            .add_systems(
                PostUpdate,
                (debug_shape.run_if(in_debug)).after(draw_sprites),
            );
    }
}

fn test_spawn_job(mut commands: Commands, mut jobs: ResMut<JobsScheduler>) {
    commands.spawn(Sprite::new_from_draw(10, 10, Color::BLACK, |_| {}));

    commands.spawn(JobTestComponent {
        job: jobs.add(0, TestJob(0), test_job),
    });
    commands.spawn(JobTestComponent {
        job: jobs.add(0, TestJob(5000), test_job),
    });
    commands.spawn(JobTestComponent {
        job: jobs.add(1, TestJob(2000), test_job),
    });
    commands.spawn(JobTestComponent {
        job: jobs.add(1, TestJob(6000), test_job),
    });

    commands
        .spawn((Name::new("Test sprite"), Transform::from_xy(20.0, 200.0)))
        .insert_loading_asset(
            SpriteTableLoader {
                sprite_loader: SpriteLoader::default(),
                index: 2,
            },
            0,
            "assets/tiles",
        );
}

fn display_job(q_test: Query<&JobTestComponent>, jobs: Res<Jobs>, timer: Res<RunningTimer>) {
    let mut y = 64;
    fill_rect(64, y, 150, 16, LCDColor::WHITE);
    draw_text(
        format!("r: {:.3}ms", timer.time_in_frame().as_secs_f32() * 1000.0),
        64,
        y,
    )
    .unwrap();

    y += 16;
    for test in q_test.iter() {
        let progress = jobs.progress(&test.job);

        fill_rect(64, y, 150, 16, LCDColor::WHITE);
        match progress {
            Some(JobStatusRef::InProgress(counter)) => {
                draw_text(format!("current: {}", counter.0), 64, y).unwrap();
            }
            Some(JobStatusRef::Success(())) => {
                draw_text("finished".to_string(), 64, y).unwrap();
            }
            _ => {
                draw_text("in progress", 64, y).unwrap();
            }
        }

        y += 16;
    }
}

fn debug_shape(tile_layer_collision: Query<(&TileLayerCollision, &GlobalTransform)>) {
    let draw = Graphics::new_with(Cache::default());
    for (layer, transform) in tile_layer_collision.iter() {
        for (_, shape) in layer.0.shapes() {
            let segment = shape.as_segment().unwrap();
            let mut a = segment.a;
            a.x += transform.x;
            a.y += transform.y;
            let mut b = segment.b;
            b.x += transform.x;
            b.y += transform.y;
            draw.draw_line(
                a.x as i32,
                a.y as i32,
                b.x as i32,
                b.y as i32,
                2,
                LCDColor::XOR,
            );
        }
    }
}

#[allow(dead_code)]
fn test_ray(
    collision: Collision,
    camera: Query<&GlobalTransform, With<Camera>>,
    input: Res<CrankInput>,
) {
    let graphics = Graphics::new_with(Cache::default());
    graphics.set_line_cap_style(LineCapStyle::kLineCapStyleRound);
    let rot = Rot2::from(input.angle.to_radians());
    let rot = Vec2::new(rot.cos, rot.sin);

    let distance = 400.0;

    for camera in camera {
        let camera = camera.0;

        let hit = collision.circle_cast(
            camera,
            12.0,
            rot,
            ShapeCastOptions {
                max_time_of_impact: distance,
                ..ShapeCastOptions::default()
            },
        );

        let mut rays = collision.move_and_slide(
            camera,
            rot,
            12.0,
            ShapeCastOptions {
                max_time_of_impact: distance,
                ..ShapeCastOptions::default()
            },
        );

        let mut last_point = camera;
        while let Ok(_hit) = rays.fire() {
            graphics.draw_line(
                last_point.x as i32,
                last_point.y as i32,
                rays.pos.x as i32,
                rays.pos.y as i32,
                24,
                LCDColor::BLACK,
            );

            last_point = rays.pos;
        }
        graphics.draw_line(
            last_point.x as i32,
            last_point.y as i32,
            rays.pos.x as i32,
            rays.pos.y as i32,
            24,
            LCDColor::BLACK,
        );

        if collision.overlap_circle(camera, 12.0).is_some() {
            graphics.fill_ellipse(
                camera.x as i32 - 12,
                camera.y as i32 - 12,
                24,
                24,
                0.0,
                0.0,
                LCDColor::XOR,
            );
        }
    }
}

fn control_job(
    q_test: Query<(Entity, &JobTestComponent)>,
    mut jobs: ResMut<Jobs>,
    mut scheduler: ResMut<JobsScheduler>,
    mut commands: Commands,
    input: Res<ButtonInput<PlaydateButton>>,
) {
    if input.just_pressed(PlaydateButton::A) {
        commands
            .spawn(JobTestComponent {
                job: scheduler.add(100, TestJob(9500), test_job),
            })
            .insert((Name::new("Map"), Transform::from_xy(100.0, 20.0)))
            .insert_loading_asset(MapLoader, -10, "assets/level-1.tmb");
    }

    if input.just_pressed(PlaydateButton::B) {
        jobs.clear_all();
        for (e, _) in q_test.iter() {
            commands.entity(e).despawn();
        }
    }
}

fn move_camera(
    mut camera: Option<Single<(&mut Transform, &GlobalTransform), With<Camera>>>,
    input: Res<ButtonInput<PlaydateButton>>,
    time: Res<Time>,
    collision: Collision,
) {
    let Some(camera) = camera else {
        return;
    };

    let mut x = 0;
    x += input.pressed(PlaydateButton::Right) as i32;
    x -= input.pressed(PlaydateButton::Left) as i32;
    let mut y = 0;
    y += input.pressed(PlaydateButton::Down) as i32;
    y -= input.pressed(PlaydateButton::Up) as i32;

    if x != 0 || y != 0 {
        let (mut camera, transform) = camera.into_inner();
        let vel = Vec2::new(x as f32, y as f32).normalize() * 150.0;
        let mut move_and_slide = collision.move_and_slide(
            transform.0,
            vel,
            12.0,
            ShapeCastOptions {
                max_time_of_impact: time.delta_secs(),
                ..ShapeCastOptions::default()
            },
        );

        while let Ok(_hit) = move_and_slide.fire() {}

        // safety check:
        let new_pos = if let Some((_, contact)) = collision.contact(move_and_slide.pos, 12.0) {
            let translation = contact.dist * Vec2::new(contact.normal1.x, contact.normal1.y);
            move_and_slide.pos + translation
        } else {
            move_and_slide.pos
        };
        let displacement = new_pos - transform.0;
        camera.0 += displacement;
    }
}

fn print_recursive(
    level: usize,
    entity: Entity,
    q_name: &Query<((&Name, Option<&GlobalTransform>), Option<&Children>)>,
) {
    let ((name, pos), children) = q_name.get(entity).unwrap();
    let pos_str = pos.map(|s| s.0).map(|p| format!(" @ {:?}", p));
    println!(
        "{}↳ {}{}",
        String::from_utf8(vec![b' '; level * 2]).unwrap(),
        name,
        pos_str.unwrap_or_default()
    );
    let Some(children) = children else {
        return;
    };
    for &child in children {
        print_recursive(level + 1, child, q_name);
    }
}

#[derive(Component)]
struct JobTestComponent {
    pub job: JobHandle<TestJob, (), ()>,
}

#[derive(Default)]
struct TestJob(pub i32);

fn test_job(counter: In<TestJob>) -> WorkResult<TestJob, (), ()> {
    let mut counter = counter.0;
    counter.0 += 1;

    if counter.0 >= 10000 {
        WorkResult::Success(())
    } else {
        WorkResult::Continue(counter)
    }
}

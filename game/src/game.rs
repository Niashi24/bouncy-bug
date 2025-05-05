use bevy_ecs::prelude::{EntityCommands, ReflectComponent};
use crate::tiled::collision::{Collision, TileLayerCollision};
use crate::tiled::spawn::MapHandle;
use crate::tiled::{AssetLoader, JobCommandsExt, Loading, Map, MapLoader, SpriteLoader, SpriteTableLoader};
use alloc::string::String;
use alloc::{format, vec};
use alloc::sync::Arc;
use core::ops::DerefMut;
use bevy_app::{App, Last, Plugin, PostUpdate, Startup, Update};
use bevy_ecs::component::HookContext;
use bevy_ecs::prelude::{Children, Commands, Component, Entity, IntoScheduleConfigs, Name, Query, Res, ResMut, Single, With};
use bevy_ecs::world::DeferredWorld;
use bevy_input::ButtonInput;
use bevy_math::{Rot2, Vec2};
use bevy_reflect::Reflect;
use bevy_state::prelude::{in_state, NextState, OnEnter, OnExit};
use bevy_playdate::debug::{in_debug, Debug};
use bevy_playdate::input::{CrankInput, PlaydateButton};
use bevy_playdate::jobs::{Jobs, JobsScheduler};
use bevy_playdate::sprite::Sprite;
use bevy_playdate::time::RunningTimer;
use bevy_playdate::transform::{GlobalTransform, Transform, TransformSystem};
use bevy_playdate::view::{Camera, DrawOffset};
use bevy_time::Time;
use parry2d::query::ShapeCastOptions;
use pd::graphics::api::Cache;
use pd::graphics::color::{Color, LCDColorConst};
use pd::graphics::text::draw_text;
use pd::graphics::{fill_rect, Graphics, LineCapStyle};
use pd::sprite::draw_sprites;
use pd::sys::ffi::LCDColor;
use bevy_playdate::asset::{AssetAsync, AssetCache, ResAssetCache};
use bevy_playdate::visibility::Visibility;
use diagnostic::dbg;
use crate::state::LoadingState;
use crate::tiled::export::PathField;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_title_screen)
            .add_systems(Update, move_camera)
            .add_systems(
                Last,
                (control_job, display_timer)
                    .chain()
                    .after(Jobs::run_jobs_system),
            )
            .add_systems(
                PostUpdate,
                (debug_collision.run_if(in_debug)).after(draw_sprites),
            )
            .register_type::<MapLoad>();
        
        // app
        //     .add_systems(OnEnter(LoadingState::StartLoading), || println!("enter start loading"))
        //     .add_systems(OnExit(LoadingState::StartLoading), || println!("exit start loading"))
        //     .add_systems(OnEnter(LoadingState::Loading), || println!("enter start loading"))
        //     .add_systems(OnExit(LoadingState::Loading), || println!("exit start loading"))
        //     .add_systems(OnEnter(LoadingState::EndLoading), || println!("enter start loading"))
        //     .add_systems(OnExit(LoadingState::EndLoading), || println!("exit start loading"))
        //     .add_systems(OnEnter(LoadingState::NotLoading), || println!("enter start loading"))
        //     .add_systems(OnExit(LoadingState::NotLoading), || println!("exit start loading"));
        
        app
            .add_plugins(crate::state::StatesPlugin)
            .add_systems(Startup, spawn_loading_transition)
            .add_systems(OnEnter(LoadingState::Loading), spawn_despawn_map)
            .add_systems(OnEnter(LoadingState::StartLoading), start_transition_in)
            .add_systems(OnEnter(LoadingState::EndLoading), start_transition_out)
            .add_systems(Update, move_screen_transition)
            .add_systems(Last, move_after_loading
                .run_if(in_state(LoadingState::Loading))
                .after(Jobs::run_jobs_system)
            );
    }
}

fn spawn_title_screen(mut commands: Commands) {
    commands.spawn((
        Name::new("Title screen"),
        Transform::from_xy(-4.0, 0.0),
    ))
        .insert_loading_asset(MapLoader, -100, "assets/title-screen.tmb");
}

#[derive(Component, Clone, Default)]
struct ScreenTransition {
    state: ScreenTransitionState
}

#[derive(Clone, Default)]
enum ScreenTransitionState {
    #[default]
    Inactive,
    MoveIn {
        t: f32,
    },
    Stay,
    MoveOut {
        t: f32,
    },
}

fn spawn_loading_transition(mut commands: Commands) {
    commands.spawn((
        Name::new("Screen Transition"),
        Transform::default(),
        Visibility::Hidden,
        ScreenTransition::default(),
    ))
        .insert_loading_asset(SpriteLoader {
            center: [1.0, 0.0],
            z_index: 10000,
            ignore_draw_offset: true,
        }, 0, "assets/transition-simple.pdi");
}

fn start_transition_in(
    transition: Single<(&mut ScreenTransition, &mut Visibility, &mut Transform)>,
) {
    let (mut transition, mut visibility, mut transform) = transition.into_inner();
    transition.state = ScreenTransitionState::MoveIn { t: 0.0 };
    transform.x = 0.0;
    *visibility = Visibility::Visible;
}

fn start_transition_out(
    transition: Single<&mut ScreenTransition>,
) {
    let mut transition = transition.into_inner();
    transition.state = ScreenTransitionState::MoveOut { t: 0.0 };
}

fn move_screen_transition(
    transition: Single<(&mut ScreenTransition, &mut Visibility, &mut Transform)>,
    time: Res<Time>,
    mut loading_state: ResMut<NextState<LoadingState>>,
) {
    let (mut transition, mut visibility, mut transform) = transition.into_inner();
    
    let initial = 0.0;
    let target_in = 520.0;
    let target_out = 1040.0;
    
    fn eval(t: f32, start: f32, end: f32) -> f32 {
        bevy_math::FloatExt::lerp(start, end, t)
    }
    
    let transition_time = 0.5;
    
    match &mut transition.state {
        ScreenTransitionState::Inactive => {}
        ScreenTransitionState::Stay => {}
        ScreenTransitionState::MoveIn { t } => {
            *t = (*t + time.delta_secs() / transition_time).min(1.0);
            if *t == 1.0 {
                transform.x = target_in;
                transition.state = ScreenTransitionState::Stay;
                loading_state.set(LoadingState::Loading);
            } else {
                let x = eval(*t, initial, target_in);
                transform.x = x;
            }
        }
        ScreenTransitionState::MoveOut { t } => {
            *t = (*t + time.delta_secs() / transition_time).min(1.0);
            if *t == 1.0 {
                transform.x = target_out;
                transition.state = ScreenTransitionState::Inactive;
                *visibility = Visibility::Hidden;
                loading_state.set(LoadingState::NotLoading);
            } else {
                let x = eval(*t, target_in, target_out);
                transform.x = x;
            }
        }
    }
}

fn move_after_loading(
    q_loading: Query<(), With<Loading>>,
    mut next_loading: ResMut<NextState<LoadingState>>,
) {
    if q_loading.is_empty() {
        next_loading.set(LoadingState::EndLoading);
    }
}

fn display_timer(timer: Res<RunningTimer>) {
    fill_rect(64, 64, 150, 16, LCDColor::WHITE);
    draw_text(
        format!("r: {:.3}ms", timer.time_in_frame().as_secs_f32() * 1000.0),
        64,
        64,
    )
    .unwrap();
    
}

fn debug_collision(tile_layer_collision: Query<(&TileLayerCollision, &GlobalTransform)>) {
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

fn spawn_despawn_map(
    q_test: Query<(Entity, &MapHandle)>,
    mut commands: Commands,
) {
    commands.spawn((
        Name::new("Level 1"),
        Transform::default(),
    ))
        .insert_loading_asset(MapLoader, 0, "assets/level-1.tmb");

    for (e, _) in q_test.iter() {
        commands.entity(e).despawn();
    }
}

fn control_job(
    input: Res<ButtonInput<PlaydateButton>>,
    debug: Res<Debug>,
    assets: Res<ResAssetCache>,
    mut loading_state: ResMut<NextState<LoadingState>>,
) {
    if input.just_pressed(PlaydateButton::Down) && debug.enabled {
        assets.0.try_read().unwrap().debug_loaded();
    }
    
    if input.just_pressed(PlaydateButton::A) {
        loading_state.set(LoadingState::StartLoading);
    }
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[component(
    on_insert = load,
)]
pub struct MapLoad {
    pub path: PathField,
    pub priority: i32,
}

pub fn load(mut world: DeferredWorld, hook_context: HookContext) {
    let load = world.get::<MapLoad>(hook_context.entity).unwrap().clone();
    
    world.commands().entity(hook_context.entity)
        .insert_loading_asset(MapLoadLoader, load.priority as isize, load.path.0)
        .remove::<MapLoad>();
}

pub struct MapLoadLoader;

impl AssetLoader for MapLoadLoader {
    type Asset = Map;

    fn on_finish_load(&self, commands: &mut EntityCommands, result: Result<Arc<Self::Asset>, <<Self as AssetLoader>::Asset as AssetAsync>::Error>) {
        match result {
            Ok(map) => { commands.insert(MapHandle(map)); },
            Err(err) => {
                dbg!(err);
            }
        }
    }
}

fn move_camera(
    camera: Option<Single<(&mut Transform, &GlobalTransform), With<Camera>>>,
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

#[allow(dead_code)]
#[allow(clippy::type_complexity)]
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

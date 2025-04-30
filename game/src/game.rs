use alloc::{format, vec};
use alloc::string::{String, ToString};
use bevy_app::{App, Last, Plugin, PostUpdate, Startup, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::{Children, Component, IntoScheduleConfigs, With};
use bevy_ecs::system::{Commands, In, Query, Res, ResMut, Single};
use bevy_input::ButtonInput;
use bevy_math::Vec2;
use bevy_time::Time;
use bevy_playdate::transform::{GlobalTransform, Transform};
use bevy_playdate::asset::ResAssetCache;
use bevy_playdate::input::PlaydateButton;
use bevy_playdate::jobs::{JobHandle, JobStatusRef, Jobs, JobsScheduler, WorkResult};
use bevy_playdate::sprite::Sprite;
use bevy_playdate::time::RunningTimer;
use pd::graphics::color::{Color, LCDColorConst};
use pd::graphics::{fill_rect, Graphics};
use pd::graphics::api::Cache;
use pd::graphics::text::draw_text;
use pd::sprite::draw_sprites;
use pd::sys::ffi::LCDColor;
use bevy_playdate::debug::{in_debug, Debug};
use bevy_playdate::view::Camera;
use diagnostic::dbg;
use crate::tiled::{JobCommandsExt, Map, MapLoader, SpriteLoader, SpriteTableLoader, TiledMap, TiledSet};
use crate::tiled::spawn::{MapHandle, TileLayerCollision};
// use crate::pdtiled::loader::TiledLoader;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, test_spawn_job);
        // app.add_systems(Startup, draw_test);
        // app.add_systems(Update, draw_text_test);
        app.add_systems(Update, move_camera);
        app.add_systems(Last, (control_job, display_job).chain().after(Jobs::run_jobs_system));
        app.add_systems(PostUpdate, debug_shape.after(draw_sprites).run_if(in_debug));
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
    
    commands.spawn((
        Name::new("Test sprite"),
        Transform::from_xy(20.0, 200.0),
    ))
        .insert_loading_asset(SpriteTableLoader {
            sprite_loader: SpriteLoader::default(),
            index: 2,
        }, 0, "assets/tiles");
}



fn display_job(q_test: Query<&JobTestComponent>, jobs: Res<Jobs>, timer: Res<RunningTimer>) {
    
    let mut y = 64;
    fill_rect(64, y, 150, 16, LCDColor::WHITE);
    draw_text(format!("r: {:.3}ms", timer.time_in_frame().as_secs_f32() * 1000.0), 64, y).unwrap();
    
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

fn debug_shape(
    tile_layer_collision: Query<(&TileLayerCollision, &GlobalTransform)>,
) {
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
            draw.draw_line(a.x as i32, a.y as i32, b.x as i32, b.y as i32, 1, LCDColor::XOR);
        }
    }
}

fn control_job(
    q_test: Query<(Entity, &JobTestComponent)>,
    mut jobs: ResMut<Jobs>,
    mut scheduler: ResMut<JobsScheduler>,
    mut commands: Commands,
    input: Res<ButtonInput<PlaydateButton>>,
    asset_cache: Res<ResAssetCache>,
    q_map_root: Query<(&Name, &Children), With<MapHandle>>,
    q_name: Query<((&Name, Option<&GlobalTransform>), Option<&Children>)>,
    q_transform: Query<&GlobalTransform>,
    q_sprite: Query<&Sprite>,
    debug: Res<Debug>,
    camera: Query<&Transform, With<Camera>>,
) {
    if input.just_pressed(PlaydateButton::A) {
        
        commands.spawn(JobTestComponent {
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
        // if let Some((e, job)) = q_test.iter().next() {
        //     jobs.cancel(&job.job);
        //     commands.entity(e).despawn();
        // }
    }
    
    
    // if input.just_pressed(PlaydateButton::Down) && debug.enabled {
    //     println!("here");
    //     for (name, children) in q_map_root {
    //         println!("{}", name);
    //         for &child in children {
    //             print_recursive(0, child, &q_name);
    //         }
    //     }
    //     asset_cache.0.try_read().unwrap().debug_loaded();
    //     dbg!(q_transform.iter().len());
    //     dbg!(q_sprite.iter().len());
    //     if let Ok(camera) = camera.single() {
    //         dbg!(camera.0);
    //     }
    // }
    // let mut file = FileHandle::read_only("assets/test-map.tmx").unwrap();
    // let mut bytes = Vec::new();
    // file.read_to_end(&mut bytes).unwrap();
    // let s = String::from_utf8(bytes).unwrap();
    // // println!("{s}");
    // for line in s.lines() {
    //     println!("{line}");
    // }
    // let mut reader = EventReader::new(FileHandle::read_only("assets/test-map.tmx").unwrap());
    // for event in reader.into_iter() {
    //     println!("{event:?}");
    // }
}

fn move_camera(
    mut camera: Option<Single<&mut Transform, With<Camera>>>,
    input: Res<ButtonInput<PlaydateButton>>,
    time: Res<Time>
) {
    let Some(mut camera) = camera else { return; };
    
    let mut x = 0;
    x += input.pressed(PlaydateButton::Right) as i32;
    x -= input.pressed(PlaydateButton::Left) as i32;
    let mut y = 0;
    y += input.pressed(PlaydateButton::Down) as i32;
    y -= input.pressed(PlaydateButton::Up) as i32;
    
    // avoid deref_mut
    if x != 0 || y != 0 {
        camera.0 += Vec2::new(x as f32, y as f32) * time.delta_secs() * 100.0;
    }
}

fn print_recursive(
    level: usize,
    entity: Entity,
    q_name: &Query<((&Name, Option<&GlobalTransform>), Option<&Children>)>,
) {
    let ((name, pos), children) = q_name.get(entity).unwrap();
    let pos_str = pos.map(|s| s.0).map(|p| format!(" @ {:?}", p));
    println!("{}↳ {}{}", String::from_utf8(vec![b' '; level * 2]).unwrap(), name, pos_str.unwrap_or_default());
    let Some(children) = children else { return; };
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

// enum JobTest {
//     
// }

// #[derive(Component)]
// pub struct TextTest {
//     text: String,
// }
// 
// fn draw_test(mut loader: TiledLoader, mut commands: Commands) {
    // Loader::with_reader()
    // commands.
    // let mut file = FileHandle::read_only("assets/test-map.tmx").unwrap();
    // let mut bytes = Vec::new();
    // file.read_to_end(&mut bytes).unwrap();
    // let s = String::from_utf8(bytes).unwrap();
    // // println!("{s}");
    // for line in s.lines() {
    //     println!("{line}");
    // }
    
    // let tileset = loader.load_tmx_map("assets/test-map.tmx").unwrap();
    // println!("{:?}", tileset.tilesets());
    // loader.0.0.lock().unwrap().debug_loaded();
    // // // // println!("{:?}", tileset);
    // println!("{:?}", tileset);
    // black_box(&tileset);
    
    
    // 
    // let mut file = FileHandle::write_only("test.txt", false).unwrap();
    // let mut writer = BufferedWriter::<_, 512>::new(file);
    // writer.write_fmt(format_args!("{:?}", tileset)).unwrap();
    // println!("wrote tilemap to file")
    // let mut x = BufWriter::<_, 1000>::new(file);
    // let out = format!("{:?}", tileset);
    // file.write(out.as_bytes()).unwrap();
    
    // commands.spawn(TextTest {
    //     text: format!("{:?}", tileset),
    // });
    
    // commands.spawn(Sprite::new());
// }

// fn draw_text_test(
//     input: Res<CrankInput>,
//     texts: Query<&TextTest>,
// ) {
//     let t = input.angle / 360.0;
//     const CHARS_EACH: usize = 40;
//     let mut y = 0;
//     for text in texts {
//         let idx = (text.text.len() as f32 * t) as usize;
//         let txt = text.text.split_at(idx).1;
//         let txt = if txt.len() < CHARS_EACH {
//             txt
//         } else {
//             txt.split_at(CHARS_EACH).0
//         };
//         draw_text(txt, 20, y).unwrap();
//         
//         y += 20;
//     }
// }
// 
// fn crank_test(input: Res<CrankInput>) {
//     draw_line(10 + input.angle as i32, 50, 10 + input.angle as i32 + 100, 70, 5, LCDColor::XOR);
//     
//     draw_ellipse(100, 20, 200, 200, 5, input.angle + 10.0, input.angle - 10.0, LCDColor::XOR);
// }

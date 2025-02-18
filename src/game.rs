use alloc::format;
use alloc::string::ToString;
use core::cell::RefMut;
use bevy_app::{App, Last, Plugin, Startup, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Component, IntoSystemConfigs};
use bevy_ecs::system::{Commands, In, Query, Res, ResMut};
use bevy_input::ButtonInput;
use no_std_io2::io::Write;
use pd::graphics::color::{Color, LCDColorConst};
use pd::graphics::fill_rect;
use pd::graphics::text::draw_text;
use pd::sys::ffi::LCDColor;
use bevy_playdate::file::{BufferedWriter, FileHandle};
use bevy_playdate::input::PlaydateButton;
use bevy_playdate::jobs::{JobHandle, JobStatusRef, Jobs, JobsScheduler, WorkResult};
use bevy_playdate::sprite::Sprite;
use bevy_playdate::time::RunningTimer;
use crate::tiled::job::TiledJobExt;
use crate::tiled::loader::TiledLoader;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, test_spawn_job);
        app.add_systems(Startup, draw_test);
        // app.add_systems(Update, draw_text_test);
        app.add_systems(Last, (control_job, display_job).chain().after(Jobs::run_jobs_system));
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

fn control_job(
    q_test: Query<(Entity, &JobTestComponent)>,
    mut jobs: ResMut<Jobs>,
    mut scheduler: ResMut<JobsScheduler>,
    mut commands: Commands,
    input: Res<ButtonInput<PlaydateButton>>
) {
    if input.just_pressed(PlaydateButton::A) {
        // let _ = scheduler.load_tilemap("assets/test-map.tmx");
        commands.spawn(JobTestComponent {
            job: scheduler.add(1, TestJob(6000), test_job),
        });
    }
    
    if input.just_pressed(PlaydateButton::B) {
        
        if let Some((e, job)) = q_test.iter().next() {
            jobs.cancel(&job.job);
            commands.entity(e).despawn();
        }
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
fn draw_test(mut loader: TiledLoader, mut commands: Commands) {
    // Loader::with_reader()
    // commands.
    // let tileset = loader.load_tmx_map("assets/tiles.tsx").unwrap();
    // // // println!("{:?}", tileset);
    // 
    // let mut file = FileHandle::write_only("test.txt", false).unwrap();
    // let mut writer = BufferedWriter::<_, 1024>::new(file);
    // writer.write_fmt(format_args!("{:?}", tileset)).unwrap();
    // println!("wrote tilemap to file")
    // let mut x = BufWriter::<_, 1000>::new(file);
    // let out = format!("{:?}", tileset);
    // file.write(out.as_bytes()).unwrap();
    
    // commands.spawn(TextTest {
    //     text: format!("{:?}", tileset),
    // });
    
    // commands.spawn(Sprite::new());
}

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

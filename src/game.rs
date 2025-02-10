use crate::tiled::TiledLoader;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::system::Commands;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_test);
        // app.add_systems(Update, draw_text_test);
    }
}

struct JobTestComponent {
    // pub job: JobHandle<JobTest, >,
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
    // let tileset = loader.load_tsx_tileset("tiled/tiles.tsx").unwrap();
    // // println!("{:?}", tileset);
    
    // let mut file = FileHandle::write_only("test.txt", false).unwrap();
    // // let mut x = BufWriter::<_, 1000>::new(file);
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

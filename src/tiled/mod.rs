use bevy_app::{App, Plugin};
use bevy_ecs::system::NonSendMut;
use tiled::{DefaultResourceCache, Loader};

mod io;

pub type TiledLoader<'w> = NonSendMut<'w, Loader<io::PDTiledReader, DefaultResourceCache>>;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource(Loader::with_reader(io::PDTiledReader));
    }
}

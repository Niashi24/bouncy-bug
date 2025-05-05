use bevy_app::{App, Plugin};
use bevy_state::prelude::{AppExtStates, States};

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(bevy_state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_state::<LoadingState>();
    }
}

#[derive(States, Eq, PartialEq, Hash, Debug, Copy, Clone, Default)]
pub enum AppState {
    #[default]
    Title,
    Game,
}

#[derive(States, Eq, PartialEq, Hash, Debug, Copy, Clone, Default)]
pub enum LoadingState {
    /// The game is currently playing.
    #[default]
    NotLoading,
    /// Start of transition into loading state
    StartLoading,
    /// Loading is currently working. Screen should be blanked out at this time.
    Loading,
    /// End of transition out of loading
    EndLoading,
}
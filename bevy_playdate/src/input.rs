use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::reflect::ReflectResource;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoSystemConfigs, SystemSet};
use bevy_ecs::system::{NonSend, ResMut};
use bevy_input::ButtonInput;
use bevy_reflect::prelude::{Reflect, ReflectDefault};
use playdate::controls::api::Cache;
use playdate::controls::buttons::PDButtonsExt;
use playdate::controls::peripherals::{Accelerometer, Crank, SystemExt};
use playdate::system::System;

/// Adds crank input (and once `bevy_input` is `no_std`, d-pad and buttons)
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource(Crank::Cached())
            .init_resource::<CrankInput>()
            .init_resource::<ButtonInput<PlaydateButton>>()
            .insert_non_send_resource(Accelerometer::Cached())
            .init_resource::<AccelerometerInput>()
            .register_type::<CrankInput>()
            .register_type::<AccelerometerInput>()
            .add_systems(
                PreUpdate,
                (
                    button_input_system,
                    crank_input_system,
                    accelerometer_input_system,
                )
                    .in_set(PdInputSystem),
            );
    }
}

/// Label for systems that update the input data.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct PdInputSystem;

/// Enumeration of Playdate button-like inputs, including both
/// the A and B buttons but also the d-pad.
///
/// Use with `Res<ButtonInput<PlaydateButton>>` to get the current input.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum PlaydateButton {
    A,
    B,
    Up,
    Left,
    Right,
    Down,
}

pub fn button_input_system(mut input: ResMut<ButtonInput<PlaydateButton>>) {
    input.bypass_change_detection().clear();
    let buttons = System::Default().buttons().get();
    for (pressed, btn) in [
        (buttons.pushed.a(), PlaydateButton::A),
        (buttons.pushed.b(), PlaydateButton::B),
        (buttons.pushed.up(), PlaydateButton::Up),
        (buttons.pushed.down(), PlaydateButton::Down),
        (buttons.pushed.right(), PlaydateButton::Right),
        (buttons.pushed.left(), PlaydateButton::Left),
    ] {
        if pressed {
            input.press(btn);
        }
    }

    for (released, btn) in [
        (buttons.released.a(), PlaydateButton::A),
        (buttons.released.b(), PlaydateButton::B),
        (buttons.released.up(), PlaydateButton::Up),
        (buttons.released.down(), PlaydateButton::Down),
        (buttons.released.right(), PlaydateButton::Right),
        (buttons.released.left(), PlaydateButton::Left),
    ] {
        if released {
            input.release(btn);
        }
    }
}

/// A resource reporting the current input or state of the crank.
#[derive(Resource, Reflect, Copy, Clone, Debug, PartialEq, Default)]
#[reflect(Resource, Default)]
pub struct CrankInput {
    /// The angle change of the crank this frame.
    /// Negative values are anti-clockwise.
    pub change: f32,
    /// The current position of the crank, in the range 0-360.
    /// Zero is pointing up, and the value increases as the crank moves clockwise,
    /// as viewed from the right side of the device.
    pub angle: f32,
    /// Whether the crank is folded into the unit.
    pub docked: bool,
}

/// Updates the [`CrankInput`] resource with the latest [`Crank`] inputs.
pub fn crank_input_system(mut input: ResMut<CrankInput>, crank: NonSend<Crank<Cache>>) {
    input.change = crank.change();
    input.angle = crank.angle();
    input.docked = crank.docked();
}

/// A resource reporting the current input or state of the accelerometer.
#[derive(Resource, Reflect, Copy, Clone, Debug, PartialEq, Default)]
#[reflect(Resource, Default)]
pub struct AccelerometerInput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Updates the [`AccelerometerInput`] resource with the latest [`Accelerometer`] inputs.
pub fn accelerometer_input_system(
    mut input: ResMut<AccelerometerInput>,
    accelerometer: NonSend<Accelerometer<Cache>>,
) {
    (input.x, input.y, input.z) = accelerometer.get();
}

use bevy_ecs::event::Event;
use playdate::sys::ffi::PDSystemEvent;

#[must_use]
#[derive(Event, Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum SystemEvent {
    Init,
    InitLua,
    Lock,
    Unlock,
    Pause,
    Resume,
    Terminate,
    KeyPressed(u32),
    KeyReleased(u32),
    LowPower,
}

impl SystemEvent {
    pub fn from_event(event: PDSystemEvent, sim_key_code: u32) -> Self {
        match event {
            PDSystemEvent::kEventInit => Self::Init,
            PDSystemEvent::kEventInitLua => Self::InitLua,
            PDSystemEvent::kEventLock => Self::Lock,
            PDSystemEvent::kEventUnlock => Self::Unlock,
            PDSystemEvent::kEventPause => Self::Pause,
            PDSystemEvent::kEventResume => Self::Resume,
            PDSystemEvent::kEventTerminate => Self::Terminate,
            PDSystemEvent::kEventKeyPressed => Self::KeyPressed(sim_key_code),
            PDSystemEvent::kEventKeyReleased => Self::KeyReleased(sim_key_code),
            PDSystemEvent::kEventLowPower => Self::LowPower,
        }
    }
}

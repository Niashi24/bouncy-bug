use alloc::vec::Vec;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::entity::Entities;
use bevy_ecs::prelude::{Commands, In, ResMut, Resource, World};
use bevy_ecs::system::SystemParam;
use bevy_ecs::world::CommandQueue;
use bevy_playdate::jobs::{JobsScheduler, WorkResult};

pub struct BatchQueuePlugin;

impl Plugin for BatchQueuePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BatchCommandQueue>()
            .add_systems(Startup, add_job);
    }
}

#[derive(Resource, Default)]
pub struct BatchCommandQueue {
    empty_queues: Vec<CommandQueue>,
    active_queues: Vec<CommandQueue>,
}

fn add_job(mut scheduler: ResMut<JobsScheduler>) {
    let _ = scheduler.add(1000, (), BatchCommandQueue::commands_job_system);
}

impl BatchCommandQueue {
    pub fn commands<'w: 'a, 'a>(&'a mut self, entities: &'w Entities) -> Commands<'a, 'a> {
        let queue = self.empty_queues.pop().unwrap_or_default();
        self.active_queues.push(queue);
        let queue = self.active_queues.last_mut().unwrap();
        Commands::new_from_entities(queue, entities)
    }
    
    /// Applies the next command queue (if any), and returns true if there are more commands to process
    #[must_use]
    pub fn apply_next(&mut self, world: &mut World) -> bool {
        match self.active_queues.pop() {
            Some(mut queue) => {
                queue.apply(world);
                self.empty_queues.push(queue);
                
                !self.active_queues.is_empty()
            }
            None => false,
        }
    }
}

impl BatchCommandQueue {
    pub fn commands_job_system(In(()): In<()>, world: &mut World) -> WorkResult<(), (), ()> {
        world.resource_scope::<BatchCommandQueue, _>(|world, mut commands| {
            if commands.apply_next(world) {
                WorkResult::Continue(())
            } else {
                WorkResult::Skip(())
            }
        })
    }
}

#[derive(SystemParam)]
pub struct BatchCommands<'w> {
    entities: &'w Entities,
    queue: ResMut<'w, BatchCommandQueue>,
}

impl<'w> BatchCommands<'w> {
    pub fn commands(&mut self) -> Commands {
        self.queue.commands(self.entities)
    }
}

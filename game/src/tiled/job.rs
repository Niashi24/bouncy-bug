use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::entity::Entities;
use bevy_ecs::prelude::{Commands, Entity, In, ResMut, Resource, World};
use bevy_ecs::system::SystemParam;
use bevy_ecs::world::CommandQueue;
use bevy_playdate::jobs::{JobsScheduler, WorkResult};
use diagnostic::dbg;
use crate::tiled::Loading;

pub struct BatchQueuePlugin;

impl Plugin for BatchQueuePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BatchCommandQueue>()
            .add_systems(Startup, add_job);
    }
}

#[derive(Resource, Default)]
pub struct BatchCommandQueue {
    queues: Box<[CommandQueue; BatchCommandQueue::MAX_QUEUES]>,
    last_used: usize,
    loading_entity: Option<Entity>,
}

fn add_job(mut scheduler: ResMut<JobsScheduler>) {
    let _ = scheduler.add(-100, (), BatchCommandQueue::commands_job_system);
}

impl BatchCommandQueue {
    // worst case scenario it will take this many frames to finish
    const MAX_QUEUES: usize = 24;
    
    pub fn commands<'w: 'a, 'a>(&'a mut self, entities: &'w Entities) -> Commands<'a, 'a> {
        Commands::new_from_entities(self.next_queue(), entities)
    }
    
    fn next_queue(&mut self) -> &mut CommandQueue {
        self.last_used += 1;
        if self.last_used >= Self::MAX_QUEUES {
            self.last_used = 0;
        }
        
        &mut self.queues[self.last_used]
    }
    
    /// Applies the next command queue (if any), and returns true if there are more commands to process
    #[must_use]
    pub fn apply_next(&mut self, world: &mut World) -> bool {
        let queue = self.next_queue();
        
        queue.apply(world);
        
        self.queues.iter().any(|b| !b.is_empty())
    }
}

impl BatchCommandQueue {
    pub fn commands_job_system(In(()): In<()>, world: &mut World) -> WorkResult<(), (), ()> {
        world.resource_scope::<BatchCommandQueue, _>(|world, mut commands| {
            let loading = *commands.loading_entity.get_or_insert_with(|| world.spawn_empty().id());
            if commands.apply_next(world) {
                world.entity_mut(loading).insert(Loading);
                WorkResult::Continue(())
            } else {
                world.entity_mut(loading).remove::<Loading>();
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

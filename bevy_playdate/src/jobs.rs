use alloc::boxed::Box;
use alloc::collections::BinaryHeap;
use alloc::vec;
use alloc::vec::Vec;
use bevy_ecs::prelude::{In, Local, Mut, Resource};
use bevy_ecs::system::{BoxedSystem, IntoSystem, SystemId};
use bevy_ecs::world::World;
use core::any::Any;
use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::DerefMut;
use bevy_app::{App, Last, Plugin};
use derive_more::From;
use hashbrown::HashMap;
use crate::time::RunningTimer;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Jobs>()
            .init_resource::<JobsScheduler>();
        app.add_systems(Last, Jobs::run_jobs_system);
    }
}

#[derive(Resource, Default)]
pub struct JobsScheduler {
    id_gen: JobId,
    unstarted: Vec<UnstartedJob>,
}

unsafe impl Send for JobsScheduler {}
unsafe impl Sync for JobsScheduler {}

impl JobsScheduler {
    fn next_id(&mut self) -> JobId {
        let out = self.id_gen;
        self.id_gen += 1;
        out
    }
    
    pub fn add<Work: Any, Success: Any, Error: Any, M>(
        &mut self,
        priority: isize,
        initial: Work,
        job: impl IntoSystem<In<Work>, WorkResult<Work, Success, Error>, M>,
    ) -> JobHandle<Work, Success, Error> {
        let job = IntoSystem::into_system(pipe_any.pipe(job).map(ErasedWorkStatus::from));

        let id = self.next_id();
        let job = UnstartedJob {
            priority,
            work: Box::new(initial),
            id,
            job: Box::new(job),
        };

        self.unstarted.push(job);

        JobHandle {
            id,
            _phantom_data: Default::default(),
        }
    }
}

type JobId = usize;

#[derive(Resource)]
pub struct Jobs {
    /// Minimum number of jobs to run per frame
    pub min_jobs: usize,
    jobs: BinaryHeap<RunningJob>,
    finished: HashMap<JobId, FinishedJob>,
    to_cancel: Vec<RunningJob>,
}

type FinishedJob = Result<Box<dyn Any>, Box<dyn Any>>;

unsafe impl Send for Jobs {}
unsafe impl Sync for Jobs {}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            min_jobs: 5,
            jobs: BinaryHeap::new(),
            finished: Default::default(),
            to_cancel: vec![],
        }
    }
}

pub type Job<Work, Success, Error> = BoxedSystem<In<Work>, WorkResult<Work, Success, Error>>;

impl Jobs {
    pub fn progress<Work: Any, Success: Any, Error: Any>(
        &self,
        job: &JobHandle<Work, Success, Error>,
    ) -> Option<JobStatusRef<Work, Success, Error>> {
        if let Some(job) = self.finished.get(&job.id) {
            return Some(match job {
                Ok(val) => JobStatusRef::Success(val.downcast_ref().unwrap()),
                Err(val) => JobStatusRef::Error(val.downcast_ref().unwrap()),
            });
        }

        if let Some(job) = self.jobs.iter().find(|j| j.id == job.id) {
            return Some(JobStatusRef::InProgress(job.work.downcast_ref().unwrap()));
        }

        None
    }

    // fn understarted_jobs(&mut self) -> (&mut Vec<UnstartedJob>, &mut BinaryHeap<RunningJob>) {
    //     (&mut self.unstarted, &mut self.jobs)
    // }

    /// System to run jobs
    pub fn run_jobs_system(world: &mut World, mut skip_buffer: Local<Vec<RunningJob>>) {
        world.resource_scope(|world, mut jobs: Mut<Jobs>| {
            for job in jobs.to_cancel.drain(..) {
                world.unregister_system(job.job)
                    .expect("unregister canceled system");
            }
            
            world.resource_scope(|world, mut scheduler: Mut<JobsScheduler>| {
                for job in scheduler.unstarted.drain(..) {
                    let j = world.register_boxed_system(job.job);

                    jobs.jobs.push(RunningJob {
                        priority: job.priority,
                        work: job.work,
                        id: job.id,
                        job: j,
                    });
                }
            });

            for _ in 0..jobs.min_jobs {
                let continue_jobs = jobs.run_job(world, skip_buffer.deref_mut());
                if !continue_jobs {
                    break;
                }
            }
            
            // target hertz we want to meet
            // default is 50fps = 20ms = 0.02s, then let's give an extra 5ms of leeway
            const TARGET_HERTZ: f32 = 0.02 - 0.005;
            while world.resource::<RunningTimer>().time_in_frame().as_secs_f32() < TARGET_HERTZ {
                let continue_jobs = jobs.run_job(world, skip_buffer.deref_mut());
                if !continue_jobs {
                    break;
                }
            }

            for job in skip_buffer.deref_mut().drain(..) {
                jobs.jobs.push(job);
            }
        });
    }
    
    #[must_use]
    fn run_job(&mut self, world: &mut World, skip_buffer: &mut Vec<RunningJob>) -> bool {
        let Some(mut job) = self.jobs.pop() else {
            return false;
        };

        match world.run_system_with(job.job, job.work).unwrap() {
            ErasedWorkStatus::Continue(val) => {
                job.work = val;
                self.jobs.push(job);
            }
            ErasedWorkStatus::Skip(val) => {
                job.work = val;
                skip_buffer.push(job);
            }
            ErasedWorkStatus::Success(val) => {
                self.finished.insert(job.id, Ok(val));
                world.unregister_system(job.job)
                    .expect("unregister completed job (success)");
            }
            ErasedWorkStatus::Error(val) => {
                self.finished.insert(job.id, Err(val));
                world.unregister_system(job.job)
                    .expect("unregister completed job (error)");
            }
        }

        true
    }
    
    pub fn cancel<Work: Any, Success: Any, Error: Any>(
        &mut self,
        job: &JobHandle<Work, Success, Error>,
    ) {
        if self.try_claim(job).is_some() {
            return;
        }
        
        let mut v = core::mem::take(&mut self.jobs).into_vec();
        if let Some((i, _)) = v.iter().enumerate().find(|(_, j)| j.id == job.id) {
            let item = v.swap_remove(i);
            self.to_cancel.push(item);
        }
        self.jobs = BinaryHeap::from(v);
    }

    pub fn try_claim<Work: Any, Success: Any, Error: Any>(
        &mut self,
        job: &JobHandle<Work, Success, Error>,
    ) -> Option<Result<Success, Error>> {
        match self.finished.remove(&job.id) {
            None => None,
            Some(Ok(val)) => Some(Ok(*val.downcast().unwrap())),
            Some(Err(val)) => Some(Err(*val.downcast().unwrap())),
        }
    }
}


fn pipe_any<T: Any>(In(val): In<Box<dyn Any>>) -> T {
    *val.downcast().unwrap()
}

pub enum JobStatusRef<'a, Work, Success, Error> {
    NotStarted(&'a Work),
    InProgress(&'a Work),
    Success(&'a Success),
    Error(&'a Error),
}

pub struct JobHandle<Work, Success, Error> {
    id: usize,
    _phantom_data: PhantomData<(Work, Success, Error)>,
}

pub struct UnstartedJob {
    priority: isize,
    work: Box<dyn Any>,
    id: usize,
    job: BoxedSystem<In<Box<dyn Any>>, ErasedWorkStatus>,
}

pub struct RunningJob {
    priority: isize,
    work: Box<dyn Any>,
    id: usize,
    job: SystemId<In<Box<dyn Any>>, ErasedWorkStatus>,
}

unsafe impl Send for RunningJob {}

impl Eq for RunningJob {}

impl PartialEq<Self> for RunningJob {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd<Self> for RunningJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RunningJob {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

pub enum WorkResult<TWork, TSuccess, TError> {
    Continue(TWork),

    Skip(TWork),

    Success(TSuccess),

    Error(TError),
}

enum ErasedWorkStatus {
    Continue(Box<dyn Any>),

    Skip(Box<dyn Any>),

    Success(Box<dyn Any>),

    Error(Box<dyn Any>),
}

impl<TWork: Any, TSuccess: Any, TError: Any> From<WorkResult<TWork, TSuccess, TError>>
    for ErasedWorkStatus
{
    fn from(value: WorkResult<TWork, TSuccess, TError>) -> Self {
        match value {
            WorkResult::Continue(val) => ErasedWorkStatus::Continue(Box::new(val)),
            WorkResult::Skip(val) => ErasedWorkStatus::Skip(Box::new(val)),
            WorkResult::Success(val) => ErasedWorkStatus::Success(Box::new(val)),
            WorkResult::Error(val) => ErasedWorkStatus::Error(Box::new(val)),
        }
    }
}

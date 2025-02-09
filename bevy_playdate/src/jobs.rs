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
use derive_more::From;
use hashbrown::HashMap;

type JobId = usize;

#[derive(Resource)]
pub struct Jobs {
    /// Minimum number of jobs to run per frame
    pub min_jobs: usize,
    id_gen: JobId,
    unstarted: Vec<UnstartedJob>,
    jobs: BinaryHeap<RunningJob>,
    finished: HashMap<JobId, FinishedJob>,
}

type FinishedJob = Result<Box<dyn Any>, Box<dyn Any>>;

unsafe impl Send for Jobs {}
unsafe impl Sync for Jobs {}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            min_jobs: 5,
            id_gen: 0,
            unstarted: vec![],
            jobs: BinaryHeap::new(),
            finished: Default::default(),
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

        if let Some(job) = self.unstarted.iter().find(|j| j.id == job.id) {
            return Some(JobStatusRef::InProgress(job.work.downcast_ref().unwrap()));
        }

        if let Some(job) = self.jobs.iter().find(|j| j.id == job.id) {
            return Some(JobStatusRef::InProgress(job.work.downcast_ref().unwrap()));
        }

        None
    }

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

    fn understarted_jobs(&mut self) -> (&mut Vec<UnstartedJob>, &mut BinaryHeap<RunningJob>) {
        (&mut self.unstarted, &mut self.jobs)
    }

    /// System to run jobs
    pub fn run_jobs(world: &mut World, mut skip_buffer: Local<Vec<RunningJob>>) {
        world.resource_scope(|world, mut jobs: Mut<Jobs>| {
            let (unstarted, jbs) = jobs.understarted_jobs();
            for job in unstarted.drain(..) {
                let j = world.register_boxed_system(job.job);

                jbs.push(RunningJob {
                    priority: job.priority,
                    work: job.work,
                    id: job.id,
                    job: j,
                });
            }

            for _ in 0..jobs.min_jobs {
                let Some(mut job) = jobs.jobs.pop() else {
                    break;
                };

                match world.run_system_with(job.job, job.work).unwrap() {
                    ErasedWorkStatus::Continue(val) => {
                        job.work = val;
                        jobs.jobs.push(job);
                    }
                    ErasedWorkStatus::Skip(val) => {
                        job.work = val;
                        skip_buffer.deref_mut().push(job);
                    }
                    ErasedWorkStatus::Success(val) => {
                        jobs.finished.insert(job.id, Ok(val));
                        world.unregister_system(job.job).unwrap();
                    }
                    ErasedWorkStatus::Error(val) => {
                        jobs.finished.insert(job.id, Err(val));
                        world.unregister_system(job.job).unwrap();
                    }
                }
            }

            // Todo: do jobs while time still left

            for job in skip_buffer.deref_mut().drain(..) {
                jobs.jobs.push(job);
            }
        });
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

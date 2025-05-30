use crate::time::RunningTimer;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::collections::BinaryHeap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use bevy_app::{App, Last, Plugin};
use bevy_ecs::event::Event;
use bevy_ecs::prelude::{In, Local, Mut, Resource};
use bevy_ecs::system::{BoxedSystem, IntoSystem, System, SystemId};
use bevy_ecs::world::World;
use core::any::Any;
use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use derive_more::From;
use hashbrown::HashMap;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Jobs>()
            .init_resource::<JobsScheduler>()
            .init_resource::<FinishedJobs>();
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

    #[must_use]
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

    #[must_use]
    pub fn add_async<S: Any, E: Any>(
        &mut self,
        priority: isize,
        generator: Gen<(), (), impl Future<Output = Result<S, E>> + 'static + Send + Sync>,
    ) -> JobHandle<(), S, E> {
        self.add(priority, (), new_gen_job_simple(generator))
    }

    #[must_use]
    pub fn load_asset<A: AssetAsync>(
        &mut self,
        priority: isize,
        path: impl Into<Cow<'static, str>>,
    ) -> JobHandle<(), Arc<A>, A::Error> {
        let path = path.into();
        let job =
            async move |mut load_ctx: AsyncLoadCtx| load_ctx.load_asset::<A>(path.into()).await;

        self.add(priority, (), new_gen_job(Gen::new(job)))
    }
}

type JobId = usize;

#[derive(Resource)]
pub struct Jobs {
    /// Minimum number of jobs to run per frame
    pub min_jobs: usize,
    jobs: BinaryHeap<RunningJob>,
    to_cancel: Vec<RunningJob>,
}

type FinishedJob = Result<Box<dyn Any>, Box<dyn Any>>;

#[derive(Event, Copy, Clone, Eq, PartialEq)]
pub struct JobFinished {
    // pub handle: JobHandle<>
    pub job_id: JobId,
}

unsafe impl Send for Jobs {}
unsafe impl Sync for Jobs {}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            min_jobs: 5,
            jobs: BinaryHeap::new(),
            to_cancel: vec![],
        }
    }
}

#[derive(Resource, Default)]
pub struct FinishedJobs {
    finished: HashMap<JobId, FinishedJob>,
}

impl FinishedJobs {
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

unsafe impl Send for FinishedJobs {}
unsafe impl Sync for FinishedJobs {}

pub type Job<Work, Success, Error> = BoxedSystem<In<Work>, WorkResult<Work, Success, Error>>;

impl Jobs {
    // pub fn progress<Work: Any, Success: Any, Error: Any>(
    //     &self,
    //     job: &JobHandle<Work, Success, Error>,
    // ) -> Option<JobStatusRef<Work, Success, Error>> {
    //     if let Some(job) = self.finished.get(&job.id) {
    //         return Some(match job {
    //             Ok(val) => JobStatusRef::Success(val.downcast_ref().unwrap()),
    //             Err(val) => JobStatusRef::Error(val.downcast_ref().unwrap()),
    //         });
    //     }
    // 
    //     if let Some(job) = self.jobs.iter().find(|j| j.id == job.id) {
    //         return Some(JobStatusRef::InProgress(job.work.downcast_ref().unwrap()));
    //     }
    // 
    //     None
    // }

    // fn understarted_jobs(&mut self) -> (&mut Vec<UnstartedJob>, &mut BinaryHeap<RunningJob>) {
    //     (&mut self.unstarted, &mut self.jobs)
    // }

    /// System to run jobs
    pub fn run_jobs_system(world: &mut World, mut skip_buffer: Local<Vec<RunningJob>>) {
        world.resource_scope(|world, mut jobs: Mut<Jobs>| {
            for job in jobs.to_cancel.drain(..) {
                world
                    .unregister_system(job.job)
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
            // default is 50fps = 20ms = 0.02s, then let's give an extra 9ms of leeway
            const TARGET_HERTZ: f32 = 0.02 - 0.009;
            while world
                .resource::<RunningTimer>()
                .time_in_frame()
                .as_secs_f32()
                < TARGET_HERTZ
            {
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
                world.resource_mut::<FinishedJobs>().finished.insert(job.id, Ok(val));
                world.trigger(JobFinished { job_id: job.id });
                world
                    .unregister_system(job.job)
                    .expect("unregister completed job (success)");
            }
            ErasedWorkStatus::Error(val) => {
                world.resource_mut::<FinishedJobs>().finished.insert(job.id, Err(val));
                world.trigger(JobFinished { job_id: job.id });
                world
                    .unregister_system(job.job)
                    .expect("unregister completed job (error)");
            }
        }

        true
    }

    pub fn cancel<Work: Any, Success: Any, Error: Any>(
        &mut self,
        job: &JobHandle<Work, Success, Error>,
    ) {
        let mut v = core::mem::take(&mut self.jobs).into_vec();
        if let Some((i, _)) = v.iter().enumerate().find(|(_, j)| j.id == job.id) {
            let item = v.swap_remove(i);
            self.to_cancel.push(item);
        }
        self.jobs = BinaryHeap::from(v);
    }

    pub fn clear_all(&mut self) {
        for job in self.jobs.drain() {
            self.to_cancel.push(job);
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
    id: JobId,
    _phantom_data: PhantomData<(Work, Success, Error)>,
}

impl<Work, Success, Error> JobHandle<Work, Success, Error> {
    pub fn id(&self) -> JobId {
        self.id
    }
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
use crate::asset::{AssetAsync, ResAssetCache};
use crate::file::FileHandle;
use genawaiter::GeneratorState;
use genawaiter::sync::{Co, Gen};
use no_std_io2::io::{Error, Read};

fn new_gen_job_simple<S: Any, E: Any>(
    mut generator: Gen<(), (), impl Future<Output = Result<S, E>> + 'static + Send + Sync>,
) -> impl System<In = In<()>, Out = WorkResult<(), S, E>> {
    IntoSystem::into_system(move |In(()): In<()>| match generator.resume_with(()) {
        GeneratorState::Yielded(()) => WorkResult::Continue(()),
        GeneratorState::Complete(Ok(ok)) => WorkResult::Success(ok),
        GeneratorState::Complete(Err(err)) => WorkResult::Error(err),
    })
}

// pub(crate) enum JobRequest {
//     WithWorld(Box<dyn FnOnce(&mut World) -> Box<dyn Any + Send>>),
// }
//
// impl JobRequest {
//     pub fn fulfill(self, world: &mut World) -> JobResponse {
//         match self {
//             JobRequest::WithWorld(f) => JobResponse::WithWorld(f(world)),
//         }
//     }
// }
//
// pub(crate) enum JobResponse {
//     WithWorld(Box<dyn Any + Send>),
// }

pub type AsyncLoadCtx = Co<JobRequest, JobResponse>;

fn new_gen_job<S: Any, E: Any>(
    mut generator: Gen<
        JobRequest,
        JobResponse,
        impl Future<Output = Result<S, E>> + 'static + Send + Sync,
    >,
) -> impl System<In = In<()>, Out = WorkResult<(), S, E>> {
    IntoSystem::into_system(
        move |In(()): In<()>, world: &mut World, mut last_message: Local<Option<JobRequest>>| {
            // todo: should we yield again after this?
            let response = last_message
                .deref_mut()
                .take()
                .map(|j| match j {
                    JobRequest::Yield => JobResponse::None,
                    JobRequest::WithWorld(j) => JobResponse::WithWorld(j(world)),
                    JobRequest::Skip => JobResponse::None,
                })
                .unwrap_or_default();

            match generator.resume_with(response) {
                GeneratorState::Yielded(request) => {
                    *last_message = Some(request);

                    WorkResult::Continue(())
                }
                GeneratorState::Complete(Ok(ok)) => WorkResult::Success(ok),
                GeneratorState::Complete(Err(err)) => WorkResult::Error(err),
            }
        },
    )
}

pub trait GenJobExtensions {
    #[allow(async_fn_in_trait)]
    async fn with_world<T, F>(&mut self, f: F) -> T
    where
        T: Any + Send + 'static,
        F: FnOnce(&mut World) -> T + Send + 'static;

    #[allow(async_fn_in_trait)]
    async fn yield_next(&mut self);

    #[allow(async_fn_in_trait)]
    async fn load_asset<A: AssetAsync>(&mut self, path: Arc<str>) -> Result<Arc<A>, A::Error>;
}

pub enum JobRequest {
    Yield,
    Skip,
    WithWorld(WithWorldFn),
}

pub type WithWorldFn = Box<dyn FnOnce(&mut World) -> Box<dyn Any + Send> + Send>;

#[derive(Default)]
pub enum JobResponse {
    #[default]
    None,
    WithWorld(Box<dyn Any + Send>),
}

impl GenJobExtensions for Co<JobRequest, JobResponse> {
    async fn with_world<T, F>(&mut self, f: F) -> T
    where
        T: Any + Send + 'static,
        F: (FnOnce(&mut World) -> T) + Send + 'static,
    {
        let request = JobRequest::WithWorld(Box::new(move |world: &mut World| {
            let result = f(world);
            Box::new(result)
        }));

        let response = self.yield_(request).await;
        let JobResponse::WithWorld(response) = response else {
            panic!("mismatched job response");
        };

        *response
            .downcast::<T>()
            .expect("received response should be ")
    }

    async fn yield_next(&mut self) {
        self.yield_(JobRequest::Yield).await;
    }

    async fn load_asset<A: AssetAsync>(&mut self, path: Arc<str>) -> Result<Arc<A>, A::Error> {
        // check to see if it's already loaded
        let asset = self
            .with_world({
                let path = path.clone();
                move |world| {
                    let cache = world.resource::<ResAssetCache>();
                    cache.0.read().unwrap().get::<A>(&path)
                }
            })
            .await;

        if let Some(asset) = asset {
            return Ok(asset);
        }
        // not already load it, let's load it
        let r = A::load(self, path.deref()).await?;

        let rc = self
            .with_world(move |world: &mut World| {
                let cache = world.resource_mut::<ResAssetCache>();
                cache.0.try_write().unwrap().insert(path.to_string(), r)
            })
            .await;

        Ok(rc)
    }
}

pub async fn load_file_bytes(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Vec<u8>, Error> {
    let mut file = FileHandle::read_only(path)?;
    let mut bytes = Vec::with_capacity(128);
    let mut file_length = 0;

    // use playdate::system::System as PDSys;
    // let cur = PDSys::Default().elapsed_time();

    loop {
        // read in next bytes
        // Ensure there's spare capacity
        if bytes.len() <= file_length {
            bytes.reserve(1);
        }

        // Get the uninit spare capacity
        let spare = bytes.spare_capacity_mut();
        if spare.is_empty() {
            continue;
        }

        let buf = spare.write_filled(0);

        // println!()
        let n = file.read(buf)?;

        if n == 0 {
            break; // EOF
        }

        // SAFETY: `n` bytes were just initialized by `read`.
        unsafe {
            let new_len = bytes.len() + n;
            bytes.set_len(new_len);
        }
        file_length += n;
        // wait for next load opportunity
        load_cx.yield_next().await;
    }

    // let after = PDSys::Default().elapsed_time();
    // dbg!(after - cur);

    bytes.truncate(file_length);
    Ok(bytes)
}

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::collections::BinaryHeap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use bevy_ecs::prelude::{In, Local, Mut, Resource};
use bevy_ecs::system::{BoxedSystem, IntoSystem, System, SystemId};
use bevy_ecs::world::World;
use core::any::Any;
use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
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
        generator: Gen<(), (), impl Future<Output=Result<S, E>> + 'static + Send + Sync>
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
        let job = async move |mut load_ctx: AsyncLoadCtx| {
            load_ctx.load_asset::<A>(path.into()).await
        };
        
        self.add(priority, (), new_gen_job(Gen::new(job)))
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
            // default is 50fps = 20ms = 0.02s, then let's give an extra 9ms of leeway
            const TARGET_HERTZ: f32 = 0.02 - 0.009;
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
    
    pub fn clear_all(&mut self) {
        for job in self.jobs.drain() {
            self.to_cancel.push(job);
        }
        self.finished.clear();
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
use genawaiter::GeneratorState;
use genawaiter::sync::{Co, Gen};
use no_std_io2::io::{Error, Read};
use playdate::println;
use diagnostic::dbg;
use crate::asset::{AssetAsync, ResAssetCache};
use crate::file::FileHandle;

fn new_gen_job_simple<S: Any, E: Any>(mut generator: Gen<(), (), impl Future<Output=Result<S, E>> + 'static + Send + Sync>) -> impl System<In = In<()>, Out = WorkResult<(), S, E>> {
    IntoSystem::into_system(move |In(()): In<()>| {
        match generator.resume_with(()) {
            GeneratorState::Yielded(()) => WorkResult::Continue(()),
            GeneratorState::Complete(Ok(ok)) => WorkResult::Success(ok),
            GeneratorState::Complete(Err(err)) => WorkResult::Error(err),
        }
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
    mut generator: Gen<JobRequest, JobResponse, impl Future<Output=Result<S, E>> + 'static + Send + Sync>,
) -> impl System<In = In<()>, Out = WorkResult<(), S, E>> {
    IntoSystem::into_system(move |
        In(()): In<()>,
        world: &mut World,
        mut last_message: Local<Option<JobRequest>>,
    | {
        // todo: should we yield again after this?
        let response = last_message.deref_mut().take()
            .map(|j| match j {
                JobRequest::Yield => JobResponse::None,
                JobRequest::WithWorld(j) => JobResponse::WithWorld(j(world)),
            })
            .unwrap_or_default();
        
        match generator.resume_with(response) {
            GeneratorState::Yielded(request) => {
                *last_message = Some(request);
                WorkResult::Continue(())
            },
            GeneratorState::Complete(Ok(ok)) => WorkResult::Success(ok),
            GeneratorState::Complete(Err(err)) => WorkResult::Error(err),
        }
    })
}

pub trait GenJobExtensions {
    #[allow(async_fn_in_trait)]
    async fn with_world<T, F>(&mut self, f: F) -> T
    where
        T: Any + Send + 'static,
        F: FnOnce(&mut World) -> T + Send + 'static;
    
    #[allow(async_fn_in_trait)]
    async fn yield_next(&mut self);
    
    async fn load_asset<A: AssetAsync>(&mut self, path: Arc<str>) -> Result<Arc<A>, A::Error>;
}

pub enum JobRequest {
    Yield,
    WithWorld(Box<dyn FnOnce(&mut World) -> Box<dyn Any + Send> + Send>)
}

#[derive(Default)]
pub enum JobResponse {
    #[default]
    None,
    WithWorld(Box<dyn Any + Send>)
}

impl GenJobExtensions for Co<JobRequest, JobResponse> {
    async fn with_world<T, F>(&mut self, f: F) -> T
    where
        T: Any + Send + 'static,
        F: (FnOnce(&mut World) -> T) + Send + 'static
    {
        let request = JobRequest::WithWorld(Box::new(move |world: &mut World| {
            let result = f(world);
            Box::new(result)
        }));

        let response = self.yield_(request).await;
        let JobResponse::WithWorld(response) = response else {
            panic!("mismatched job response");
        };
        
        *response.downcast::<T>().ok().expect("received response should be ")
    }

    async fn yield_next(&mut self) {
        self.yield_(JobRequest::Yield).await;
    }

    async fn load_asset<A: AssetAsync>(&mut self, path: Arc<str>) -> Result<Arc<A>, A::Error> {
        // check to see if it's already loaded
        let asset = self.with_world({
            let path = path.clone();
            move |world| {
                let cache = world.resource::<ResAssetCache>();
                cache.0.read().unwrap().get::<A>(&*path)
            }
        }).await;
        
        if let Some(asset) = asset {
            return Ok(asset);
        }
        // not already load it, let's load it
        let r = A::load(self, path.deref()).await?;
        
        let rc = self.with_world(move |world: &mut World| {
            let cache = world.resource_mut::<ResAssetCache>();
            cache.0.try_write().unwrap().insert(path.to_string(), r)
        }).await;
        
        Ok(rc)
    }
}

pub async fn load_file_bytes(load_cx: &mut AsyncLoadCtx, path: &str) -> Result<Vec<u8>, Error> {
    let mut file = FileHandle::read_only(&path)?;
    let mut bytes = Vec::with_capacity(800);
    // let mut file_length = 0;
    file.read_to_end(&mut bytes)?;
    
    // println!("here");
    // println!("here");
    // println!("here");

    // loop {
    //     // read in next bytes
    //     // Ensure there's spare capacity
    //     if bytes.len() <= file_length {
    //         bytes.
    //     }
    // 
    //     // Get the uninit spare capacity
    //     let spare = bytes.spare_capacity_mut();
    //     if spare.is_empty() {
    //         continue;
    //     }
    // 
    //     // SAFETY: We're only writing to the uninitialized part, and `set_len` will
    //     // only advance by the number of bytes actually written.
    //     let buf = unsafe {
    //         core::slice::from_raw_parts_mut(spare.as_mut_ptr() as *mut u8, spare.len())
    //     };
    // 
    //     // println!()
    //     let n = file.read(buf)?;
    //     println!("N: {}", n);
    //     println!("N: {}", n);
    //     println!("N: {}", n);
    // 
    //     if n == 0 {
    //         break; // EOF
    //     }
    // 
    //     // SAFETY: `n` bytes were just initialized by `read`.
    //     unsafe {
    //         let new_len = bytes.len() + n;
    //         bytes.set_len(new_len);
    //     }
    //     // wait for next load opportunity
    //     load_cx.yield_next().await;
    // }
    
    // println!("[");
    // for &byte in bytes.iter() {
    //     println!("{:2X?}", byte);
    // }
    // println!("]");
    // println!("{:2X?}", bytes);
    // 
    println!("{:20X}", bytes.as_ptr() as *const _ as usize);
    println!("{:20X}", bytes.as_ptr() as *const _ as usize);
    println!("{:20X}", bytes.as_ptr() as *const _ as usize);
    dbg!(bytes[0]);
    dbg!(bytes[0]);
    dbg!(bytes[0]);
    Ok(bytes)
}


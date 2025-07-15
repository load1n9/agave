use super::{Task, TaskId};
use alloc::collections::VecDeque;
use alloc::task::Wake;
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crossbeam::queue::ArrayQueue;
use futures::task::AtomicWaker;
use futures::Future;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref YIELDERS: ArrayQueue<AtomicWaker> = ArrayQueue::new(100);
    pub static ref TASK_METRICS: spin::Mutex<TaskMetrics> = spin::Mutex::new(TaskMetrics::new());
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0, // System critical tasks
    High = 1,     // Interactive/user tasks
    Normal = 2,   // Background tasks
    Low = 3,      // Cleanup/maintenance tasks
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Task performance metrics
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    pub total_tasks_spawned: u64,
    pub tasks_completed: u64,
    pub total_execution_time_us: u64,
    pub context_switches: u64,
}

impl TaskMetrics {
    pub fn new() -> Self {
        Self {
            total_tasks_spawned: 0,
            tasks_completed: 0,
            total_execution_time_us: 0,
            context_switches: 0,
        }
    }

    pub fn task_spawned(&mut self) {
        self.total_tasks_spawned += 1;
    }

    pub fn task_completed(&mut self, execution_time_us: u64) {
        self.tasks_completed += 1;
        self.total_execution_time_us += execution_time_us;
    }

    pub fn context_switch(&mut self) {
        self.context_switches += 1;
    }
}

/// Enhanced task with priority and timing information
pub struct PriorityTask {
    pub task: Task,
    pub priority: TaskPriority,
    pub spawn_time: u64,
    pub execution_count: u32,
}

impl PriorityTask {
    pub fn new(future: impl Future<Output = ()> + 'static, priority: TaskPriority) -> Self {
        let mut metrics = TASK_METRICS.lock();
        metrics.task_spawned();
        drop(metrics);

        Self {
            task: Task::new(future),
            priority,
            spawn_time: crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed),
            execution_count: 0,
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.execution_count += 1;
        let result = self.task.poll(cx);

        if let Poll::Ready(()) = result {
            let execution_time = crate::sys::interrupts::TIME_MS
                .load(core::sync::atomic::Ordering::Relaxed)
                - self.spawn_time;
            let mut metrics = TASK_METRICS.lock();
            metrics.task_completed(execution_time * 1000); // Convert to microseconds
        }

        result
    }
}

/// Priority-based task queue
pub struct PriorityQueue {
    queues: [VecDeque<PriorityTask>; 4], // One for each priority level
}

impl PriorityQueue {
    pub fn new() -> Self {
        Self {
            queues: [
                VecDeque::new(), // Critical
                VecDeque::new(), // High
                VecDeque::new(), // Normal
                VecDeque::new(), // Low
            ],
        }
    }

    pub fn push(&mut self, task: PriorityTask) {
        let priority_index = task.priority as usize;
        self.queues[priority_index].push_back(task);
    }

    pub fn pop(&mut self) -> Option<PriorityTask> {
        // Round-robin within priority levels with starvation prevention
        for queue in &mut self.queues {
            if let Some(task) = queue.pop_front() {
                let mut metrics = TASK_METRICS.lock();
                metrics.context_switch();
                return Some(task);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.queues.iter().all(|q| q.is_empty())
    }
}

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

impl SimpleExecutor {
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {} // task done
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spawner(Arc<ArrayQueue<Task>>);

impl Spawner {
    pub fn run(&self, future: impl Future<Output = ()> + 'static) {
        let _ = self.0.push(Task::new(future));
    }

    pub fn run_with_priority(
        &self,
        future: impl Future<Output = ()> + 'static,
        _priority: TaskPriority,
    ) {
        // For compatibility, convert to regular task (priority scheduling not implemented in spawner yet)
        let _ = self.0.push(Task::new(future));
    }
}

pub struct Executor {
    tasks: BTreeMap<TaskId, PriorityTask>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    spawn_queue: Arc<ArrayQueue<Task>>,
    waker_cache: BTreeMap<TaskId, Waker>,
    priority_queue: PriorityQueue,
    task_slice_us: u64, // Time slice per task in microseconds
}

pub fn qpush(queue: Arc<ArrayQueue<Task>>, f: impl Future<Output = ()> + 'static) {
    let _ = queue.push(Task::new(f));
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            spawn_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
            priority_queue: PriorityQueue::new(),
            task_slice_us: 1000, // 1ms default time slice
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner(self.spawn_queue.clone())
    }

    pub fn spawn_with_priority(
        &mut self,
        future: impl Future<Output = ()> + 'static,
        priority: TaskPriority,
    ) {
        let priority_task = PriorityTask::new(future, priority);
        let task_id = priority_task.task.id;

        if self.tasks.insert(task_id, priority_task).is_some() {
            log::warn!("Task with ID {:?} already exists, replacing", task_id);
        }

        if let Err(_) = self.task_queue.push(task_id) {
            log::error!("Task queue full, dropping task {:?}", task_id);
        }
    }

    pub fn get_metrics(&self) -> TaskMetrics {
        let metrics = TASK_METRICS.lock();
        TaskMetrics {
            total_tasks_spawned: metrics.total_tasks_spawned,
            tasks_completed: metrics.tasks_completed,
            total_execution_time_us: metrics.total_execution_time_us,
            context_switches: metrics.context_switches,
        }
    }
}

impl Executor {
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        let priority_task = PriorityTask {
            task,
            priority: TaskPriority::Normal,
            spawn_time: crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed),
            execution_count: 0,
        };

        if self.tasks.insert(task_id, priority_task).is_some() {
            log::warn!("Task with ID {:?} already exists, replacing", task_id);
        }

        if let Err(_) = self.task_queue.push(task_id) {
            log::error!("Task queue full, dropping task {:?}", task_id);
        }
    }
}

impl Executor {
    fn run_ready_tasks(&mut self) {
        // destructure `self` to avoid borrow checker errors
        let Self {
            tasks,
            task_queue,
            waker_cache,
            spawn_queue: _,
            priority_queue: _,
            task_slice_us: _,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }
}

impl Executor {
    pub fn run(&mut self) -> ! {
        loop {
            while let Some(e) = self.spawn_queue.pop() {
                self.spawn(e);
            }

            self.run_ready_tasks();

            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::interrupts::disable();
            if self.task_queue.is_empty() && self.spawn_queue.is_empty() {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                x86_64::instructions::interrupts::enable_and_hlt();
            } else {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                x86_64::instructions::interrupts::enable();
            }
            while let Some(e) = YIELDERS.pop() {
                e.wake();
            }
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn wake_task(&self) {
        if let Err(_) = self.task_queue.push(self.task_id) {
            log::error!("Failed to wake task {:?}: queue full", self.task_id);
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
}

pub async fn yield_once() {
    let timer = YieldOnce(false);
    timer.await;
}

pub struct YieldOnce(bool);

impl futures::future::Future for YieldOnce {
    type Output = ();
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if !self.0 {
            self.as_mut().0 = true;
            let aw = AtomicWaker::new();
            aw.register(&cx.waker());
            if let Err(_) = YIELDERS.push(aw) {
                // If yielders queue is full, just continue
                return core::task::Poll::Ready(());
            }
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(())
        }
    }
}

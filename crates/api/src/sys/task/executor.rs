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
}

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    spawn_queue: Arc<ArrayQueue<Task>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

pub fn qpush(queue: Arc<ArrayQueue<Task>>, f: impl Future<Output = ()> + 'static) {
    let _ = queue.push(Task::new(f));
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            spawn_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner(self.spawn_queue.clone())
    }
}

impl Executor {
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
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
        self.task_queue.push(self.task_id).expect("task_queue full");
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
            let _ = YIELDERS.push(aw);
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(())
        }
    }
}

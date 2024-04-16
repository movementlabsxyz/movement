use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
    collections::VecDeque,  // VecDeque is typically more efficient for task queues
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use futures::task::{waker_ref, ArcWake};
use std::cell::Cell;

pub struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl Task {
    fn poll(&self, waker: &Waker) -> Poll<()> {
        let mut future = self.future.lock().unwrap();
        let mut context = Context::from_waker(&waker);
        future.as_mut().poll(&mut context)
    }
}

thread_local! {
    static DEPTH: Cell<u8> = Cell::new(0);
}


impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Enqueue the task for polling instead of polling directly here
        let task = arc_self.clone();

        DEPTH.with(|depth| {
           
            depth.set(depth.get() + 1);
            println!("Depth: {}", depth.get());
            
            let waker = waker_ref(arc_self).clone();
            waker.wake_by_ref();  // This recurses back to `wake_by_ref`, be cautious
            
            depth.set(depth.get() - 1);
           
        });

        EXECUTOR.with(|executor| {
            let mut exec = executor.lock().unwrap();
            exec.tasks.push_back(task);
        });
    }
}

pub struct Executor {
    tasks: VecDeque<Arc<Task>>,
}

impl Executor {
    pub fn new() -> Self {
        Executor { tasks: VecDeque::new() }
    }

    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
        });
        self.tasks.push_back(task);
    }

    pub fn run(&mut self) {
        while let Some(task) = self.tasks.pop_front() {
            let waker = futures::task::waker_ref(&task);
            if let Poll::Pending = task.poll(&*waker) {
                // If task is pending, it should have re-enqueued itself via wake_by_ref
                self.tasks.push_back(task);
            }
        }
    }
}

thread_local! {
    static EXECUTOR: Mutex<Executor> = Mutex::new(Executor::new());
}

pub struct Timer {
    when: Instant,
}

impl Timer {
    pub fn after(duration: Duration) -> Self {
        Self { when: Instant::now() + duration }
    }

    pub fn new(when: Instant) -> Self {
        Self { when }
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.when {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
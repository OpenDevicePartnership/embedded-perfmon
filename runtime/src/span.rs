use core::{
    sync::atomic::{AtomicU32, Ordering},
    task::Poll,
};

use embedded_perfmon_transport::{Event, EventKind, GlobalEvent, TaskEvent, TaskEventKind};

use crate::{get_trace_event_timestamp, write_trace_event};

pub fn start_global_span(name: &'static str) -> GlobalSpan {
    static ID: AtomicU32 = AtomicU32::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    write_trace_event(Event {
        timestamp: get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::SpanStart { name, id }),
    });

    GlobalSpan { id }
}

pub struct GlobalSpan {
    id: u32,
}

impl Drop for GlobalSpan {
    fn drop(&mut self) {
        write_trace_event(Event {
            timestamp: get_trace_event_timestamp(),
            kind: EventKind::Global(GlobalEvent::SpanEnd { id: self.id }),
        });
    }
}

pub async fn start_task_span(name: &'static str) -> TaskSpan {
    static ID: AtomicU32 = AtomicU32::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    let task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    write_trace_event(Event {
        timestamp: get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: task_ref.id().get() as u32,
            kind: TaskEventKind::SpanStart { name, id },
        }),
    });

    TaskSpan {
        id,
        task_id: task_ref.id().get() as u32,
    }
}

pub struct TaskSpan {
    id: u32,
    task_id: u32,
}

impl Drop for TaskSpan {
    fn drop(&mut self) {
        write_trace_event(Event {
            timestamp: get_trace_event_timestamp(),
            kind: EventKind::Task(TaskEvent {
                task_id: self.task_id,
                kind: TaskEventKind::SpanEnd { id: self.id },
            }),
        });
    }
}

#[allow(async_fn_in_trait)]
pub trait SpanFutureExt {
    type Output;
    async fn with_global_span(self, name: &'static str) -> Self::Output;
    async fn with_task_span(self, name: &'static str) -> Self::Output;
}

impl<F: Future> SpanFutureExt for F {
    type Output = F::Output;

    async fn with_global_span(self, name: &'static str) -> Self::Output {
        let token = start_global_span(name);
        let value = self.await;
        drop(token);
        value
    }

    async fn with_task_span(self, name: &'static str) -> Self::Output {
        let token = start_task_span(name).await;
        let value = self.await;
        drop(token);
        value
    }
}

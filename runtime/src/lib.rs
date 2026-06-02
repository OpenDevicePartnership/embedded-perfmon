#![no_std]

use core::{
    sync::atomic::{AtomicU32, Ordering},
    task::Poll,
};

use embassy_executor::Spawner;
use embedded_perfmon_transport::{
    Event, EventKind, ExecutorEvent, ExecutorEventKind, GlobalEvent, TaskEvent, TaskEventKind,
};

pub use embedded_perfmon_transport as transport;

/// Register the main task so it's properly known
pub async fn register_main(spawner: &Spawner) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::TickRate {
            rate: _get_trace_event_tickrate(),
        }),
    });

    let main_task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: main_task_ref.id(),
            kind: TaskEventKind::TaskNew {
                executor_id: spawner.executor_id() as u32,
            },
        }),
    });

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: main_task_ref.id(),
            kind: TaskEventKind::TaskNamed { name: "main" },
        }),
    });
}

pub fn emit_global_marker(name: &str) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::Marker { name }),
    });
}

pub async fn emit_task_marker(name: &str) {
    let task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: task_ref.id(),
            kind: TaskEventKind::Marker { name },
        }),
    });
}

pub fn start_global_span(name: &str) -> GlobalSpan {
    static ID: AtomicU32 = AtomicU32::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::SpanStart { name, id }),
    });

    GlobalSpan { id }
}

pub struct GlobalSpan {
    id: u32,
}

impl Drop for GlobalSpan {
    fn drop(&mut self) {
        _write_trace_event(Event {
            timestamp: _get_trace_event_timestamp(),
            kind: EventKind::Global(GlobalEvent::SpanEnd { id: self.id }),
        });
    }
}

pub async fn start_task_span(name: &str) -> TaskSpan {
    static ID: AtomicU32 = AtomicU32::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    let task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: task_ref.id(),
            kind: TaskEventKind::SpanStart { name, id },
        }),
    });

    TaskSpan {
        id,
        task_id: task_ref.id(),
    }
}

pub struct TaskSpan {
    id: u32,
    task_id: u32,
}

impl Drop for TaskSpan {
    fn drop(&mut self) {
        _write_trace_event(Event {
            timestamp: _get_trace_event_timestamp(),
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
    async fn with_global_span(self, name: &str) -> Self::Output;
    async fn with_task_span(self, name: &str) -> Self::Output;
}

impl<F: Future> SpanFutureExt for F {
    type Output = F::Output;

    async fn with_global_span(self, name: &str) -> Self::Output {
        let token = start_global_span(name);
        let value = self.await;
        drop(token);
        value
    }

    async fn with_task_span(self, name: &str) -> Self::Output {
        let token = start_task_span(name).await;
        let value = self.await;
        drop(token);
        value
    }
}

/// This callback is called when the executor begins polling. This will always
/// be paired with a later call to `_embassy_trace_executor_idle`.
///
/// This marks the EXECUTOR state transition from IDLE -> SCHEDULING.
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_poll_start(executor_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::ExecutorPollStart,
        }),
    });
}

/// This callback is called AFTER a task is initialized/allocated, and BEFORE
/// it is enqueued to run for the first time. If the task ends (and does not
/// loop "forever"), there will be a matching call to `_embassy_trace_task_end`.
///
/// Tasks start life in the SPAWNED state.
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_new(executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id,
            kind: TaskEventKind::TaskNew { executor_id },
        }),
    });
}

/// This callback is called AFTER a task is destructed/freed. This will always
/// have a prior matching call to `_embassy_trace_task_new`.
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_end(_executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id,
            kind: TaskEventKind::TaskEnd,
        }),
    });
}

/// This callback is called AFTER a task has been dequeued from the runqueue,
/// and BEFORE the task is polled. There will always be a matching call to
/// `_embassy_trace_task_exec_end`.
///
/// This marks the TASK state transition from WAITING -> RUNNING
/// This marks the EXECUTOR state transition from SCHEDULING -> POLLING
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_exec_begin(_executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id,
            kind: TaskEventKind::TaskExecBegin,
        }),
    });
}

/// This callback is called AFTER a task has completed polling. There will
/// always be a matching call to `_embassy_trace_task_exec_begin`.
///
/// This marks the TASK state transition from either:
/// * RUNNING -> IDLE - if there were no `_embassy_trace_task_ready_begin` events
///   for this task since the last `_embassy_trace_task_exec_begin` for THIS task
/// * RUNNING -> WAITING - if there WAS a `_embassy_trace_task_ready_begin` event
///   for this task since the last `_embassy_trace_task_exec_begin` for THIS task
///
/// This marks the EXECUTOR state transition from POLLING -> SCHEDULING
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_exec_end(_executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id,
            kind: TaskEventKind::TaskExecEnd,
        }),
    });
}

/// This callback is called AFTER the waker for a task is awoken, and BEFORE it
/// is added to the run queue.
///
/// If the given task is currently RUNNING, this marks no state change, BUT the
/// RUNNING task will then move to the WAITING stage when polling is complete.
///
/// If the given task is currently IDLE, this marks the TASK state transition
/// from IDLE -> WAITING.
///
/// NOTE: This may be called from an interrupt, outside the context of the current
/// task or executor.
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_ready_begin(_executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id,
            kind: TaskEventKind::TaskReadyBegin,
        }),
    });
}

/// This callback is called AFTER all dequeued tasks in a single call to poll
/// have been processed. This will always be paired with a call to
/// `_embassy_trace_executor_idle`.
///
/// This marks the EXECUTOR state transition from SCHEDULING -> IDLE
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_executor_idle(executor_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::ExecutorIdle,
        }),
    });
}

#[unsafe(no_mangle)]
unsafe fn _embassy_mcxa_trace_irq_start(irq: u16) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::IrqStart { irq }),
    });
}

#[unsafe(no_mangle)]
unsafe fn _embassy_mcxa_trace_irq_end(irq: u16) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::IrqEnd { irq }),
    });
}

unsafe extern "Rust" {
    /// Gets called for every event. The implementation should call [`Event::serialize`] to turn the event into bytes.
    /// The bytes of multiple events form a byte stream that doesn't need additional framing. This byte stream can later
    /// be consumed by the analyzer directly.
    /// 
    /// The stream may have gaps (at the cost of having incomplete trace data), but must be in order.
    safe fn _write_trace_event(event: Event<'_>);
    /// Get the current time in ticks
    safe fn _get_trace_event_timestamp() -> u64;
    /// Get the amount of ticks per second that the timestamp uses
    safe fn _get_trace_event_tickrate() -> u64;
}

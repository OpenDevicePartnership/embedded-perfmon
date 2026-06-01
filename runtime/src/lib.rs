#![no_std]

use embedded_perfmon_transport::{Event, EventKind, ExecutorEvent, ExecutorEventKind, GlobalEvent};

pub fn emit_tickrate_trace() {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::TickRate {
            rate: _get_trace_event_tickrate(),
        }),
    });
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
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::TaskNew { task_id },
        }),
    });
}

/// This callback is called AFTER a task is destructed/freed. This will always
/// have a prior matching call to `_embassy_trace_task_new`.
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_end(executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::TaskEnd { task_id },
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
unsafe fn _embassy_trace_task_exec_begin(executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::TaskExecBegin { task_id },
        }),
    });
}

/// This callback is called AFTER a task has completed polling. There will
/// always be a matching call to `_embassy_trace_task_exec_begin`.
///
/// This marks the TASK state transition from either:
/// * RUNNING -> IDLE - if there were no `_embassy_trace_task_ready_begin` events
///     for this task since the last `_embassy_trace_task_exec_begin` for THIS task
/// * RUNNING -> WAITING - if there WAS a `_embassy_trace_task_ready_begin` event
///     for this task since the last `_embassy_trace_task_exec_begin` for THIS task
///
/// This marks the EXECUTOR state transition from POLLING -> SCHEDULING
#[unsafe(no_mangle)]
unsafe fn _embassy_trace_task_exec_end(executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::TaskExecEnd { task_id },
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
unsafe fn _embassy_trace_task_ready_begin(executor_id: u32, task_id: u32) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Executor(ExecutorEvent {
            executor_id,
            kind: ExecutorEventKind::TaskReadyBegin { task_id },
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
    safe fn _write_trace_event(event: Event<'_>);
    safe fn _get_trace_event_timestamp() -> u64;
    safe fn _get_trace_event_tickrate() -> u64;
}

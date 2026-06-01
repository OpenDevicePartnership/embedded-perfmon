use std::{fs, path::PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use embedded_perfmon_transport::{Event, EventKind, ExecutorEvent, ExecutorEventKind, GlobalEvent};
use indexmap::IndexMap;
use serde::Serialize;

/// Analyzer of trace data
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    source: Source,
    /// The path where the output json is saved.
    /// If not specified, the json is outputted to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// Doc comment
#[derive(Subcommand, Debug, Clone)]
enum Source {
    /// Decode from a file
    File {
        /// The path to the file to decode
        #[arg(short, long)]
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut bytes = match args.source {
        Source::File { path } => collect_from_file(path)?,
    };

    let events = deserialize_events(&mut bytes)?;

    let traces = Capture::parse_traces(&events);

    if let Some(output_path) = args.output {
        let mut file = fs::File::create(&output_path).context(format!(
            "creating output path at: {}",
            output_path.display()
        ))?;
        serde_json::to_writer_pretty(&mut file, &traces)
            .context("serializing traces to json and writing to file")?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&traces).context("serializing traces to json")?
        );
    }

    Ok(())
}

fn collect_from_file(path: PathBuf) -> anyhow::Result<Vec<u8>> {
    fs::read(path).context("reading input file")
}

fn deserialize_events(mut bytes: &mut [u8]) -> anyhow::Result<Vec<Event<'_>>> {
    let mut events = Vec::new();

    while !bytes.is_empty() {
        let bytes_len = bytes.len();
        let (event, rest) = match Event::deserialize(bytes).context("deserializing event") {
            Ok(v) => v,
            Err(e) => {
                if bytes_len < 128 {
                    // We're near the end. Probably just an early termination of the byte stream, so just ignore
                    return Ok(events);
                } else {
                    eprintln!("{} events deserialized before error: {events:?}", events.len());
                    return Err(e);
                }
            }
        };
        bytes = rest;
        events.push(event);
    }

    Ok(events)
}

#[derive(Serialize)]
pub struct Capture {
    pub tickrate: u64,
    pub irq_traces: IndexMap<u16, Vec<TimedState<bool>>>,
    pub executor_traces: IndexMap<u32, Trace>,
}

impl Capture {
    fn parse_traces(events: &[Event<'_>]) -> Self {
        let mut tickrate = 0;
        let mut irq_traces: IndexMap<u16, Vec<TimedState<bool>>> = IndexMap::new();
        let mut executor_traces: IndexMap<u32, Trace> = IndexMap::new();

        for event in events {
            match &event.kind {
                EventKind::Global(global_event) => match global_event {
                    GlobalEvent::TickRate { rate } => tickrate = *rate,
                    GlobalEvent::IrqStart { irq } => {
                        irq_traces.entry(*irq).or_default().push(TimedState {
                            timestamp: event.timestamp,
                            state: true,
                        });
                    }
                    GlobalEvent::IrqEnd { irq } => {
                        irq_traces.entry(*irq).or_default().push(TimedState {
                            timestamp: event.timestamp,
                            state: false,
                        });
                    }
                },
                EventKind::Executor(executor_event) => {
                    executor_traces
                        .entry(executor_event.executor_id)
                        .or_default()
                        .handle(executor_event, event.timestamp);
                }
            }
        }

        Self {
            tickrate,
            irq_traces,
            executor_traces,
        }
    }
}

#[derive(Serialize)]
pub enum TaskState {
    Spawned,
    Waiting,
    Running,
    /// Currently running, but also waiting to be polled again already
    RunningWaiting,
    Idle,
    End,
}

#[derive(Serialize)]
pub enum ExecutorState {
    Idle,
    Scheduling,
    Polling,
}

#[derive(Serialize)]
pub struct TimedState<T> {
    pub timestamp: u64,
    pub state: T,
}

#[derive(Default, Serialize)]
pub struct Trace {
    pub executor: Vec<TimedState<ExecutorState>>,
    pub tasks: IndexMap<u32, Vec<TimedState<TaskState>>>,
}

impl Trace {
    fn handle(&mut self, event: &ExecutorEvent<'_>, timestamp: u64) {
        if self.executor.is_empty() {
            // Executor always starts idle
            self.executor.push(TimedState {
                timestamp,
                state: ExecutorState::Idle,
            });
        }

        match event.kind {
            ExecutorEventKind::ExecutorPollStart => {
                self.executor.push(TimedState {
                    timestamp,
                    state: ExecutorState::Scheduling,
                });
            }
            ExecutorEventKind::ExecutorIdle => {
                self.executor.push(TimedState {
                    timestamp,
                    state: ExecutorState::Idle,
                });
            }
            ExecutorEventKind::TaskNew { task_id } => {
                let task_trace = self.tasks.entry(task_id).or_default();

                task_trace.push(TimedState {
                    timestamp,
                    state: TaskState::Spawned,
                });
            }
            ExecutorEventKind::TaskEnd { task_id } => {
                let task_trace = self.tasks.entry(task_id).or_default();

                if task_trace.is_empty() {
                    eprintln!("Detected TaskEnd for non-existing task: {task_id}");
                }

                if !matches!(
                    task_trace.last(),
                    Some(TimedState {
                        state: TaskState::Running,
                        ..
                    })
                ) {
                    eprintln!(
                        "Detected TaskEnd for task that's not in the running state: {task_id}"
                    );
                }

                task_trace.push(TimedState {
                    timestamp,
                    state: TaskState::End,
                });
            }
            ExecutorEventKind::TaskExecBegin { task_id } => {
                let task_trace = self.tasks.entry(task_id).or_default();

                if task_trace.is_empty() {
                    eprintln!("Detected TaskExecBegin for non-existing task: {task_id}");
                }

                if !matches!(
                    task_trace.last(),
                    Some(TimedState {
                        state: TaskState::Waiting,
                        ..
                    })
                ) {
                    eprintln!(
                        "Detected TaskExecBegin for task that's not in the waiting state: {task_id}"
                    );
                }

                task_trace.push(TimedState {
                    timestamp,
                    state: TaskState::Running,
                });

                self.executor.push(TimedState {
                    timestamp,
                    state: ExecutorState::Polling,
                });
            }
            ExecutorEventKind::TaskExecEnd { task_id } => {
                let task_trace = self.tasks.entry(task_id).or_default();

                if task_trace.is_empty() {
                    eprintln!("Detected TaskExecEnd for non-existing task: {task_id}");
                }

                if !matches!(
                    task_trace.last(),
                    Some(TimedState {
                        state: TaskState::Running,
                        ..
                    })
                ) {
                    eprintln!(
                        "Detected TaskExecEnd for task that's not in the running state: {task_id}"
                    );
                }

                task_trace.push(TimedState {
                    timestamp,
                    state: TaskState::Idle,
                });

                self.executor.push(TimedState {
                    timestamp,
                    state: ExecutorState::Scheduling,
                });
            }
            ExecutorEventKind::TaskReadyBegin { task_id } => {
                let task_trace = self.tasks.entry(task_id).or_default();

                if task_trace.is_empty() {
                    eprintln!("Detected TaskReadyBegin for non-existing task: {task_id}");
                }

                match task_trace.last() {
                    Some(TimedState {
                        state: TaskState::Running,
                        ..
                    }) => {
                        task_trace.push(TimedState {
                            timestamp,
                            state: TaskState::RunningWaiting,
                        });
                    }
                    Some(TimedState {
                        state: TaskState::Idle | TaskState::Spawned,
                        ..
                    }) => {
                        task_trace.push(TimedState {
                            timestamp,
                            state: TaskState::Waiting,
                        });
                    }
                    _ => {
                        eprintln!(
                            "Detected TaskReadyBegin for task that's not in the running, idle or spawned state: {task_id}"
                        );
                    }
                }
            }
            ExecutorEventKind::TaskNamed {
                task_id: _,
                name: _,
            } => unimplemented!(),
        }
    }
}

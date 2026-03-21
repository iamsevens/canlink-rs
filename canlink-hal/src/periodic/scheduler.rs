//! Periodic message scheduler.

use super::{PeriodicMessage, PeriodicStats};
use crate::{CanBackendAsync, CanError};
use std::collections::{BinaryHeap, HashMap};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

/// Commands for controlling the scheduler.
#[derive(Debug)]
pub enum SchedulerCommand {
    /// Add a new periodic message
    Add {
        /// The message to add
        message: PeriodicMessage,
        /// Reply channel for the assigned ID
        reply: oneshot::Sender<Result<u32, CanError>>,
    },
    /// Remove a periodic message
    Remove {
        /// ID of the message to remove
        id: u32,
        /// Reply channel
        reply: oneshot::Sender<Result<(), CanError>>,
    },
    /// Update message data
    UpdateData {
        /// ID of the message to update
        id: u32,
        /// New data
        data: Vec<u8>,
        /// Reply channel
        reply: oneshot::Sender<Result<(), CanError>>,
    },
    /// Update send interval
    UpdateInterval {
        /// ID of the message to update
        id: u32,
        /// New interval
        interval: Duration,
        /// Reply channel
        reply: oneshot::Sender<Result<(), CanError>>,
    },
    /// Enable or disable a message
    SetEnabled {
        /// ID of the message
        id: u32,
        /// Whether to enable
        enabled: bool,
        /// Reply channel
        reply: oneshot::Sender<Result<(), CanError>>,
    },
    /// Get statistics for a message
    GetStats {
        /// ID of the message
        id: u32,
        /// Reply channel
        reply: oneshot::Sender<Option<PeriodicStats>>,
    },
    /// List all message IDs
    ListIds {
        /// Reply channel
        reply: oneshot::Sender<Vec<u32>>,
    },
    /// Shutdown the scheduler
    Shutdown,
}

/// Entry in the scheduling priority queue.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ScheduledEntry {
    /// Next send time
    next_send: Instant,
    /// Message ID
    message_id: u32,
}

impl Ord for ScheduledEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap behavior
        other.next_send.cmp(&self.next_send)
    }
}

impl PartialOrd for ScheduledEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Internal scheduler state.
struct SchedulerState {
    /// Message configurations
    messages: HashMap<u32, PeriodicMessage>,
    /// Statistics per message
    stats: HashMap<u32, PeriodicStats>,
    /// Priority queue for scheduling
    schedule: BinaryHeap<ScheduledEntry>,
    /// Next message ID to assign
    next_id: u32,
    /// Maximum capacity
    capacity: usize,
}

impl SchedulerState {
    fn new(capacity: usize) -> Self {
        Self {
            messages: HashMap::new(),
            stats: HashMap::new(),
            schedule: BinaryHeap::new(),
            next_id: 1,
            capacity,
        }
    }

    fn add(&mut self, mut message: PeriodicMessage) -> Result<u32, CanError> {
        if self.messages.len() >= self.capacity {
            return Err(CanError::InsufficientResources {
                resource: format!(
                    "periodic message capacity exceeded (max: {})",
                    self.capacity
                ),
            });
        }

        let id = self.next_id;
        self.next_id += 1;
        message.set_id(id);

        let interval = message.interval();
        self.messages.insert(id, message);
        self.stats.insert(id, PeriodicStats::new());

        // Schedule first send
        self.schedule.push(ScheduledEntry {
            next_send: Instant::now() + interval,
            message_id: id,
        });

        Ok(id)
    }

    fn remove(&mut self, id: u32) -> Result<(), CanError> {
        if self.messages.remove(&id).is_none() {
            return Err(CanError::InvalidParameter {
                parameter: "id".to_string(),
                reason: format!("periodic message with id {id} not found"),
            });
        }
        self.stats.remove(&id);
        // Note: Entry will be cleaned up when it's popped from the queue
        Ok(())
    }
}

/// Handle for controlling a periodic scheduler.
///
/// This handle can be cloned and shared across tasks to control the scheduler.
/// The actual scheduler loop runs separately via [`run_scheduler`].
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage, run_scheduler};
/// use canlink_mock::MockBackend;
/// use tokio::task::LocalSet;
///
/// let local = LocalSet::new();
/// local.run_until(async {
///     let (scheduler, command_rx) = PeriodicScheduler::new(32);
///
///     // Spawn the scheduler loop locally (doesn't require Send)
///     tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));
///
///     // Use the scheduler handle
///     let id = scheduler.add(periodic_message).await?;
/// }).await;
/// ```
#[derive(Clone)]
pub struct PeriodicScheduler {
    /// Command sender
    command_tx: mpsc::Sender<SchedulerCommand>,
}

impl PeriodicScheduler {
    /// Create a new scheduler handle and command receiver.
    ///
    /// Returns a tuple of (handle, receiver). The receiver should be passed to
    /// [`run_scheduler`] which runs the actual scheduling loop.
    ///
    /// # Arguments
    ///
    /// * `channel_size` - Size of the command channel buffer (typically 64)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (scheduler, command_rx) = PeriodicScheduler::new(64);
    /// tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));
    /// ```
    #[must_use]
    pub fn new(channel_size: usize) -> (Self, mpsc::Receiver<SchedulerCommand>) {
        let (command_tx, command_rx) = mpsc::channel(channel_size);
        (Self { command_tx }, command_rx)
    }

    /// Add a periodic message.
    ///
    /// # Returns
    ///
    /// The unique ID assigned to the message.
    ///
    /// # Errors
    ///
    /// Returns an error if the capacity is exceeded or the scheduler is shut down.
    pub async fn add(&self, message: PeriodicMessage) -> Result<u32, CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::Add {
                message,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })?
    }

    /// Remove a periodic message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message ID is not found or the scheduler is shut down.
    pub async fn remove(&self, id: u32) -> Result<(), CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::Remove {
                id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })?
    }

    /// Update message data.
    ///
    /// # Errors
    ///
    /// Returns an error if the message ID is not found, the data is invalid,
    /// or the scheduler is shut down.
    pub async fn update_data(&self, id: u32, data: Vec<u8>) -> Result<(), CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::UpdateData {
                id,
                data,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })?
    }

    /// Update send interval.
    ///
    /// # Errors
    ///
    /// Returns an error if the message ID is not found, the interval is invalid,
    /// or the scheduler is shut down.
    pub async fn update_interval(&self, id: u32, interval: Duration) -> Result<(), CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::UpdateInterval {
                id,
                interval,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })?
    }

    /// Enable or disable a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message ID is not found or the scheduler is shut down.
    pub async fn set_enabled(&self, id: u32, enabled: bool) -> Result<(), CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::SetEnabled {
                id,
                enabled,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })?
    }

    /// Get statistics for a message.
    ///
    /// # Returns
    ///
    /// `Some(stats)` if the message exists, `None` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler is shut down.
    pub async fn get_stats(&self, id: u32) -> Result<Option<PeriodicStats>, CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::GetStats {
                id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })
    }

    /// List all message IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler is shut down.
    pub async fn list_ids(&self) -> Result<Vec<u32>, CanError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(SchedulerCommand::ListIds { reply: reply_tx })
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })?;

        reply_rx.await.map_err(|_| CanError::Other {
            message: "scheduler reply channel closed".to_string(),
        })
    }

    /// Shutdown the scheduler.
    ///
    /// This sends a shutdown command to the scheduler loop. The loop will
    /// finish processing any pending commands and then exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler is already shut down.
    pub async fn shutdown(&self) -> Result<(), CanError> {
        self.command_tx
            .send(SchedulerCommand::Shutdown)
            .await
            .map_err(|_| CanError::Other {
                message: "scheduler channel closed".to_string(),
            })
    }
}

/// Run the periodic scheduler loop.
///
/// This function runs the main scheduling loop that sends periodic messages.
/// It should be spawned as a task (using `spawn_local` for non-Send backends
/// or `spawn` for Send backends).
///
/// # Arguments
///
/// * `backend` - The CAN backend to use for sending messages
/// * `command_rx` - Command receiver from [`PeriodicScheduler::new`]
/// * `capacity` - Maximum number of periodic messages (typically 32)
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::periodic::{PeriodicScheduler, run_scheduler};
/// use tokio::task::LocalSet;
///
/// // For non-Send backends, use LocalSet
/// let local = LocalSet::new();
/// local.run_until(async {
///     let (scheduler, command_rx) = PeriodicScheduler::new(64);
///     tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));
///     // ... use scheduler
/// }).await;
///
/// // For Send backends, use regular spawn
/// let (scheduler, command_rx) = PeriodicScheduler::new(64);
/// tokio::spawn(run_scheduler(send_backend, command_rx, 32));
/// ```
#[allow(clippy::too_many_lines)]
pub async fn run_scheduler<B>(
    mut backend: B,
    mut command_rx: mpsc::Receiver<SchedulerCommand>,
    capacity: usize,
) where
    B: CanBackendAsync,
{
    let mut state = SchedulerState::new(capacity);

    loop {
        // Calculate sleep duration until next scheduled send
        let sleep_duration = state
            .schedule
            .peek()
            .map_or(Duration::from_secs(1), |entry| {
                let now = Instant::now();
                if entry.next_send > now {
                    entry.next_send - now
                } else {
                    Duration::ZERO
                }
            });

        tokio::select! {
            // Handle commands
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    SchedulerCommand::Add { message, reply } => {
                        let _ = reply.send(state.add(message));
                    }
                    SchedulerCommand::Remove { id, reply } => {
                        let _ = reply.send(state.remove(id));
                    }
                    SchedulerCommand::UpdateData { id, data, reply } => {
                        let result = if let Some(msg) = state.messages.get_mut(&id) {
                            msg.update_data(data)
                        } else {
                            Err(CanError::InvalidParameter {
                                parameter: "id".to_string(),
                                reason: format!("periodic message with id {id} not found"),
                            })
                        };
                        let _ = reply.send(result);
                    }
                    SchedulerCommand::UpdateInterval { id, interval, reply } => {
                        let result = if let Some(msg) = state.messages.get_mut(&id) {
                            msg.set_interval(interval)
                        } else {
                            Err(CanError::InvalidParameter {
                                parameter: "id".to_string(),
                                reason: format!("periodic message with id {id} not found"),
                            })
                        };
                        let _ = reply.send(result);
                    }
                    SchedulerCommand::SetEnabled { id, enabled, reply } => {
                        let result = if let Some(msg) = state.messages.get_mut(&id) {
                            msg.set_enabled(enabled);
                            Ok(())
                        } else {
                            Err(CanError::InvalidParameter {
                                parameter: "id".to_string(),
                                reason: format!("periodic message with id {id} not found"),
                            })
                        };
                        let _ = reply.send(result);
                    }
                    SchedulerCommand::GetStats { id, reply } => {
                        let message_stats = state.stats.get(&id).cloned();
                        let _ = reply.send(message_stats);
                    }
                    SchedulerCommand::ListIds { reply } => {
                        let ids: Vec<u32> = state.messages.keys().copied().collect();
                        let _ = reply.send(ids);
                    }
                    SchedulerCommand::Shutdown => {
                        break;
                    }
                }
            }

            // Handle scheduled sends
            () = tokio::time::sleep(sleep_duration) => {
                let now = Instant::now();

                // Process all due entries
                while let Some(entry) = state.schedule.peek() {
                    if entry.next_send > now {
                        break;
                    }

                    let Some(entry) = state.schedule.pop() else {
                        break;
                    };
                    let id = entry.message_id;

                    // Check if message still exists and is enabled
                    if let Some(msg) = state.messages.get(&id) {
                        if msg.is_enabled() {
                            // Send the message
                            let send_result = backend.send_message_async(msg.message()).await;

                            // Record statistics
                            if let Some(stats) = state.stats.get_mut(&id) {
                                stats.record_send(now.into());
                            }

                            // Log errors but continue (FR-006: skip on failure)
                            if let Err(e) = send_result {
                                #[cfg(feature = "tracing")]
                                tracing::warn!(
                                    "Periodic send failed for message {}: {}",
                                    id,
                                    e
                                );
                                let _ = e; // Suppress unused warning when tracing is disabled
                            }
                        }

                        // Reschedule (use current interval in case it was updated)
                        if let Some(msg) = state.messages.get(&id) {
                            state.schedule.push(ScheduledEntry {
                                next_send: now + msg.interval(),
                                message_id: id,
                            });
                        }
                    }
                    // If message was removed, don't reschedule
                }
            }
        }
    }
}

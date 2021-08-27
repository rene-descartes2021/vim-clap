mod context;
mod manager;

use std::sync::Arc;

use anyhow::Result;
use crossbeam_channel::Sender;
use futures::future::AbortHandle;
use log::debug;

use crate::stdio_server::types::{Message, ProviderId};

pub use self::context::SessionContext;
pub use self::manager::{NewSession, SessionManager};

pub type SessionId = u64;

#[async_trait::async_trait]
pub trait EventHandler: Send + Sync + 'static {
    /// Use the mutable self so that we can cache some info inside the handler.
    async fn handle_on_move(&mut self, msg: Message, context: Arc<SessionContext>) -> Result<()>;

    /// Use the mutable self so that we can cache some info inside the handler.
    async fn handle_on_typed(
        &mut self,
        msg: Message,
        context: Arc<SessionContext>,
        sender: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<()>;
}

#[derive(Debug)]
pub struct Session<T> {
    pub session_id: u64,
    pub context: Arc<SessionContext>,
    /// Each Session can have its own message processing logic.
    pub event_handler: T,
    pub event_recv: crossbeam_channel::Receiver<SessionEvent>,
    pub last_abort_handle: Option<AbortHandle>,
    pub last_rx: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl<T: Clone> Clone for Session<T> {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id,
            context: self.context.clone(),
            event_handler: self.event_handler.clone(),
            event_recv: self.event_recv.clone(),
            last_abort_handle: self.last_abort_handle.clone(),
            last_rx: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    OnTyped(Message),
    OnMove(Message),
    Terminate,
}

impl SessionEvent {
    pub fn short_display(&self) -> String {
        match self {
            Self::OnTyped(msg) => format!("OnTyped, msg id: {}", msg.id),
            Self::OnMove(msg) => format!("OnMove, msg id: {}", msg.id),
            Self::Terminate => "Terminate".into(),
        }
    }
}

impl<T: EventHandler + Clone> Session<T> {
    pub fn new(msg: Message, event_handler: T) -> (Self, Sender<SessionEvent>) {
        let (session_sender, session_receiver) = crossbeam_channel::unbounded();

        let session = Session {
            session_id: msg.session_id,
            context: Arc::new(msg.into()),
            event_handler,
            event_recv: session_receiver,
            last_abort_handle: None,
            last_rx: None,
        };

        (session, session_sender)
    }

    /// Sets the running signal to false, in case of the forerunner thread is still working.
    pub fn handle_terminate(&mut self) {
        let mut val = self.context.is_running.lock().unwrap();
        *val.get_mut() = false;
        debug!(
            "session-{}-{} terminated",
            self.session_id,
            self.provider_id()
        );
    }

    /// This session is still running, hasn't received Terminate event.
    pub fn is_running(&self) -> bool {
        self.context
            .is_running
            .lock()
            .unwrap()
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Saves the forerunner result.
    /// TODO: Store full lines, or a cached file?
    pub fn set_source_list(&mut self, lines: Vec<String>) {
        let mut source_list = self.context.source_list.lock().unwrap();
        *source_list = Some(lines);
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.context.provider_id
    }

    pub fn start_event_loop(mut self) -> Result<()> {
        tokio::spawn(async move {
            debug!(
                "spawn a new task for session-{}-{}",
                self.session_id,
                self.provider_id()
            );
            loop {
                match self.event_recv.recv() {
                    Ok(event) => {
                        debug!(
                            "Event(in) receive a session event: {:?}",
                            event.short_display()
                        );
                        match event {
                            SessionEvent::Terminate => {
                                self.handle_terminate();
                                return;
                            }
                            SessionEvent::OnMove(msg) => {
                                if let Err(e) = self
                                    .event_handler
                                    .handle_on_move(msg, self.context.clone())
                                    .await
                                {
                                    debug!("Error occurrred when handling OnMove event: {:?}", e);
                                }
                            }
                            SessionEvent::OnTyped(msg) => {
                                if let Some(mut rx) = self.last_rx {
                                    log::debug!("Checking last_rx");
                                    // Last OnTyped filtering is done.
                                    if let Ok(()) = rx.try_recv() {
                                        log::debug!("Received a message from last_rx");
                                    } else {
                                        log::debug!("Received no message from last_rx");
                                        // Kill the last job if it's still running.
                                        if let Some(abort_last) = self.last_abort_handle {
                                            abort_last.abort();
                                        }
                                    }

                                    rx.close();
                                }

                                let (tx, rx) = tokio::sync::oneshot::channel();

                                self.last_rx = Some(rx);

                                let mut event_handler_clone = self.event_handler.clone();
                                let context_clone = self.context.clone();

                                let (task, handle) = futures::future::abortable(async move {
                                    if let Err(e) = event_handler_clone
                                        .handle_on_typed(msg, context_clone, Some(tx))
                                        .await
                                    {
                                        debug!(
                                            "Error occurrred when handling OnTyped event: {:?}",
                                            e
                                        );
                                    }
                                });

                                self.last_abort_handle = Some(handle);

                                tokio::spawn(task).await;
                            }
                        }
                    }
                    Err(err) => debug!(
                        "The channel is possibly disconnected, session recv error: {:?}",
                        err
                    ),
                }
            }
        });

        Ok(())
    }
}

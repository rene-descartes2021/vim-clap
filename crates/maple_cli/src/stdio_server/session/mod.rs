mod context;
mod manager;

use std::sync::Arc;

use anyhow::Result;
use crossbeam_channel::Sender;
use futures::future::AbortHandle;
use log::debug;

use crate::stdio_server::event_handlers::on_init::on_create;
use crate::stdio_server::types::{Message, ProviderId};

pub use self::context::{Scale, SessionContext};
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
        stop_recv: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<()>;

    fn notify_on_typed_done(sender: Option<tokio::sync::oneshot::Sender<()>>) {
        if let Some(sender) = sender {
            if let Err(e) = sender.send(()) {
                log::error!("Failed to send: {:?}", e);
            }
        }
    }
}

#[derive(Debug)]
pub struct Session<T> {
    pub session_id: u64,
    pub context: Arc<SessionContext>,
    /// Each Session can have its own message processing logic.
    pub event_handler: T,
    pub event_recv: crossbeam_channel::Receiver<SessionEvent>,
    pub source_scale: Scale,
    pub last_on_typed_is_running: bool,
    pub last_on_typed_abort_handle: Option<AbortHandle>,
    pub last_on_typed_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    pub last_on_typed_stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl<T: Clone> Clone for Session<T> {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id,
            context: self.context.clone(),
            event_handler: self.event_handler.clone(),
            event_recv: self.event_recv.clone(),
            source_scale: self.source_scale.clone(),
            last_on_typed_is_running: self.last_on_typed_is_running.clone(),
            last_on_typed_abort_handle: self.last_on_typed_abort_handle.clone(),
            last_on_typed_rx: None,
            last_on_typed_stop_tx: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    OnTyped(Message),
    OnMove(Message),
    Create,
    Terminate,
}

impl SessionEvent {
    pub fn short_display(&self) -> String {
        match self {
            Self::OnTyped(msg) => format!("OnTyped, msg id: {}", msg.id),
            Self::OnMove(msg) => format!("OnMove, msg id: {}", msg.id),
            Self::Create => "Create".into(),
            Self::Terminate => "Terminate".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InitialJobResult {
    pub total: usize,
    pub lines: Option<Vec<String>>,
}

impl<T: EventHandler + Clone> Session<T> {
    pub fn new(msg: Message, event_handler: T) -> (Self, Sender<SessionEvent>) {
        let (session_sender, session_receiver) = crossbeam_channel::unbounded();

        let session = Session {
            session_id: msg.session_id,
            context: Arc::new(msg.into()),
            event_handler,
            event_recv: session_receiver,
            source_scale: Scale::Indefinite,
            last_on_typed_is_running: false,
            last_on_typed_abort_handle: None,
            last_on_typed_rx: None,
            last_on_typed_stop_tx: None,
        };

        (session, session_sender)
    }

    /// Sets the running signal to false, in case of the forerunner thread is still working.
    pub fn handle_terminate(&mut self) {
        let mut val = self.context.is_running.lock();
        *val = false.into();
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
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Saves the forerunner result.
    /// TODO: Store full lines, or a cached file?
    pub fn set_source_list(&mut self, lines: Vec<String>) {
        let mut source_list = self.context.source_list.lock();
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
                            SessionEvent::Create => {
                                let context_clone = self.context.clone();

                                match tokio::spawn(async move {
                                    match tokio::time::timeout(
                                        std::time::Duration::from_millis(300),
                                        on_create(context_clone),
                                    )
                                    .await
                                    {
                                        Ok(scale) => Some(scale),
                                        Err(_) => None, // timeout
                                    }
                                })
                                .await
                                {
                                    Ok(Some(Ok(scale))) => {
                                        log::debug!("============= receive scale: {:?}", scale);
                                        if let Some(total) = scale.total() {
                                            let method = "s:set_total_size";
                                            log::debug!("============= Setting total: {}", total);
                                            utility::println_json_with_length!(total, method);
                                        }
                                        self.source_scale = scale;
                                    }
                                    Ok(Some(Err(e))) => {
                                        log::error!("Error occurrred inside on_create(): {:?}", e);
                                    }
                                    Ok(None) => {
                                        log::debug!("Did not receive value with 300 ms, keep the large scale");
                                    }
                                    Err(e) => {
                                        log::error!("Error occurrred in the Create future: {:?}", e)
                                    }
                                }
                            }
                            SessionEvent::Terminate => {
                                self.handle_terminate();
                                return;
                            }
                            SessionEvent::OnMove(msg) => {
                                if !self.last_on_typed_is_running {
                                    // TODO: in case of the overwhelm of OnTyped messages, a debounce
                                    // should be added.
                                    if let Err(e) = self
                                        .event_handler
                                        .handle_on_move(msg, self.context.clone())
                                        .await
                                    {
                                        debug!(
                                            "Error occurrred when handling OnMove event: {:?}",
                                            e
                                        );
                                    }
                                } else {
                                    log::debug!("Ignoring OnMove message since `last_on_typed_is_running` is true");
                                }
                            }
                            SessionEvent::OnTyped(msg) => {
                                // Add debounce according to the scale.
                                if self.last_on_typed_is_running {
                                    if let Some(stop_tx) = self.last_on_typed_stop_tx {
                                        log::debug!("Sending last_on_typed_stop_tx =================================== Aborting ");
                                        stop_tx.send(()).unwrap();
                                    }

                                    if let Some(abort_last) = self.last_on_typed_abort_handle {
                                        log::debug!("=================================== Aborting last OnTyped");
                                        // It may not immediately stop running.
                                        abort_last.abort();
                                    }
                                }

                                let (tx, rx) = tokio::sync::oneshot::channel();

                                let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();

                                self.last_on_typed_rx = Some(rx);
                                self.last_on_typed_stop_tx = Some(stop_tx);

                                let mut event_handler_clone = self.event_handler.clone();
                                let context_clone = self.context.clone();

                                let (on_typed_task, handle) =
                                    futures::future::abortable(async move {
                                        if let Err(e) = event_handler_clone
                                            .handle_on_typed(
                                                msg,
                                                context_clone,
                                                Some(tx),
                                                Some(stop_rx),
                                            )
                                            .await
                                        {
                                            debug!(
                                                "Error occurrred when handling OnTyped event: {:?}",
                                                e
                                            );
                                        }
                                    });

                                self.last_on_typed_is_running = true;
                                self.last_on_typed_abort_handle = Some(handle);

                                match tokio::spawn(on_typed_task).await {
                                    Ok(_) => {
                                        log::debug!("Last OnTyped job is done successfully");
                                        self.last_on_typed_is_running = false;
                                    }
                                    Err(e) => {
                                        log::debug!("Last OnTyped job is done with error: {:?}", e);
                                        self.last_on_typed_is_running = false;
                                    }
                                }
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

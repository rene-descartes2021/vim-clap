//! Processes the RPC message wrapper in Event type.

pub mod on_init;
pub mod on_move;
pub mod on_typed;

use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use crate::stdio_server::{
    session::{EventHandler, SessionContext},
    write_response,
};

pub use on_move::{OnMove, OnMoveHandler};

use super::types::Message;

#[derive(Clone)]
pub struct DefaultEventHandler;

#[async_trait::async_trait]
impl EventHandler for DefaultEventHandler {
    async fn handle_on_move(&mut self, msg: Message, context: Arc<SessionContext>) -> Result<()> {
        log::debug!("[handle_on_move] DefaultEventHandler");

        let msg_id = msg.id;
        if let Err(e) = on_move::OnMoveHandler::create(&msg, &context, None).map(|x| x.handle()) {
            log::error!("Failed to handle OnMove event: {:?}", e);
            write_response(json!({"error": e.to_string(), "id": msg_id }));
        }

        Ok(())
    }

    async fn handle_on_typed(
        &mut self,
        msg: Message,
        context: Arc<SessionContext>,
        sender: Option<tokio::sync::oneshot::Sender<()>>,
        stop_recv: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<()> {
        log::debug!("calling DefaultEventHandler handle_on_typed");

        use filter::{dyn_run, dyn_run_with_stop_signal, FilterContext, Source};

        match context.provider_id.as_str() {
            "blines" => {
                dyn_run_with_stop_signal(
                    &msg.get_query(),
                    Source::List(
                        std::fs::read_to_string(&context.start_buffer_path)?
                            .lines()
                            .enumerate()
                            .map(|(idx, item)| format!("{} {}", idx + 1, item))
                            .map(Into::into),
                    ),
                    FilterContext::new(
                        None,
                        Some(filter::ITEMS_TO_SHOW),
                        Some(context.display_winwidth as usize),
                        None,
                        filter::matcher::MatchType::Full,
                    ),
                    Default::default(),
                    stop_recv.unwrap(),
                )?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown provider_id: {} in general handle_on_typed",
                    context.provider_id
                ));
            }
        }

        Self::notify_on_typed_done(sender);

        Ok(())
    }
}

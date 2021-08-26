//! Processes the RPC message wrapper in Event type.

pub mod on_init;
pub mod on_move;
pub mod on_typed;

use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use crate::stdio_server::{
    session::{Event, EventHandler, SessionContext},
    write_response,
};

pub use on_move::{OnMove, OnMoveHandler};

use super::types::Message;

#[derive(Clone)]
pub struct DefaultEventHandler;

#[async_trait::async_trait]
impl EventHandler for DefaultEventHandler {
    async fn handle(&mut self, event: Event, context: Arc<SessionContext>) -> Result<()> {
        match event {
            Event::OnMove(msg) => {
                let msg_id = msg.id;
                if let Err(e) =
                    on_move::OnMoveHandler::create(&msg, &context, None).map(|x| x.handle())
                {
                    log::error!("Failed to handle OnMove event: {:?}", e);
                    write_response(json!({"error": e.to_string(), "id": msg_id }));
                }
            }
            // Event::OnTyped(msg) => on_typed::handle_on_typed(msg, &context),
            //
            // TODO: kill last unfinished job and start new one.
            Event::OnTyped(msg) => {
                if let Err(e) = handle_on_typed(msg, &context) {
                    log::error!("Error occurred when handling OnTyped message: {:?}", e);
                }
            }
        }
        Ok(())
    }
}

fn handle_on_typed(msg: Message, context: &SessionContext) -> Result<()> {
    use filter::{dyn_run, FilterContext, Source};

    match context.provider_id.as_str() {
        "blines" => {
            dyn_run(
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
                    Some(30),
                    Some(context.display_winwidth as usize),
                    None,
                    filter::matcher::MatchType::Full,
                ),
                Default::default(),
            )?;
        }
        _ => log::error!("Unknown provider_id in general handle_on_typed"),
    }

    Ok(())
}

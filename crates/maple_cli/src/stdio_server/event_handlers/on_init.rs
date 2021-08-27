use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use crate::process::AsyncCommand;
use crate::stdio_server::session::SessionContext;
use crate::stdio_server::{
    session::{EventHandler, Scale, Session},
    write_response,
};

pub async fn run<T: EventHandler + Clone>(
    msg_id: u64,
    source_cmd: String,
    session: Session<T>,
) -> Result<()> {
    let lines = AsyncCommand::new(source_cmd)
        .current_dir(&session.context.cwd)
        .lines()
        .await?;

    if session.is_running() {
        // Send the forerunner result to client.
        let initial_size = lines.len();
        let response_lines = lines
            .iter()
            .by_ref()
            .take(30)
            .map(|line| icon::IconPainter::File.paint(&line))
            .collect::<Vec<_>>();
        write_response(json!({
        "id": msg_id,
        "provider_id": session.context.provider_id,
        "result": {
          "event": "on_init",
          "initial_size": initial_size,
          "lines": response_lines,
        }}));

        let mut session = session;
        session.set_source_list(lines);
    }

    Ok(())
}

pub fn on_create(
    context: Arc<SessionContext>,
    sender: tokio::sync::oneshot::Sender<Scale>,
) -> Result<()> {
    if context.provider_id.as_str() == "blines" {
        let total = crate::utils::count_lines(std::fs::File::open(&context.start_buffer_path)?)?;

        let scale = if total > 500_000 {
            Scale::Large
        } else {
            Scale::Affordable(total)
        };

        let method = "s:set_total_size";
        utility::println_json_with_length!(total, method);

        sender.send(scale).expect("Failed to send inside on_create");
    }

    Ok(())
}

use std::path::PathBuf;

use anyhow::Result;
use crossbeam_channel::Sender;
use log::error;
use serde::Deserialize;
use serde_json::json;

use crate::commands::dumb_jump::{DumbJump, Lines};
use crate::stdio_server::{
    session::{
        Event, EventHandler, NewSession, OnMoveHandler, Session, SessionContext, SessionEvent,
    },
    write_response, Message,
};

const CMD: &str = "ctags -R -x --output-format=json --fields=+n";

/// Generate ctags recursively given the directory.
#[derive(Debug, Clone)]
pub struct ProjTags {
    dir: PathBuf,
}

impl ProjTags {
    pub fn execute(&self) -> Vec<String> {
        crate::commands::recursive_tags::create_tags_stream(CMD, &self.dir)
            .unwrap()
            .collect()
    }
}

pub async fn collect_dumb_jump_source(
    msg: Message,
    session: Session<ProjTagsMessageHandler>,
) -> Result<()> {
    let msg_id = msg.id;

    let dir: PathBuf = session.context.cwd.clone().into();
    let proj_tags = ProjTags { dir };
    let lines = proj_tags.execute();

    if session.is_running() {
        // Send the forerunner result to client.
        let initial_size = lines.len();
        let response_lines = lines
            .iter()
            .by_ref()
            .take(100)
            .map(|line| icon::IconPainter::ProjTags.paint(&line))
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
        log::debug!("========== setting source list: {}", initial_size);
        session.set_source_list(lines);
    }

    Ok(())
}

pub async fn handle_dumb_jump_message(msg: Message, force_execute: bool) -> Vec<String> {
    let msg_id = msg.id;

    #[derive(Deserialize)]
    struct Params {
        cwd: String,
        query: String,
        extension: String,
    }

    let Params {
        cwd,
        query,
        extension,
    } = msg
        .deserialize_params()
        .unwrap_or_else(|e| panic!("Failed to deserialize dumb_jump params: {:?}", e));

    if query.is_empty() {
        return Default::default();
    }

    let dumb_jump = DumbJump {
        word: query,
        extension,
        kind: None,
        cmd_dir: Some(cwd.into()),
    };

    let result = match dumb_jump.references_or_occurrences(false).await {
        Ok(Lines { lines, mut indices }) => {
            let total_lines = lines;

            let total = total_lines.len();
            // Only show the top 200 items.
            let lines = total_lines.iter().take(200).clone().collect::<Vec<_>>();
            indices.truncate(200);
            let result = json!({
            "lines": lines,
            "indices": indices,
            "total": total,
            });

            let result = json!({
              "id": msg_id,
              "force_execute": force_execute,
              "provider_id": "dumb_jump",
              "result": result,
            });

            write_response(result);

            return total_lines;
        }
        Err(e) => {
            error!("Error when running dumb_jump: {:?}", e);
            let error = json!({"message": e.to_string()});
            json!({ "id": msg_id, "provider_id": "dumb_jump", "error": error })
        }
    };

    write_response(result);

    Default::default()
}

#[derive(Debug, Clone, Default)]
pub struct ProjTagsMessageHandler {
    /// When passing the line content from Vim to Rust, for
    /// these lines that are extremely long, the performance
    /// of Vim can become very bad, we cache the display lines
    /// on Rust to pass the line number instead.
    lines: Vec<String>,
}

#[async_trait::async_trait]
impl EventHandler for ProjTagsMessageHandler {
    async fn handle(&mut self, event: Event, context: SessionContext) {
        match event {
            Event::OnMove(msg) => {
                let msg_id = msg.id;

                // lnum is 1-indexed
                if let Err(e) = OnMoveHandler::try_new(&msg, &context, None).map(|x| x.handle()) {
                    log::error!("Failed to handle OnMove event: {:?}", e);
                    write_response(json!({"error": e.to_string(), "id": msg_id }));
                }
            }
            Event::OnTyped(msg) => {
                let lines = tokio::spawn(handle_dumb_jump_message(msg, false))
                    .await
                    .unwrap_or_else(|e| {
                        log::error!(
                            "Failed to spawn a task for handle_dumb_jump_message: {:?}",
                            e
                        );
                        Default::default()
                    });
                self.lines = lines;
            }
        }
    }
}

pub struct ProjTagsSession;

impl NewSession for ProjTagsSession {
    fn spawn(msg: Message) -> Result<Sender<SessionEvent>> {
        let (session_sender, session_receiver) = crossbeam_channel::unbounded();

        let session = Session {
            session_id: msg.session_id,
            context: msg.clone().into(),
            event_handler: ProjTagsMessageHandler::default(),
            event_recv: session_receiver,
        };

        let session_clone = session.clone();

        session.start_event_loop()?;

        // TODO: choose different fitler strategy according to the time forerunner job spent.
        tokio::spawn(async move {
            let msg_id = msg.id;
            if let Err(e) = collect_dumb_jump_source(msg, session_clone).await {
                log::error!(
                    "Error occurred when running the forerunner job, msg_id: {}, error: {:?}",
                    msg_id,
                    e
                );
            }
        });

        Ok(session_sender)
    }
}

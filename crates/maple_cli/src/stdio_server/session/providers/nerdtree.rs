use std::path::{self, Path, PathBuf};
use std::{fs, io};

use anyhow::Result;
use crossbeam_channel::Sender;
use log::debug;
use serde_json::json;

use icon::prepend_filer_icon;
use nerdtree::PathNode;

use crate::stdio_server::{
    session::{
        build_abs_path, Event, EventHandler, NewSession, OnMove, OnMoveHandler, Session,
        SessionContext, SessionEvent,
    },
    write_response, Message,
};

pub fn handle_nerdtree_message(msg: Message) {
    let cwd = msg.get_cwd();
    let lnum = msg.get_lnum();
    debug!("Recv nerdtree params: cwd:{}", cwd,);

    let mut root = PathNode::new_expanded(&cwd);

    let lines = root.expand_at(lnum);

    let result = json!({
    "lines": lines,
    });

    let result = json!({ "id": msg.id, "provider_id": "nerdtree", "result": result });

    write_response(result);
}

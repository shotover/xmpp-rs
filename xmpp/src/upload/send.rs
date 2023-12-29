// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::Path;
use tokio::fs::File;
use tokio_xmpp::{
    parsers::{http_upload::SlotRequest, iq::Iq},
    Jid,
};

use crate::Agent;

pub async fn upload_file_with(agent: &mut Agent, service: &str, path: &Path) {
    let name = path.file_name().unwrap().to_str().unwrap().to_string();
    let file = File::open(path).await.unwrap();
    let size = file.metadata().await.unwrap().len();
    let slot_request = SlotRequest {
        filename: name,
        size: size,
        content_type: None,
    };
    let to = service.parse::<Jid>().unwrap();
    let request = Iq::from_get("upload1", slot_request).with_to(to.clone());
    agent
        .uploads
        .push((String::from("upload1"), to, path.to_path_buf()));
    agent.client.send_stanza(request.into()).await.unwrap();
}

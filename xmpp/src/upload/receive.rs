// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use reqwest::{
    header::HeaderMap as ReqwestHeaderMap, Body as ReqwestBody, Client as ReqwestClient,
};
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_xmpp::{
    parsers::http_upload::{Header as HttpUploadHeader, SlotResult},
    Element, Jid,
};

use crate::{Agent, Event};

pub async fn handle_upload_result(
    from: &Jid,
    iqid: String,
    elem: Element,
    agent: &mut Agent,
) -> impl IntoIterator<Item = Event> {
    let mut res: Option<(usize, PathBuf)> = None;

    for (i, (id, to, filepath)) in agent.uploads.iter().enumerate() {
        if to == from && id == &iqid {
            res = Some((i, filepath.to_path_buf()));
            break;
        }
    }

    if let Some((index, file)) = res {
        agent.uploads.remove(index);
        let slot = SlotResult::try_from(elem).unwrap();

        let mut headers = ReqwestHeaderMap::new();
        for header in slot.put.headers {
            let (attr, val) = match header {
                HttpUploadHeader::Authorization(val) => ("Authorization", val),
                HttpUploadHeader::Cookie(val) => ("Cookie", val),
                HttpUploadHeader::Expires(val) => ("Expires", val),
            };
            headers.insert(attr, val.parse().unwrap());
        }

        let web = ReqwestClient::new();
        let stream = FramedRead::new(File::open(file).await.unwrap(), BytesCodec::new());
        let body = ReqwestBody::wrap_stream(stream);
        let res = web
            .put(slot.put.url.as_str())
            .headers(headers)
            .body(body)
            .send()
            .await
            .unwrap();
        if res.status() == 201 {
            return vec![Event::HttpUploadedFile(slot.get.url)];
        }
    }

    return vec![];
}

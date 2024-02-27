// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env::args;
use std::str::FromStr;
use tokio_xmpp::parsers::{message::MessageType, BareJid, Jid};
use xmpp::{ClientBuilder, ClientFeature, ClientType, Event};

#[tokio::main]
async fn main() -> Result<(), Option<()>> {
    env_logger::init();

    let args: Vec<String> = args().collect();
    if args.len() != 3 {
        println!("Usage: {} <jid> <password>", args[0]);
        return Err(None);
    }

    let jid = BareJid::from_str(&args[1]).expect(&format!("Invalid JID: {}", &args[1]));
    let password = &args[2];

    // Client instance
    let mut client = ClientBuilder::new(jid, password)
        .set_client(ClientType::Bot, "xmpp-rs")
        .set_website("https://gitlab.com/xmpp-rs/xmpp-rs")
        .set_default_nick("bot")
        .enable_feature(ClientFeature::Avatars)
        .enable_feature(ClientFeature::ContactList)
        .enable_feature(ClientFeature::JoinRooms)
        .build();

    while let Some(events) = client.wait_for_events().await {
        for event in events {
            match event {
                Event::Online => {
                    println!("Online.");
                }
                Event::Disconnected(e) => {
                    println!("Disconnected because of {}.", e);
                    return Err(None);
                }
                Event::ContactAdded(contact) => {
                    println!("Contact {} added.", contact.jid);
                }
                Event::ContactRemoved(contact) => {
                    println!("Contact {} removed.", contact.jid);
                }
                Event::ContactChanged(contact) => {
                    println!("Contact {} changed.", contact.jid);
                }
                Event::ChatMessage(_id, jid, body, time_info) => {
                    println!("Message from {} at {}: {}", jid, time_info.received, body.0);
                }
                Event::JoinRoom(jid, conference) => {
                    println!("Joining room {} ({:?})…", jid, conference.name);
                    client
                        .join_room(
                            jid,
                            conference.nick,
                            conference.password,
                            "en",
                            "Yet another bot!",
                        )
                        .await;
                }
                Event::LeaveRoom(jid) => {
                    println!("Leaving room {}…", jid);
                }
                Event::LeaveAllRooms => {
                    println!("Leaving all rooms…");
                }
                Event::RoomJoined(jid) => {
                    println!("Joined room {}.", jid);
                    client
                        .send_message(Jid::Bare(jid), MessageType::Groupchat, "en", "Hello world!")
                        .await;
                }
                Event::RoomLeft(jid) => {
                    println!("Left room {}.", jid);
                }
                Event::RoomMessage(_id, jid, nick, body, time_info) => {
                    println!(
                        "Message in room {} from {} at {}: {}",
                        jid, nick, time_info.received, body.0
                    );
                }
                Event::AvatarRetrieved(jid, path) => {
                    println!("Received avatar for {} in {}.", jid, path);
                }
                _ => (),
            }
        }
    }

    Ok(())
}

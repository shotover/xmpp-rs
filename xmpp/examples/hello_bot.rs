// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use env_logger;
use std::env::args;
use xmpp::{ClientBuilder, ClientFeature, ClientType, Event};
use xmpp_parsers::{message::MessageType, Jid};

#[tokio::main]
async fn main() -> Result<(), Option<()>> {
    env_logger::init();

    let args: Vec<String> = args().collect();
    if args.len() != 3 {
        println!("Usage: {} <jid> <password>", args[0]);
        return Err(None);
    }
    let jid = &args[1];
    let password = &args[2];

    // Client instance
    let mut client = ClientBuilder::new(jid, password)
        .set_client(ClientType::Bot, "xmpp-rs")
        .set_website("https://gitlab.com/xmpp-rs/xmpp-rs")
        .set_default_nick("bot")
        .enable_feature(ClientFeature::Avatars)
        .enable_feature(ClientFeature::ContactList)
        .enable_feature(ClientFeature::JoinRooms)
        .build()
        .unwrap();

    while let Some(events) = client.wait_for_events().await {
        for event in events {
            match event {
                Event::Online => {
                    println!("Online.");
                }
                Event::Disconnected => {
                    println!("Disconnected");
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
                Event::ChatMessage(_id, jid, body) => {
                    println!("Message from {}: {}", jid, body.0);
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
                Event::RoomMessage(_id, jid, nick, body) => {
                    println!("Message in room {} from {}: {}", jid, nick, body.0);
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

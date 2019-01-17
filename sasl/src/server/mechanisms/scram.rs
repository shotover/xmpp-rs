use std::marker::PhantomData;

use base64;

use crate::common::scram::{generate_nonce, ScramProvider};
use crate::common::{parse_frame, xor, ChannelBinding, Identity};
use crate::secret;
use crate::secret::Pbkdf2Secret;
use crate::server::{Mechanism, Provider, Response};

enum ScramState {
    Init,
    SentChallenge {
        initial_client_message: Vec<u8>,
        initial_server_message: Vec<u8>,
        gs2_header: Vec<u8>,
        server_nonce: String,
        identity: Identity,
        salted_password: Vec<u8>,
    },
    Done,
}

pub struct Scram<S, P>
where
    S: ScramProvider,
    P: Provider<S::Secret>,
    S::Secret: secret::Pbkdf2Secret,
{
    name: String,
    state: ScramState,
    channel_binding: ChannelBinding,
    provider: P,
    _marker: PhantomData<S>,
}

impl<S, P> Scram<S, P>
where
    S: ScramProvider,
    P: Provider<S::Secret>,
    S::Secret: secret::Pbkdf2Secret,
{
    pub fn new(provider: P, channel_binding: ChannelBinding) -> Scram<S, P> {
        Scram {
            name: format!("SCRAM-{}", S::name()),
            state: ScramState::Init,
            channel_binding: channel_binding,
            provider: provider,
            _marker: PhantomData,
        }
    }
}

impl<S, P> Mechanism for Scram<S, P>
where
    S: ScramProvider,
    P: Provider<S::Secret>,
    S::Secret: secret::Pbkdf2Secret,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn respond(&mut self, payload: &[u8]) -> Result<Response, String> {
        let next_state;
        let ret;
        match self.state {
            ScramState::Init => {
                // TODO: really ugly, mostly because parse_frame takes a &[u8] and i don't
                //       want to double validate utf-8
                //
                //       NEED TO CHANGE THIS THOUGH. IT'S AWFUL.
                let mut commas = 0;
                let mut idx = 0;
                for &b in payload {
                    idx += 1;
                    if b == 0x2C {
                        commas += 1;
                        if commas >= 2 {
                            break;
                        }
                    }
                }
                if commas < 2 {
                    return Err("failed to decode message".to_owned());
                }
                let gs2_header = payload[..idx].to_vec();
                let rest = payload[idx..].to_vec();
                // TODO: process gs2 header properly, not this ugly stuff
                match self.channel_binding {
                    ChannelBinding::None | ChannelBinding::Unsupported => {
                        // Not supported.
                        if gs2_header[0] != 0x79 {
                            // ord("y")
                            return Err("channel binding not supported".to_owned());
                        }
                    }
                    ref other => {
                        // Supported.
                        if gs2_header[0] == 0x79 {
                            // ord("y")
                            return Err("channel binding is supported".to_owned());
                        } else if !other.supports("tls-unique") {
                            // TODO: grab the data
                            return Err("channel binding mechanism incorrect".to_owned());
                        }
                    }
                }
                let frame =
                    parse_frame(&rest).map_err(|_| "can't decode initial message".to_owned())?;
                let username = frame.get("n").ok_or_else(|| "no username".to_owned())?;
                let identity = Identity::Username(username.to_owned());
                let client_nonce = frame.get("r").ok_or_else(|| "no nonce".to_owned())?;
                let mut server_nonce = String::new();
                server_nonce += client_nonce;
                server_nonce +=
                    &generate_nonce().map_err(|_| "failed to generate nonce".to_owned())?;
                let pbkdf2 = self.provider.provide(&identity)?;
                let mut buf = Vec::new();
                buf.extend(b"r=");
                buf.extend(server_nonce.bytes());
                buf.extend(b",s=");
                buf.extend(base64::encode(pbkdf2.salt()).bytes());
                buf.extend(b",i=");
                buf.extend(pbkdf2.iterations().to_string().bytes());
                ret = Response::Proceed(buf.clone());
                next_state = ScramState::SentChallenge {
                    server_nonce: server_nonce,
                    identity: identity,
                    salted_password: pbkdf2.digest().to_vec(),
                    initial_client_message: rest,
                    initial_server_message: buf,
                    gs2_header: gs2_header,
                };
            }
            ScramState::SentChallenge {
                ref server_nonce,
                ref identity,
                ref salted_password,
                ref gs2_header,
                ref initial_client_message,
                ref initial_server_message,
            } => {
                let frame = parse_frame(payload).map_err(|_| "can't decode response".to_owned())?;
                let mut cb_data: Vec<u8> = Vec::new();
                cb_data.extend(gs2_header);
                cb_data.extend(self.channel_binding.data());
                let mut client_final_message_bare = Vec::new();
                client_final_message_bare.extend(b"c=");
                client_final_message_bare.extend(base64::encode(&cb_data).bytes());
                client_final_message_bare.extend(b",r=");
                client_final_message_bare.extend(server_nonce.bytes());
                let client_key = S::hmac(b"Client Key", &salted_password);
                let server_key = S::hmac(b"Server Key", &salted_password);
                let mut auth_message = Vec::new();
                auth_message.extend(initial_client_message);
                auth_message.extend(b",");
                auth_message.extend(initial_server_message);
                auth_message.extend(b",");
                auth_message.extend(client_final_message_bare.clone());
                let stored_key = S::hash(&client_key);
                let client_signature = S::hmac(&auth_message, &stored_key);
                let client_proof = xor(&client_key, &client_signature);
                let sent_proof = frame.get("p").ok_or_else(|| "no proof".to_owned())?;
                let sent_proof =
                    base64::decode(sent_proof).map_err(|_| "can't decode proof".to_owned())?;
                if client_proof != sent_proof {
                    return Err("authentication failed".to_owned());
                }
                let server_signature = S::hmac(&auth_message, &server_key);
                let mut buf = Vec::new();
                buf.extend(b"v=");
                buf.extend(base64::encode(&server_signature).bytes());
                ret = Response::Success(identity.clone(), buf);
                next_state = ScramState::Done;
            }
            ScramState::Done => {
                return Err("sasl session is already over".to_owned());
            }
        }
        self.state = next_state;
        Ok(ret)
    }
}

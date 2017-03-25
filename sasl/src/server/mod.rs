use common::{Credentials, Identity};

#[cfg(feature = "scram")]
use common::scram::ScramProvider;

pub trait Validator {
    fn validate_credentials(&self, credentials: &Credentials) -> Result<Identity, String>;

    #[cfg(feature = "scram")]
    fn request_pbkdf2<S: ScramProvider>(&self) -> Result<(Vec<u8>, usize, Vec<u8>), String>;
}

pub trait Mechanism<V: Validator> {
    fn name(&self) -> &str;
    fn respond(&mut self, payload: &[u8]) -> Result<Response, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Success(Identity, Vec<u8>),
    Proceed(Vec<u8>),
}

pub mod mechanisms {
    mod plain {
        use common::{ChannelBinding, Credentials, Identity, Secret};
        use server::{Mechanism, Response, Validator};

        pub struct Plain<V: Validator> {
            validator: V,
        }

        impl<V: Validator> Plain<V> {
            pub fn new(validator: V) -> Plain<V> {
                Plain {
                    validator: validator,
                }
            }
        }

        impl<V: Validator> Mechanism<V> for Plain<V> {
            fn name(&self) -> &str {
                "PLAIN"
            }

            fn respond(&mut self, payload: &[u8]) -> Result<Response, String> {
                let mut sp = payload.split(|&b| b == 0);
                sp.next();
                let username = sp
                    .next()
                    .ok_or_else(|| "no username specified".to_owned())?;
                let username =
                    String::from_utf8(username.to_vec()).map_err(|_| "error decoding username")?;
                let password = sp
                    .next()
                    .ok_or_else(|| "no password specified".to_owned())?;
                let password =
                    String::from_utf8(password.to_vec()).map_err(|_| "error decoding password")?;
                let creds = Credentials {
                    identity: Identity::Username(username),
                    secret: Secret::password_plain(password),
                    channel_binding: ChannelBinding::None,
                };
                let ret = self.validator.validate_credentials(&creds)?;
                Ok(Response::Success(ret, Vec::new()))
            }
        }
    }

    #[cfg(feature = "scram")]
    mod scram {
        use std::marker::PhantomData;

        use base64;

        use common::scram::{generate_nonce, ScramProvider};
        use common::{parse_frame, xor, ChannelBinding, Credentials, Identity, Secret};
        use server::{Mechanism, Response, Validator};

        enum ScramState {
            Init,
            SentChallenge {
                initial_client_message: Vec<u8>,
                initial_server_message: Vec<u8>,
                gs2_header: Vec<u8>,
                server_nonce: String,
                username: String,
                salted_password: Vec<u8>,
            },
            Done,
        }

        pub struct Scram<S: ScramProvider, V: Validator> {
            name: String,
            state: ScramState,
            channel_binding: ChannelBinding,
            validator: V,
            _marker: PhantomData<S>,
        }

        impl<S: ScramProvider, V: Validator> Scram<S, V> {
            pub fn new(validator: V, channel_binding: ChannelBinding) -> Scram<S, V> {
                Scram {
                    name: format!("SCRAM-{}", S::name()),
                    state: ScramState::Init,
                    channel_binding: channel_binding,
                    validator: validator,
                    _marker: PhantomData,
                }
            }
        }

        impl<S: ScramProvider, V: Validator> Mechanism<V> for Scram<S, V> {
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
                        let frame = parse_frame(&rest)
                            .map_err(|_| "can't decode initial message".to_owned())?;
                        let username = frame.get("n").ok_or_else(|| "no username".to_owned())?;
                        let client_nonce = frame.get("r").ok_or_else(|| "no nonce".to_owned())?;
                        let mut server_nonce = String::new();
                        server_nonce += client_nonce;
                        server_nonce +=
                            &generate_nonce().map_err(|_| "failed to generate nonce".to_owned())?;
                        let (salt, iterations, data) = self.validator.request_pbkdf2::<S>()?;
                        let mut buf = Vec::new();
                        buf.extend(b"r=");
                        buf.extend(server_nonce.bytes());
                        buf.extend(b",s=");
                        buf.extend(base64::encode(&salt).bytes());
                        buf.extend(b",i=");
                        buf.extend(iterations.to_string().bytes());
                        ret = Response::Proceed(buf.clone());
                        next_state = ScramState::SentChallenge {
                            server_nonce: server_nonce,
                            username: username.to_owned(),
                            salted_password: data,
                            initial_client_message: rest,
                            initial_server_message: buf,
                            gs2_header: gs2_header,
                        };
                    }
                    ScramState::SentChallenge {
                        server_nonce: ref server_nonce,
                        username: ref username,
                        salted_password: ref salted_password,
                        gs2_header: ref gs2_header,
                        initial_client_message: ref initial_client_message,
                        initial_server_message: ref initial_server_message,
                    } => {
                        let frame =
                            parse_frame(payload).map_err(|_| "can't decode response".to_owned())?;
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
                        let stored_key = S::hash(&client_key);
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
                        let sent_proof = base64::decode(sent_proof)
                            .map_err(|_| "can't decode proof".to_owned())?;
                        if client_proof != sent_proof {
                            return Err("authentication failed".to_owned());
                        }
                        let server_signature = S::hmac(&auth_message, &server_key);
                        let mut buf = Vec::new();
                        buf.extend(b"v=");
                        buf.extend(base64::encode(&server_signature).bytes());
                        ret = Response::Success(Identity::Username(username.to_owned()), buf);
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
    }

    pub use self::plain::Plain;
    #[cfg(feature = "scram")]
    pub use self::scram::Scram;
}

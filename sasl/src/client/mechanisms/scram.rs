//! Provides the SASL "SCRAM-*" mechanisms and a way to implement more.

use base64;

use client::Mechanism;
use common::scram::{generate_nonce, ScramProvider};
use common::{parse_frame, xor, ChannelBinding, Credentials, Identity, Password, Secret};

use error::Error;

use std::marker::PhantomData;

enum ScramState {
    Init,
    SentInitialMessage {
        initial_message: Vec<u8>,
        gs2_header: Vec<u8>,
    },
    GotServerData {
        server_signature: Vec<u8>,
    },
}

/// A struct for the SASL SCRAM-* and SCRAM-*-PLUS mechanisms.
pub struct Scram<S: ScramProvider> {
    name: String,
    username: String,
    password: Password,
    client_nonce: String,
    state: ScramState,
    channel_binding: ChannelBinding,
    _marker: PhantomData<S>,
}

impl<S: ScramProvider> Scram<S> {
    /// Constructs a new struct for authenticating using the SASL SCRAM-* and SCRAM-*-PLUS
    /// mechanisms, depending on the passed channel binding.
    ///
    /// It is recommended that instead you use a `Credentials` struct and turn it into the
    /// requested mechanism using `from_credentials`.
    pub fn new<N: Into<String>, P: Into<Password>>(
        username: N,
        password: P,
        channel_binding: ChannelBinding,
    ) -> Result<Scram<S>, Error> {
        Ok(Scram {
            name: format!("SCRAM-{}", S::name()),
            username: username.into(),
            password: password.into(),
            client_nonce: generate_nonce()?,
            state: ScramState::Init,
            channel_binding: channel_binding,
            _marker: PhantomData,
        })
    }

    // Used for testing.
    #[doc(hidden)]
    #[cfg(test)]
    pub fn new_with_nonce<N: Into<String>, P: Into<Password>>(
        username: N,
        password: P,
        nonce: String,
    ) -> Scram<S> {
        Scram {
            name: format!("SCRAM-{}", S::name()),
            username: username.into(),
            password: password.into(),
            client_nonce: nonce,
            state: ScramState::Init,
            channel_binding: ChannelBinding::None,
            _marker: PhantomData,
        }
    }
}

impl<S: ScramProvider> Mechanism for Scram<S> {
    fn name(&self) -> &str {
        // TODO: this is quite the workaround…
        &self.name
    }

    fn from_credentials(credentials: Credentials) -> Result<Scram<S>, String> {
        if let Secret::Password(password) = credentials.secret {
            if let Identity::Username(username) = credentials.identity {
                Scram::new(username, password, credentials.channel_binding)
                    .map_err(|_| "can't generate nonce".to_owned())
            } else {
                Err("SCRAM requires a username".to_owned())
            }
        } else {
            Err("SCRAM requires a password".to_owned())
        }
    }

    fn initial(&mut self) -> Result<Vec<u8>, String> {
        let mut gs2_header = Vec::new();
        gs2_header.extend(self.channel_binding.header());
        let mut bare = Vec::new();
        bare.extend(b"n=");
        bare.extend(self.username.bytes());
        bare.extend(b",r=");
        bare.extend(self.client_nonce.bytes());
        let mut data = Vec::new();
        data.extend(&gs2_header);
        data.extend(bare.clone());
        self.state = ScramState::SentInitialMessage {
            initial_message: bare,
            gs2_header: gs2_header,
        };
        Ok(data)
    }

    fn response(&mut self, challenge: &[u8]) -> Result<Vec<u8>, String> {
        let next_state;
        let ret;
        match self.state {
            ScramState::SentInitialMessage {
                ref initial_message,
                ref gs2_header,
            } => {
                let frame =
                    parse_frame(challenge).map_err(|_| "can't decode challenge".to_owned())?;
                let server_nonce = frame.get("r");
                let salt = frame.get("s").and_then(|v| base64::decode(v).ok());
                let iterations = frame.get("i").and_then(|v| v.parse().ok());
                let server_nonce = server_nonce.ok_or_else(|| "no server nonce".to_owned())?;
                let salt = salt.ok_or_else(|| "no server salt".to_owned())?;
                let iterations = iterations.ok_or_else(|| "no server iterations".to_owned())?;
                // TODO: SASLprep
                let mut client_final_message_bare = Vec::new();
                client_final_message_bare.extend(b"c=");
                let mut cb_data: Vec<u8> = Vec::new();
                cb_data.extend(gs2_header);
                cb_data.extend(self.channel_binding.data());
                client_final_message_bare.extend(base64::encode(&cb_data).bytes());
                client_final_message_bare.extend(b",r=");
                client_final_message_bare.extend(server_nonce.bytes());
                let salted_password = S::derive(&self.password, &salt, iterations)?;
                let client_key = S::hmac(b"Client Key", &salted_password);
                let server_key = S::hmac(b"Server Key", &salted_password);
                let mut auth_message = Vec::new();
                auth_message.extend(initial_message);
                auth_message.push(b',');
                auth_message.extend(challenge);
                auth_message.push(b',');
                auth_message.extend(&client_final_message_bare);
                let stored_key = S::hash(&client_key);
                let client_signature = S::hmac(&auth_message, &stored_key);
                let client_proof = xor(&client_key, &client_signature);
                let server_signature = S::hmac(&auth_message, &server_key);
                let mut client_final_message = Vec::new();
                client_final_message.extend(&client_final_message_bare);
                client_final_message.extend(b",p=");
                client_final_message.extend(base64::encode(&client_proof).bytes());
                next_state = ScramState::GotServerData {
                    server_signature: server_signature,
                };
                ret = client_final_message;
            }
            _ => {
                return Err("not in the right state to receive this response".to_owned());
            }
        }
        self.state = next_state;
        Ok(ret)
    }

    fn success(&mut self, data: &[u8]) -> Result<(), String> {
        let frame = parse_frame(data).map_err(|_| "can't decode success response".to_owned())?;
        match self.state {
            ScramState::GotServerData {
                ref server_signature,
            } => {
                if let Some(sig) = frame.get("v").and_then(|v| base64::decode(&v).ok()) {
                    if sig == *server_signature {
                        Ok(())
                    } else {
                        Err("invalid signature in success response".to_owned())
                    }
                } else {
                    Err("no signature in success response".to_owned())
                }
            }
            _ => Err("not in the right state to get a success response".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use client::mechanisms::Scram;
    use client::Mechanism;
    use common::scram::{Sha1, Sha256};

    #[test]
    fn scram_sha1_works() {
        // Source: https://wiki.xmpp.org/web/SASLandSCRAM-SHA-1
        let username = "user";
        let password = "pencil";
        let client_nonce = "fyko+d2lbbFgONRv9qkxdawL";
        let client_init = b"n,,n=user,r=fyko+d2lbbFgONRv9qkxdawL";
        let server_init = b"r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,s=QSXCR+Q6sek8bf92,i=4096";
        let client_final =
            b"c=biws,r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,p=v0X8v3Bz2T0CJGbJQyF0X+HI4Ts=";
        let server_final = b"v=rmF9pqV8S7suAoZWja4dJRkFsKQ=";
        let mut mechanism =
            Scram::<Sha1>::new_with_nonce(username, password, client_nonce.to_owned());
        let init = mechanism.initial().unwrap();
        assert_eq!(
            String::from_utf8(init.clone()).unwrap(),
            String::from_utf8(client_init[..].to_owned()).unwrap()
        ); // depends on ordering…
        let resp = mechanism.response(&server_init[..]).unwrap();
        assert_eq!(
            String::from_utf8(resp.clone()).unwrap(),
            String::from_utf8(client_final[..].to_owned()).unwrap()
        ); // again, depends on ordering…
        mechanism.success(&server_final[..]).unwrap();
    }

    #[test]
    fn scram_sha256_works() {
        // Source: RFC 7677
        let username = "user";
        let password = "pencil";
        let client_nonce = "rOprNGfwEbeRWgbNEkqO";
        let client_init = b"n,,n=user,r=rOprNGfwEbeRWgbNEkqO";
        let server_init = b"r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
        let client_final = b"c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ=";
        let server_final = b"v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=";
        let mut mechanism =
            Scram::<Sha256>::new_with_nonce(username, password, client_nonce.to_owned());
        let init = mechanism.initial().unwrap();
        assert_eq!(
            String::from_utf8(init.clone()).unwrap(),
            String::from_utf8(client_init[..].to_owned()).unwrap()
        ); // depends on ordering…
        let resp = mechanism.response(&server_init[..]).unwrap();
        assert_eq!(
            String::from_utf8(resp.clone()).unwrap(),
            String::from_utf8(client_final[..].to_owned()).unwrap()
        ); // again, depends on ordering…
        mechanism.success(&server_final[..]).unwrap();
    }
}

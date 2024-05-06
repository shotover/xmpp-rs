//! Provides the SASL "SCRAM-*" mechanisms and a way to implement more.

use base64::{engine::general_purpose::STANDARD as Base64, Engine};

use crate::client::{Mechanism, MechanismError};
use crate::common::scram::{generate_nonce, ScramProvider};
use crate::common::{parse_frame, xor, ChannelBinding, Credentials, Identity, Password, Secret};

use crate::error::Error;

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
    name_plus: String,
    username: String,
    password: Password,
    client_first_extensions: String,
    client_final_extensions: String,
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
    ///
    /// `client_first_extensions` and `client_final_extensions` should be empty strings if unused.
    /// Otherwise they should be a comma seperated list of SCRAM extensions to be used e.g. `foo=true,bar=baz`
    pub fn new<N: Into<String>, P: Into<Password>>(
        username: N,
        password: P,
        channel_binding: ChannelBinding,
        client_first_extensions: String,
        client_final_extensions: String,
    ) -> Result<Scram<S>, Error> {
        Ok(Scram {
            name: format!("SCRAM-{}", S::name()),
            name_plus: format!("SCRAM-{}-PLUS", S::name()),
            username: username.into(),
            password: password.into(),
            client_first_extensions,
            client_final_extensions,
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
        client_first_extensions: String,
        client_final_extensions: String,
    ) -> Scram<S> {
        Scram {
            name: format!("SCRAM-{}", S::name()),
            name_plus: format!("SCRAM-{}-PLUS", S::name()),
            username: username.into(),
            password: password.into(),
            client_first_extensions,
            client_final_extensions,
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
        match self.channel_binding {
            ChannelBinding::None | ChannelBinding::Unsupported => &self.name,
            ChannelBinding::TlsUnique(_) | ChannelBinding::TlsExporter(_) => &self.name_plus,
        }
    }

    fn from_credentials(credentials: Credentials) -> Result<Scram<S>, MechanismError> {
        if let Secret::Password(password) = credentials.secret {
            if let Identity::Username(username) = credentials.identity {
                Scram::new(
                    username,
                    password,
                    credentials.channel_binding,
                    String::new(),
                    String::new(),
                )
                .map_err(|_| MechanismError::CannotGenerateNonce)
            } else {
                Err(MechanismError::ScramRequiresUsername)
            }
        } else {
            Err(MechanismError::ScramRequiresPassword)
        }
    }

    fn initial(&mut self) -> Vec<u8> {
        let mut gs2_header = Vec::new();
        gs2_header.extend(self.channel_binding.header());
        let mut bare = Vec::new();
        bare.extend(b"n=");
        bare.extend(self.username.bytes());
        bare.extend(b",r=");
        bare.extend(self.client_nonce.bytes());
        if !self.client_first_extensions.is_empty() {
            bare.extend(b",");
            bare.extend(self.client_first_extensions.bytes());
        }
        let mut data = Vec::new();
        data.extend(&gs2_header);
        data.extend(&bare);
        self.state = ScramState::SentInitialMessage {
            initial_message: bare,
            gs2_header: gs2_header,
        };
        data
    }

    fn response(&mut self, challenge: &[u8]) -> Result<Vec<u8>, MechanismError> {
        let next_state;
        let ret;
        match self.state {
            ScramState::SentInitialMessage {
                ref initial_message,
                ref gs2_header,
            } => {
                let frame =
                    parse_frame(challenge).map_err(|_| MechanismError::CannotDecodeChallenge)?;
                let server_nonce = frame.get("r");
                let salt = frame.get("s").and_then(|v| Base64.decode(v).ok());
                let iterations = frame.get("i").and_then(|v| v.parse().ok());
                let server_nonce = server_nonce.ok_or_else(|| MechanismError::NoServerNonce)?;
                let salt = salt.ok_or_else(|| MechanismError::NoServerSalt)?;
                let iterations = iterations.ok_or_else(|| MechanismError::NoServerIterations)?;
                // TODO: SASLprep
                let mut client_final_message_bare = Vec::new();
                client_final_message_bare.extend(b"c=");
                let mut cb_data: Vec<u8> = Vec::new();
                cb_data.extend(gs2_header);
                cb_data.extend(self.channel_binding.data());
                client_final_message_bare.extend(Base64.encode(&cb_data).bytes());
                client_final_message_bare.extend(b",r=");
                client_final_message_bare.extend(server_nonce.bytes());
                if !self.client_final_extensions.is_empty() {
                    client_final_message_bare.extend(b",");
                    client_final_message_bare.extend(self.client_final_extensions.bytes());
                }
                let salted_password = S::derive(&self.password, &salt, iterations)?;
                let client_key = S::hmac(b"Client Key", &salted_password)?;
                let server_key = S::hmac(b"Server Key", &salted_password)?;
                let mut auth_message = Vec::new();
                auth_message.extend(initial_message);
                auth_message.push(b',');
                auth_message.extend(challenge);
                auth_message.push(b',');
                auth_message.extend(&client_final_message_bare);
                let stored_key = S::hash(&client_key);
                let client_signature = S::hmac(&auth_message, &stored_key)?;
                let client_proof = xor(&client_key, &client_signature);
                let server_signature = S::hmac(&auth_message, &server_key)?;
                let mut client_final_message = Vec::new();
                client_final_message.extend(&client_final_message_bare);
                client_final_message.extend(b",p=");
                client_final_message.extend(Base64.encode(&client_proof).bytes());
                next_state = ScramState::GotServerData {
                    server_signature: server_signature,
                };
                ret = client_final_message;
            }
            _ => {
                return Err(MechanismError::InvalidState);
            }
        }
        self.state = next_state;
        Ok(ret)
    }

    fn success(&mut self, data: &[u8]) -> Result<(), MechanismError> {
        let frame = parse_frame(data).map_err(|_| MechanismError::CannotDecodeSuccessResponse)?;
        match self.state {
            ScramState::GotServerData {
                ref server_signature,
            } => {
                if let Some(sig) = frame.get("v").and_then(|v| Base64.decode(&v).ok()) {
                    if sig == *server_signature {
                        Ok(())
                    } else {
                        Err(MechanismError::InvalidSignatureInSuccessResponse)
                    }
                } else {
                    Err(MechanismError::NoSignatureInSuccessResponse)
                }
            }
            _ => Err(MechanismError::InvalidState),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::mechanisms::Scram;
    use crate::client::Mechanism;
    use crate::common::scram::{Sha1, Sha256};

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
        let mut mechanism = Scram::<Sha1>::new_with_nonce(
            username,
            password,
            client_nonce.to_owned(),
            String::new(),
            String::new(),
        );
        let init = mechanism.initial();
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
        let mut mechanism = Scram::<Sha256>::new_with_nonce(
            username,
            password,
            client_nonce.to_owned(),
            String::new(),
            String::new(),
        );
        let init = mechanism.initial();
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
    fn scram_kafka_token_delegation_works() {
        // credentials and raw messages taken from a real kafka SCRAM token delegation authentication
        let username = "6Lbb79aSTs-mDWUPc64D9Q";
        let password = "O574x+7mB0B8R9Yt8DqwWbIzBgEm3lUE+fy7VWdvCwcLvGvwJK9GM4y0Qaz/MxiIxDHEnxDfSuB13uycXiUqyg==";
        let client_nonce = "o6wj2xqdu0fxe4nmnukkj076m";
        let client_init = b"n,,n=6Lbb79aSTs-mDWUPc64D9Q,r=o6wj2xqdu0fxe4nmnukkj076m,tokenauth=true";
        let server_init = b"r=o6wj2xqdu0fxe4nmnukkj076m1eut816hvmsycqw2qzyn14zxvr,s=MWVtNWw1Mzc1MnFianNoYWhqMjhyYzVzZHM=,i=4096";
        let client_final = b"c=biws,r=o6wj2xqdu0fxe4nmnukkj076m1eut816hvmsycqw2qzyn14zxvr,p=qVfqg28hDgroc6pal4qCF+8hO1/wiB84o7snGRDZKuE=";
        let server_final = b"v=2ZSkAlHEUj6WehcizLhQRiiVGn+VDVtmAqj1v/IPa28=";
        let mut mechanism = Scram::<Sha256>::new_with_nonce(
            username,
            password,
            client_nonce.to_owned(),
            "tokenauth=true".to_owned(),
            String::new(),
        );
        let init = mechanism.initial();
        assert_eq!(
            std::str::from_utf8(&init).unwrap(),
            std::str::from_utf8(client_init).unwrap()
        ); // depends on ordering…
        let resp = mechanism.response(server_init).unwrap();
        assert_eq!(
            std::str::from_utf8(&resp).unwrap(),
            std::str::from_utf8(client_final).unwrap()
        ); // again, depends on ordering…
        mechanism.success(server_final).unwrap();
    }

    #[test]
    fn scram_final_extension_works() {
        let username = "some_user";
        let password = "a_password";
        let client_nonce = "client_nonce";
        let client_init = b"n,,n=some_user,r=client_nonce";
        let server_init =
            b"r=client_nonceserver_nonce,s=MWVtNWw1Mzc1MnFianNoYWhqMjhyYzVzZHM=,i=4096";
        let client_final = b"c=biws,r=client_nonceserver_nonce,foo=true,p=T9XQLmykBv74DzbaCtX90/ElJYJU2XWM/jHmHJ+BI/w=";
        let mut mechanism = Scram::<Sha256>::new_with_nonce(
            username,
            password,
            client_nonce.to_owned(),
            String::new(),
            "foo=true".to_owned(),
        );
        let init = mechanism.initial();
        assert_eq!(
            std::str::from_utf8(&init).unwrap(),
            std::str::from_utf8(client_init).unwrap()
        ); // depends on ordering…
        let resp = mechanism.response(server_init).unwrap();
        assert_eq!(
            std::str::from_utf8(&resp).unwrap(),
            std::str::from_utf8(client_final).unwrap()
        ); // again, depends on ordering…
    }
}

// Copyright (c) 2021 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::data_forms::DataForm;
use crate::date::DateTime;
use crate::iq::{IqGetPayload, IqResultPayload, IqSetPayload};

generate_attribute!(
    /// When sending a push update, the action value indicates if the service is being added or
    /// deleted from the set of known services (or simply being modified).
    Action, "action", {
        /// The service is being added from the set of known services.
        Add => "add",

        /// The service is being removed from the set of known services.
        Remove => "remove",

        /// The service is being modified.
        Modify => "modify",
    }, Default = Add
);

generate_attribute!(
    /// The underlying transport protocol to be used when communicating with the service.
    Transport, "transport", {
        /// Use TCP as a transport protocol.
        Tcp => "tcp",

        /// Use UDP as a transport protocol.
        Udp => "udp",
    }
);

generate_attribute!(
    /// The service type as registered with the XMPP Registrar.
    Type, "type", {
        /// A server that provides Session Traversal Utilities for NAT (STUN).
        Stun => "stun",

        /// A server that provides Traversal Using Relays around NAT (TURN).
        Turn => "turn",
    }
);

generate_attribute!(
    /// Username and password credentials are required and will need to be requested if not already
    /// provided.
    Restricted,
    "restricted",
    bool
);

generate_element!(
    /// Structure representing a `<service xmlns='urn:xmpp:extdisco:2'/>` element.
    Service, "service", EXT_DISCO,
    attributes: [
        /// When sending a push update, the action value indicates if the service is being added or
        /// deleted from the set of known services (or simply being modified).
        action: Default<Action> = "action",

        /// A timestamp indicating when the provided username and password credentials will expire.
        expires: Option<DateTime> = "expires",

        /// Either a fully qualified domain name (FQDN) or an IP address (IPv4 or IPv6).
        host: Required<String> = "host",

        /// A friendly (human-readable) name or label for the service.
        name: Option<String> = "name",

        /// A service- or server-generated password for use at the service.
        password: Option<String> = "password",

        /// The communications port to be used at the host.
        port: Option<u16> = "port",

        /// A boolean value indicating that username and password credentials are required and will
        /// need to be requested if not already provided.
        restricted: Default<Restricted> = "restricted",

        /// The underlying transport protocol to be used when communicating with the service (typically
        /// either TCP or UDP).
        transport: Option<Transport> = "transport",

        /// The service type as registered with the XMPP Registrar.
        type_: Required<Type> = "type",

        /// A service- or server-generated username for use at the service.
        username: Option<String> = "username",
    ], children: [
        /// Extended information
        ext_info: Vec<DataForm> = ("x", DATA_FORMS) => DataForm
    ]
);

impl IqGetPayload for Service {}

generate_element!(
    /// Structure representing a `<services xmlns='urn:xmpp:extdisco:2'/>` element.
    ServicesQuery, "services", EXT_DISCO,
    attributes: [
        /// TODO
        type_: Option<Type> = "type",
    ]
);

impl IqGetPayload for ServicesQuery {}

generate_element!(
    /// Structure representing a `<services xmlns='urn:xmpp:extdisco:2'/>` element.
    ServicesResult, "services", EXT_DISCO,
    attributes: [
        /// TODO
        type_: Option<Type> = "type",
    ],
    children: [
        /// List of services.
        services: Vec<Service> = ("service", EXT_DISCO) => Service
    ]
);

impl IqResultPayload for ServicesResult {}
impl IqSetPayload for ServicesResult {}

generate_element!(
    /// Structure representing a `<credentials xmlns='urn:xmpp:extdisco:2'/>` element.
    Credentials, "credentials", EXT_DISCO,
    children: [
        /// List of services.
        services: Vec<Service> = ("service", EXT_DISCO) => Service
    ]
);

impl IqGetPayload for Credentials {}
impl IqResultPayload for Credentials {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ns;
    use crate::Element;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Action, 1);
        assert_size!(Transport, 1);
        assert_size!(Restricted, 1);
        assert_size!(Type, 1);
        assert_size!(Service, 84);
        assert_size!(ServicesQuery, 1);
        assert_size!(ServicesResult, 16);
        assert_size!(Credentials, 12);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Action, 1);
        assert_size!(Transport, 1);
        assert_size!(Restricted, 1);
        assert_size!(Type, 1);
        assert_size!(Service, 144);
        assert_size!(ServicesQuery, 1);
        assert_size!(ServicesResult, 32);
        assert_size!(Credentials, 24);
    }

    #[test]
    fn test_simple() {
        let elem: Element = "<service xmlns='urn:xmpp:extdisco:2' host='stun.shakespeare.lit' port='9998' transport='udp' type='stun'/>".parse().unwrap();
        let service = Service::try_from(elem).unwrap();
        assert_eq!(service.action, Action::Add);
        assert!(service.expires.is_none());
        assert_eq!(service.host, "stun.shakespeare.lit");
        assert!(service.name.is_none());
        assert!(service.password.is_none());
        assert_eq!(service.port.unwrap(), 9998);
        assert_eq!(service.restricted, Restricted::False);
        assert_eq!(service.transport.unwrap(), Transport::Udp);
        assert_eq!(service.type_, Type::Stun);
        assert!(service.username.is_none());
        assert!(service.ext_info.is_empty());
    }

    #[test]
    fn test_service_query() {
        let query = ServicesQuery { type_: None };
        let elem = Element::from(query);
        assert!(elem.is("services", ns::EXT_DISCO));
        assert_eq!(elem.attrs().next(), None);
        assert_eq!(elem.nodes().next(), None);
    }

    #[test]
    fn test_service_result() {
        let elem: Element = "<services xmlns='urn:xmpp:extdisco:2' type='stun'><service host='stun.shakespeare.lit' port='9998' transport='udp' type='stun'/></services>".parse().unwrap();
        let services = ServicesResult::try_from(elem).unwrap();
        assert_eq!(services.type_, Some(Type::Stun));
        assert_eq!(services.services.len(), 1);
    }
}

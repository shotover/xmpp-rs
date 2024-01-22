xmpp-rs
=======

What's this?
------------

A very much WIP set of rust XMPP library with the goals of being type-safe and
well-tested.

- `xmpp-rs` provides a high-level API for the protocol. You shouldn't need to
  know about the protocol to use it.
- `tokio-xmpp` is a lower-level library that handles the XMPP stream(s).
- `xmpp-parsers` parses XML into Rust and back.
- `minidom` is a DOM library quite specific for XMPP
- `jid` is a Jid parsing library.

Contact
-------

There is an XMPP MUC for the discussion of this library, feel free to join! :)

[chat@xmpp.rs](xmpp:chat@xmpp.rs?join)

Community
---------

A Code of Conduct is available as
[CODE\_OF\_CONDUCT.md](https://gitlab.com/xmpp-rs/xmpp-rs/-/raw/main/CODE_OF_CONDUCT.md)
in the repository for the well-being of the community. Please refer to it in
case of a possible conflict in any of the xmpp-rs venues (channel, forge,
etc.).

License
-------

Mozilla Public License 2 (MPL2). See the LICENSE file.

Building
--------

Dependencies should be provided by crates if you use the default features. If
you use tokio-xmpp's `tls-native` feature you will need an ssl library
(openssl, libressl, etc.).

```
cargo build
```

The various features available should be explained in the crates themselves.

Contributing
------------

Thank you for your interest in the project!

We recommend you come and talk to us in the channel and/or open an issue
before you start working on a feature, to see if it aligns with our goals.

The library is still in development, and while this is the case we adopt a
fail-fast strategy, which is also a reason for the choice of the language.

The earlier we catch bugs and fix them, the less chances they have to confuse
users of our library, or make end-users give up on software developed using
this library. This also helps improving other software in the ecosystem.

Code changes should try to include documentation as possible. They should also
include tests where appropriate, and pass the existing test suite.

CI should pass to submit your changes. This is done by ensuring `cargo fmt`
and `cargo test` pass (in the workspace). Please do not run `cargo fmt` as a
separate commit but do it as part of each of your commits.

More thorough tests can be done locally with `act` or `forgejo-runner
exec`, which is also what is run in the CI. We require docker to be setup for
this to work.

Merge requests can contain as many commits as necessary, but commits should be
kept rather small and meaningful (not include too many different things).

Do not forget to update changelogs and other crate metadata where necessary.

Signing commits (`git commit -S`) and adding DCO bits (`git commit -s`) are
welcome but not mandatory.

We'll do our best to review and discuss changes with you but we're also humans
with other activities, please be patient with us.

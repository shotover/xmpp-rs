# Contributing guidelines

Thank you (again) for your interest in the project!

## Talk to us before commiting to a change

We recommend you come and talk to us in the channel and/or open an issue
before you start working on a feature, to **see if it aligns with our goals**.

We'll do our best to review and discuss changes with you but we're also humans
with other activities, please be patient with us too.

## General guidelines

The library is still **in development**, and while this is the case we adopt a
**fail-fast** strategy, which is also a reason for the choice of the language.

The earlier we catch bugs and fix them, the less chances they have to confuse
users of our library, or make end-users give up on software developed using
this library. This also helps improving other software in the ecosystem.

## Keep commits short and meaningful

To help with reviews, to facilitate reverts, reading and grepping through
commit history, bisects: it is important for us that changes come in small and
meaningful commits.

That is, **don't bundle all the things in a single commit**, if a change can
be added via different commits standalone and still make sense, then it's
probably a good candidate for splitting.

## Meta

Do not forget to **update changelogs** and other crate metadata where necessary.

Signing commits (`git commit -S`) and adding DCO bits (`git commit -s`) are
welcome but not mandatory.

## Ensure CI passes

Code changes should **include documentation**. They should also **include
tests** where appropriate, and pass the existing test suite.

CI should pass to submit your changes. This is done by ensuring `cargo
fmt` and `cargo test` pass (in the workspace). Please run `cargo fmt` as part
of each of your commits.

More thorough tests can be done locally with `act` or `forgejo-runner
exec`, which is also what is run in the CI. We require docker to be setup for
this to work.

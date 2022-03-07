Version 0.9.3, release 2022-03-07:
  * Updates
    - Bumped minidom to 0.14

Version 0.9.2, release 2021-01-13:
  * Updates
    - Bumped minidom to 0.13

Version 0.9.1, release 2021-01-13:
  * Updates
    - Added serde support behind "serde" feature
    - Added equality operators between Jid, BareJid and FullJid.

Version 0.9.0, release 2020-02-15:
  * Breaking
    - Update minidom dependency to 0.12

Version 0.8, released 2019-10-15:
  * Updates
    - CI: Split jobs, add tests, and caching
  * Breaking
    - 0.7.1 was actually a breaking release

Version 0.7.2, released 2019-09-13:
  * Updates
    - Impl Error for JidParseError again, it got removed due to the failure removal but is still wanted.

Version 0.7.1, released 2019-09-06:
  * Updates
    - Remove failure dependency, to keep compilation times in check
    - Impl Display for Jid

Version 0.7.0, released 2019-07-26:
  * Breaking
    - Update minidom dependency to 0.11

Version 0.6.2, released 2019-07-20:
  * Updates
    - Implement From<BareJid> and From<FullJid> for Jid
    - Add node and domain getters on Jid

Version 0.6.1, released 2019-06-10:
  * Updates
    - Change the license from LGPLv3 to MPL-2.0.

Version 0.6.0, released 2019-06-10:
  * Updates
    - Jid is now an enum, with two variants, Bare(BareJid) and Full(FullJid).
    - BareJid and FullJid are two specialised variants of a JID.

Version 0.5.3, released 2019-01-16:
  * Updates
    - Link Mauve bumped the minidom dependency version.
    - Use Edition 2018, putting the baseline rustc version to 1.31.
    - Run cargo-fmt on the code, to lower the barrier of entry.

Version 0.5.2, released 2018-07-31:
  * Updates
    - Astro bumped the minidom dependency version.
    - Updated the changelog to reflect that 0.5.1 was never actually released.

Version 0.5.1, "released" 2018-03-01:
  * Updates
    - Link Mauve implemented failure::Fail on JidParseError.
    - Link Mauve simplified the code a bit.

Version 0.5.0, released 2018-02-18:
  * Updates
    - Link Mauve has updated the optional `minidom` dependency.
    - Link Mauve has added tests for invalid JIDs, which adds more error cases.

Version 0.4.0, released 2017-12-27:
  * Updates
    - Maxime Buquet has updated the optional `minidom` dependency.
    - The repository has been transferred to xmpp-rs/jid-rs.

Version 0.3.1, released 2017-10-31:
  * Additions
    - Link Mauve added a minidom::IntoElements implementation on Jid behind the "minidom" feature. ( https://gitlab.com/lumi/jid-rs/merge_requests/9 )

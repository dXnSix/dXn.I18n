# MessageFormat 2 conformance tests

The JSON files here are the Unicode MessageFormat Working Group's test suite, copied unchanged from
[unicode-org/message-format-wg](https://github.com/unicode-org/message-format-wg) `test/tests/` at
commit `16372a6a02f41728be472a5921ba7c91774edeac`, under the Unicode License ([LICENSE](LICENSE)).
`tests/conformance.rs` runs them; the `:test:*` functions it registers follow the suite's own
`test/README.md`. To update, copy the newer files over these, bump the commit above, and rerun
`cargo test -p dxn_i18n --test conformance`.

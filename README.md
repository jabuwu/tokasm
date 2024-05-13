# tokasm

Emulates the browser's event loop natively with Tokio and provides helpful async utilities that work in both environments.

## Differences from Tokio

Tokio doesn't work in WASM, and likely never will, at least not without some major compromises. The reason for this is that Tokio is built on certain assumptions that aren't true in a web browser. This crate makes a few changes to accomodate web browsers:

- Only one runtime is active at a time, like the browser's single event loop. (You can create more using Tokio directly, but the `tokasm` API only uses one).
- Certain blocking APIs are not possible (for example, `RwLock::blocking_write`). A browser tab cannot block.
- A browser tab continues running so long as the user has it open. When running natively, tasks are counted, and the process can be stalled with `tokasm::wait_until_finished` to avoid the process terminating too early (this is added automatically with the `tokasm::main` macro).

## License

Due to the similarities with Tokio, this crate uses the same MIT license as Tokio.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, shall be licensed as MIT, without any additional terms or conditions.

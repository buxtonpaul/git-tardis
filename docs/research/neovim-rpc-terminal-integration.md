# Research: Neovim RPC API for Terminal Integration

This research document details the technical solution for establishing a bidirectional RPC communication loop between a terminal subprocess (written in Rust) and its parent/host Neovim instance.

## Executive Summary

When launching a Terminal User Interface (TUI) or CLI subprocess from within Neovim (e.g., via `:terminal` or `termopen()`), the subprocess often needs to interact with the host Neovim editor (for example, to open floating windows, request user inputs, or sync states) and pause/resume its own rendering accordingly.

This document details:
1. How the subprocess retrieves the parent Neovim's Unix domain socket or named pipe path using the `$NVIM` environment variable.
2. How to establish an asynchronous MessagePack-RPC connection using the `nvim-rs` crate and `tokio`.
3. How to command Neovim to create a scratch buffer and render a floating window centered on screen.
4. How to configure Neovim autocommands (`BufWipeout`) to notify the subprocess when the floating window is closed, allowing the TUI to resume execution cleanly.

---

## 1. Retrieving the Parent Neovim Socket Path

When Neovim launches a terminal emulator buffer (via `:terminal` or Lua's `vim.fn.termopen()`), it automatically sets environment variables in the subprocess environment:

* **`$NVIM`** (Neovim 0.5+): Contains the absolute path of the Unix domain socket (on macOS and Linux) or the named pipe path (on Windows) of the host Neovim instance. This is the modern, canonical environment variable.
* **`$NVIM_LISTEN_ADDRESS`** (Legacy): Used in older versions of Neovim, but has been deprecated in favor of `--listen` and `$NVIM` (see [Neovim PR #11009](https://github.com/neovim/neovim/pull/11009)).

### Rust Implementation:
```rust
use std::env;

let socket_path = env::var("NVIM").map_err(|_| {
    "This program must be run from inside a Neovim terminal (:terminal) so that the NVIM env var is set."
})?;
```

---

## 2. Connecting to Neovim via RPC in Rust

The most active, mature, and idiomatic library for writing Neovim RPC clients in Rust is **`nvim-rs`** (re-exported by various downstream projects). It supports `tokio` or `async-std` runtimes.

Using `tokio`, we connect to Neovim's socket using `nvim_rs::create::tokio::new_path` which wraps a standard `tokio::net::UnixStream` (on Unix) or `tokio::net::windows::named_pipe::NamedPipeClient` (on Windows).

### Defining the Handler:
To handle notifications (e.g., our custom close signal) and requests initiated by Neovim, we implement the `Handler` trait. Since `nvim-rs` relies on async trait methods, we use the `#[async_trait]` macro:

```rust
use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, Handler, Neovim};
use rmpv::Value;
use tokio::io::WriteHalf;
use tokio::net::UnixStream;
use tokio::sync::mpsc;

type Writer = Compat<WriteHalf<UnixStream>>;

#[derive(Clone)]
struct NeovimHandler {
    tx: mpsc::UnboundedSender<String>,
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Writer;

    async fn handle_notify(&self, name: String, args: Vec<Value>, _neovim: Neovim<Self::Writer>) {
        // Handle incoming notifications from Neovim
        let _ = self.tx.send(name);
    }

    async fn handle_request(&self, _name: String, _args: Vec<Value>, _neovim: Neovim<Self::Writer>) -> Result<Value, Value> {
        Ok(Value::Nil)
    }
}
```

---

## 3. Creating Buffers and Opening Floating Windows

Neovim's RPC API mirrors its Lua API. To show a floating window, we must:
1. Create a scratch buffer (`nvim_create_buf`).
2. Populate the buffer's contents (`nvim_buf_set_lines`).
3. Open a floating window referencing the buffer and specifying configuration parameters (`nvim_open_win`).

### Neovim API Signatures:
* **`nvim_create_buf(listed, scratch)`**: Creates a scratch buffer.
  * `listed: false` (does not show in the buffer list).
  * `scratch: true` (throwaway buffer, sets `'buftype'` to `'nofile'`).
* **`nvim_open_win(buffer, enter, config)`**: Opens a window.
  * `buffer`: The buffer handle (as an Ext type).
  * `enter: true` (focuses the newly opened window).
  * `config`: A map/dictionary defining size, position, border style, and anchor relative to the editor.

### Rust Implementation:
```rust
// 1. Create scratch buffer
let buf = neovim.create_buf(false, true).await?;

// 2. Set 'bufhidden' to 'wipe' so the buffer is destroyed when its window closes
buf.set_option("bufhidden", Value::from("wipe")).await?;

// 3. Write text lines into the buffer
buf.set_lines(0, -1, false, vec![
    "Hello from Rust TUI subprocess!".to_string(),
    "Press :q to close this window and resume.".to_string(),
]).await?;

// 4. Open floating window
let config = vec![
    (Value::from("relative"), Value::from("editor")),
    (Value::from("width"), Value::from(40)),
    (Value::from("height"), Value::from(6)),
    (Value::from("row"), Value::from(5)),
    (Value::from("col"), Value::from(20)),
    (Value::from("style"), Value::from("minimal")),
    (Value::from("border"), Value::from("rounded")),
];
let win = neovim.open_win(&buf, true, config).await?;
```

---

## 4. Signal and Event Notification Hook

A major design challenge is pausing the Rust TUI process while Neovim displays the floating window, and resuming once the window is closed. To achieve this asynchronously:

1. **Get the Channel ID**: The Rust process queries its own RPC connection channel ID using `nvim_get_api_info()`.
2. **Register an Autocommand**: The Rust process registers a transient autocommand (`nvim_create_autocmd`) for the **`BufWipeout`** event, restricted precisely to our newly created scratch buffer (`opts.buffer = buf`).
3. **Execute `rpcnotify`**: When the buffer is wiped out (which happens automatically when the floating window is closed because `'bufhidden'` is `'wipe'`), Neovim executes `rpcnotify(<channel_id>, 'float_closed')`.
4. **Block & Resume**: The Rust process blocks on a receiver channel (`rx.recv().await`) until it receives the `'float_closed'` notification, after which it resumes TUI operations cleanly.

### Setup Autocommand in Rust:
```rust
// Get our channel ID
let api_info = neovim.get_api_info().await?;
let channel_id = api_info[0].as_i64().ok_or("Failed to get channel ID")?;

// Define BufWipeout autocommand scoped only to our buffer
let opts = vec![
    (Value::from("buffer"), buf.get_value().clone()),
    (Value::from("command"), Value::from(format!("call rpcnotify({}, 'float_closed')", channel_id))),
    (Value::from("once"), Value::from(true)),
];

neovim.create_autocmd(Value::from("BufWipeout"), opts).await?;
```

---

## 5. Verified Proof of Concept (PoC)

A complete, self-contained, and fully tested Proof of Concept is available in this repository under:
`research/neovim-rpc-poc/`

### Running the PoC:
To compile and test the PoC under real conditions:

1. Build the binary:
   ```bash
   cd research/neovim-rpc-poc
   cargo build
   ```
2. Run Neovim and spawn the binary in a terminal pane:
   ```bash
   nvim -c "terminal ./target/debug/neovim-rpc-poc"
   ```
3. A rounded, centered floating window will programmatically open on top of your Neovim session.
4. Press `:q` (or close the floating window/buffer).
5. The Rust subprocess receives the `float_closed` RPC signal, outputs its confirmation log, and exits cleanly back to your terminal prompt.

### Execution Log Evidence:
A successful integration run produced the following logs:
```
[2026-07-21 17:05:47.060] Subprocess starting...
[2026-07-21 17:05:47.061] Connecting to host Neovim at socket: /var/folders/.../nvim.83939.0
[2026-07-21 17:05:47.070] Connected! RPC channel ID is: 4
[2026-07-21 17:05:47.070] Created scratch buffer: Ext(0, [2])
[2026-07-21 17:05:47.071] Set bufhidden to wipe
[2026-07-21 17:05:47.071] Set buffer lines
[2026-07-21 17:05:47.071] Opened floating window: Ext(1, [205, 3, 234])
[2026-07-21 17:05:47.071] Registered BufWipeout autocommand
[2026-07-21 17:05:47.071] Waiting for 'float_closed' notification from Neovim...
[2026-07-21 17:05:48.393] Received RPC notification: float_closed with args: []
[2026-07-21 17:05:48.393] Received 'float_closed' notification from Neovim!
[2026-07-21 17:05:48.393] Successfully validated RPC notification loop!
[2026-07-21 17:05:48.393] Resuming execution. Exiting now.
```

---

## Citations & Sources
* **Neovim Remote Documentation (`remote.txt`)**: [Neovim.io Doc](https://neovim.io/doc/user/remote/)
* **`nvim-rs` crate by KillTheMule**: [Crates.io / Docs.rs](https://docs.rs/nvim-rs/latest/nvim_rs/)
* **Neovim API specifications (`api.txt`)**: [Neovim.io API](https://neovim.io/doc/user/api/)
* **Neovim Autocommands documentation (`autocmd.txt`)**: [Neovim.io Autocmd](https://neovim.io/doc/user/autocmd/)

use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, create::tokio as create, Handler, Neovim};
use rmpv::Value;
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::io::WriteHalf;
use tokio::net::UnixStream;
use tokio::sync::mpsc;

type Writer = Compat<WriteHalf<UnixStream>>;

// Thread-safe logger to a file
#[derive(Clone)]
struct FileLogger {
    file: Arc<Mutex<std::fs::File>>,
}

impl FileLogger {
    fn new(path: &str) -> Self {
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        Self {
            file: Arc::new(Mutex::new(f)),
        }
    }

    fn log(&self, msg: &str) {
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), msg);
            let _ = f.flush();
        }
    }
}

#[derive(Clone)]
struct NeovimHandler {
    tx: mpsc::UnboundedSender<String>,
    logger: FileLogger,
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Writer;

    async fn handle_notify(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Self::Writer>,
    ) {
        self.logger.log(&format!("Received RPC notification: {} with args: {:?}", name, args));
        let _ = self.tx.send(name);
    }

    async fn handle_request(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Self::Writer>,
    ) -> Result<Value, Value> {
        self.logger.log(&format!("Received RPC request: {} with args: {:?}", name, args));
        Ok(Value::Nil)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let logger = FileLogger::new("research-poc.log");
    logger.log("Subprocess starting...");

    // 1. Get host Neovim socket path from NVIM environment variable
    let socket_path = match env::var("NVIM") {
        Ok(path) => path,
        Err(_) => {
            let err_msg = "This program must be run from inside a Neovim terminal (:terminal) so that the NVIM env var is set.";
            logger.log(err_msg);
            return Err(err_msg.into());
        }
    };

    logger.log(&format!("Connecting to host Neovim at socket: {}", socket_path));

    // 2. Setup RPC connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handler = NeovimHandler { tx, logger: logger.clone() };
    
    let (neovim, io_handler) = create::new_path(&socket_path, handler).await?;
    
    // Spawn the IO loop in the background
    let logger_clone = logger.clone();
    let io_join = tokio::spawn(async move {
        match io_handler.await {
            Err(e) => logger_clone.log(&format!("IO loop error: {:?}", e)),
            Ok(Err(e)) => logger_clone.log(&format!("IO loop finished with error: {:?}", e)),
            Ok(Ok(())) => logger_clone.log("IO loop finished successfully"),
        }
    });

    // 3. Get the RPC channel ID of our connection
    let api_info = neovim.get_api_info().await?;
    let channel_id = api_info[0]
        .as_i64()
        .ok_or("Failed to parse channel ID from get_api_info")?;
    
    logger.log(&format!("Connected! RPC channel ID is: {}", channel_id));

    // 4. Create a scratch buffer
    let buf = neovim.create_buf(false, true).await?;
    logger.log(&format!("Created scratch buffer: {:?}", buf.get_value()));
    
    // Set bufhidden to wipe so that the buffer is automatically wiped out when the window is closed
    buf.set_option("bufhidden", Value::from("wipe"))
        .await?;
    logger.log("Set bufhidden to wipe");

    // Set some contents in the buffer
    buf.set_lines(
        0,
        -1,
        false,
        vec![
            "Hello from Rust TUI subprocess!".to_string(),
            "Press :q to close this window and resume.".to_string(),
        ],
    )
    .await?;
    logger.log("Set buffer lines");

    // 5. Open a floating window
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
    logger.log(&format!("Opened floating window: {:?}", win.get_value()));

    // 6. Set up autocommand on BufWipeout for this specific buffer
    let opts = vec![
        (Value::from("buffer"), buf.get_value().clone()),
        (Value::from("command"), Value::from(format!("call rpcnotify({}, 'float_closed')", channel_id))),
        (Value::from("once"), Value::from(true)),
    ];
    neovim.create_autocmd(Value::from("BufWipeout"), opts).await?;
    logger.log("Registered BufWipeout autocommand");

    logger.log("Waiting for 'float_closed' notification from Neovim...");

    // 7. Block waiting for the 'float_closed' notification from Neovim
    let mut got_close = false;
    while let Some(msg) = rx.recv().await {
        if msg == "float_closed" {
            logger.log("Received 'float_closed' notification from Neovim!");
            got_close = true;
            break;
        }
    }

    if got_close {
        logger.log("Successfully validated RPC notification loop!");
    } else {
        logger.log("Warning: channel closed without receiving float_closed notification.");
    }

    logger.log("Resuming execution. Exiting now.");

    // Clean up IO task
    io_join.abort();

    Ok(())
}

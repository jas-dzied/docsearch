use std::{path::Path, thread};

use anyhow::Result;

fn main() -> Result<()> {
    let ip = "127.0.0.1:3012";
    let path = Path::new("/home/may/notes/geography").to_owned();

    thread::spawn(|| {
        backend::start_server(ip, path);
    });

    frontend::start_gui(ip)?;
    Ok(())
}

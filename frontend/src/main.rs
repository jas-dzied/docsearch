use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use anyhow::Result;
use eframe::egui;
use tungstenite::connect;
use url::Url;

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "docsearch",
        native_options,
        Box::new(|cc| Box::new(Docsearch::new(cc))),
    )
    .unwrap();
    Ok(())
}

struct Docsearch {
    previous_text: String,
    search_text: String,
    results: Arc<Mutex<Vec<(String, f64)>>>,
    messager: Sender<String>,
}

impl Default for Docsearch {
    fn default() -> Self {
        let results = Mutex::new(vec![]);
        let results_ref = Arc::new(results);
        let results_arc = Arc::clone(&results_ref);
        let (send, recv) = channel();

        thread::spawn(move || {
            let (mut socket, _) = connect(Url::parse("ws://127.0.0.1:3012").unwrap()).unwrap();
            loop {
                let query = recv.recv().unwrap();
                socket
                    .write_message(tungstenite::Message::Text(query))
                    .unwrap();
                let response_text = socket.read_message().unwrap().into_text().unwrap();
                let response: Vec<(String, f64)> = serde_json::from_str(&response_text).unwrap();
                *results_arc.lock().unwrap() = response;
            }
        });

        Self {
            previous_text: String::new(),
            search_text: String::new(),
            results: results_ref,
            messager: send,
        }
    }
}

impl Docsearch {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for Docsearch {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(&mut self.search_text);
            if let Ok(ref mut results) = self.results.try_lock() {
                for result in results.iter() {
                    ui.label(format!("{} (score: {})", &result.0, result.1));
                }
            }
        });
        if self.previous_text != self.search_text {
            self.messager.send(self.search_text.clone()).unwrap();
            self.previous_text = self.search_text.clone();
        }
    }
}

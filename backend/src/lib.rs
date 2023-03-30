use anyhow::{anyhow, Result};
use colored::Colorize;
use std::io::Write;
use std::{
    collections::HashMap,
    env, fs, io,
    net::TcpListener,
    path::{Path, PathBuf},
    thread::spawn,
};
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
};

fn get_all_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut result = vec![];
    for entry in root.as_ref().read_dir()? {
        let item = entry?.path();
        let is_hidden = item
            .file_name()
            .ok_or_else(|| anyhow!("File name not found"))?
            .to_str()
            .ok_or_else(|| anyhow!("Couldn't parse OsStr to &str"))?
            .starts_with(".");

        if item.is_file() && !is_hidden {
            result.push(item);
        } else if !is_hidden {
            let mut subitems = get_all_files(item)?;
            result.append(&mut subitems);
        }
    }
    Ok(result)
}

fn calculate_tf(term: &str, document: &PathBuf) -> Result<f64> {
    let text = fs::read_to_string(document)?.to_lowercase();
    let mut num_words = 0;
    let num_occurences = text
        .split(' ')
        .inspect(|_| num_words += 1)
        .map(|x| x.trim())
        .filter(|x| x == &term)
        .count();
    //Ok(num_occurences as f64 / num_words as f64)
    Ok(num_occurences as f64)
}

fn contains_term(term: &str, document: &PathBuf) -> Result<bool> {
    let text = fs::read_to_string(document)?.to_lowercase();
    Ok(text.contains(term))
}

fn score_documents(terms: &[&str], documents: &[PathBuf]) -> Result<HashMap<PathBuf, f64>> {
    let mut result = HashMap::new();
    let num_documents = documents.len();

    for document in documents {
        let mut total = 0.0;
        for term in terms {
            let term = term.trim().to_lowercase();
            let mut num_containing = 0;
            for document in documents {
                if let Ok(true) = contains_term(&term, document) {
                    num_containing += 1;
                }
            }
            if let Ok(tf) = calculate_tf(&term, &document) {
                let idf = (num_documents as f64 / num_containing as f64).log2();
                total += tf * idf;
            }
        }
        result.insert(document.clone(), total);
    }

    Ok(result)
}

fn get_results(terms: &[&str], folder: impl AsRef<Path>) -> Result<Vec<(String, f64)>> {
    let files = get_all_files(&folder)?;
    let scores = score_documents(terms, &files)?;
    let mut scores = scores
        .iter()
        .map(|x| (x.0.to_str().unwrap().to_string(), *x.1))
        .filter(|x| !x.1.is_nan())
        .collect::<Vec<_>>();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(scores)
}

fn info(description: String) -> Result<()> {
    println!("{} {}", "INFO".yellow().bold(), description);
    io::stdout().flush()?;
    Ok(())
}

macro_rules! info {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        info(text)
    }};
}

pub fn start_server(ip: &str, path: PathBuf) -> Result<()> {
    let server = TcpListener::bind(ip)?;
    info!("Started watching on {}", ip.blue())?;
    for stream in server.incoming() {
        let path_clone = path.clone();
        spawn(move || {
            let callback = |_req: &Request, response: Response| {
                info!("Connected to client").unwrap();
                Ok(response)
            };
            let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();

            loop {
                let msg = websocket.read_message().unwrap();
                let text = msg.to_string();
                info!("Received request {}", text.blue()).unwrap();
                let results =
                    get_results(&text.split(' ').collect::<Vec<_>>(), &path_clone).unwrap();
                info!("Found results:").unwrap();
                for result in &results {
                    println!(
                        "{} (score {})",
                        result.0.green(),
                        result.1.to_string().blue()
                    );
                }
                websocket
                    .write_message(tungstenite::Message::Text(
                        serde_json::to_string(&results).unwrap(),
                    ))
                    .unwrap();
            }
        });
    }
    Ok(())
}

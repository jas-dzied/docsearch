use anyhow::{anyhow, Result};
use colored::Colorize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
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

fn main() -> Result<()> {
    let folder = Path::new("geography_notes");
    let files = get_all_files(folder)?;
    let arg = env::args().nth(1).unwrap();
    let query = arg.split(' ').collect::<Vec<_>>();
    let scores = score_documents(&query, &files)?;
    let mut scores = scores.iter().collect::<Vec<_>>();
    scores.sort_by(|a, b| b.1.total_cmp(a.1));
    for (i, score) in scores.iter().enumerate() {
        println!(
            "{}{} {} {}{}{}",
            (i + 1).to_string().yellow(),
            "|".to_string().yellow(),
            score.0.display().to_string().blue(),
            "(score: ".truecolor(100, 100, 100),
            score.1.to_string().truecolor(100, 100, 100),
            ")".truecolor(100, 100, 100),
        );
    }
    Ok(())
}

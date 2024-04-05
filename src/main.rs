use std::{
  collections::HashMap,
  fs::{self, File, OpenOptions},
  io::{BufRead, BufReader, Write},
  path::Path,
  process::Command,
};

use ordered_float::OrderedFloat;
use serde::Deserialize;

use tldextract::{TldExtractor, TldOption};
use tokio::task::JoinHandle;

use clap::{arg, Parser};

const LINES_PER_CHUNK: usize = 1000;
const FAILURE_LOG_NAME: &str = "failure_log.txt";

const EXTENSIONS: [&str; 2] =
  ["uBlock0.chromium", "bypass-paywalls-chrome-clean-v3.6.1.0"];

enum ScrapeError {
  NoPagesJson(usize),
  JsonParse(usize),
}

impl ScrapeError {
  fn description(&self) -> String {
    match self {
      Self::NoPagesJson(index) => {
        format!("No pages.jsonl file found for url on line {}", index)
      }
      ScrapeError::JsonParse(index) => {
        format!("Failed to parse json document on line {}", index)
      }
    }
  }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ScrapeOptions {
  #[arg(long, default_value_t = 1)]
  workers: u16,
  #[arg(long, default_value_t = false)]
  descend_urls: bool,
  #[arg(long, default_value = "urls.txt")]
  url_file: String,
}

macro_rules! panic_with_stderr {
  ($output:ident, $process_name:literal) => {
    panic!(
      "{} process returned stderr:\n{}",
      $process_name,
      String::from_utf8($output.stderr).expect(&format!(
        "failed to interpret stderr of {} command as String",
        $process_name
      ))
    )
  };
}

#[derive(Deserialize, Debug)]
struct ParsedDocument {
  url: String,
  text: String,
  status: i32,
}

fn scrape_url(url: &str, folder_name: &str, options: &ScrapeOptions) {
  let mut browsertrix_command = Command::new("docker");
  browsertrix_command
    .arg("run")
    .args([
      "-e",
      &format!(
        "CHROME_FLAGS=\"--disable-extensions-except={}\"",
        &EXTENSIONS
          .iter()
          .map(|filename: &&str| { format!("/ext/{}/", filename) })
          .reduce(|a, b| a + "," + &b)
          .unwrap_or("".to_string()),
      ),
    ])
    .args(["-v", "./crawls:/crawls/"])
    .args(["-v", "./chrome_plugins/:/ext/"])
    .args(["-v", "./chrome_profile/:/chrome_profile/"])
    .args(["-d", "webrecorder/browsertrix-crawler"])
    .arg("crawl")
    .args(["--profile", "\"/chrome_profile/profile.tar.gz\""]);
  if !options.descend_urls {
    browsertrix_command.args(["--pageLimit", "1"]);
  }
  browsertrix_command
    .args(["--workers", options.workers.to_string().as_ref()])
    .args(["--url", url])
    .arg("--text")
    .args(["--collection", folder_name]);
  let browsertrix_output = browsertrix_command
    .output()
    .expect("failed to execte browsertrix through docker");
  let browsertrix_stdout = match browsertrix_output.status.code() {
    Some(exit_code) => {
      if exit_code == 0 {
        if !browsertrix_output.stderr.is_empty() {
          panic_with_stderr!(browsertrix_output, "browsertrix");
        }
        String::from_utf8(browsertrix_output.stdout)
          .expect("failed to interpret stdout of browsertrix command as String")
      } else {
        println!(
          "Recieved exit code {} from browsertrix docker process",
          exit_code
        );
        panic_with_stderr!(browsertrix_output, "browsertrix");
      }
    }
    None => {
      panic!("Failed to get exit status code for browsertrix docker process")
    }
  };
  let docker_wait_output = Command::new("docker")
    .arg("wait")
    .arg(browsertrix_stdout.trim())
    .output()
    .expect("failed to execute docker wait for browsertrix container");
  match docker_wait_output.status.code() {
    Some(exit_code) => {
      if exit_code == 0 {
        if !docker_wait_output.stderr.is_empty() {
          panic_with_stderr!(docker_wait_output, "docker wait");
        }
        /*String::from_utf8(docker_wait_output.stdout)
        .expect("failed to interpret stdout of docket wait command as String")*/
      } else {
        println!("Recieved exit code {} from docker wait process", exit_code);
        panic_with_stderr!(docker_wait_output, "docker wait");
      }
    }
    None => {
      panic!("Failed to get exit status code for docker wait process");
    }
  }
}

fn gather_documents_from_crawl(
  index: usize,
) -> Result<Vec<ParsedDocument>, ScrapeError> {
  let file_contents = fs::read_to_string(format!(
    "./crawls/collections/{}/pages/pages.jsonl",
    index.to_string()
  ))
  .map_err(|_err| ScrapeError::NoPagesJson(index))?;
  let mut lines = file_contents.lines().into_iter();
  lines.next();
  let mut line_strings = lines
    .map(|line| String::from(line))
    .collect::<Vec<String>>();
  let documents: Result<Vec<ParsedDocument>, ScrapeError> = line_strings
    .iter_mut()
    .filter_map(|line| {
      if line.is_empty() {
        None
      } else {
        Some(
          unsafe { simd_json::from_str(line) }
            .map_err(|_err| ScrapeError::JsonParse(index)),
        )
      }
    })
    .collect();
  documents
}

fn ensure_directory_exists(path: &str) {
  match std::fs::create_dir_all(path) {
    Ok(_) => {}
    Err(err) => match err.kind() {
      std::io::ErrorKind::AlreadyExists => {}
      _ => panic!("{:?}", err),
    },
  }
}

fn save_documents(
  folder_name: &str,
  documents: Vec<ParsedDocument>,
) -> std::io::Result<()> {
  ensure_directory_exists(&format!("./dataset/{}/", folder_name));
  for (index, document) in documents.into_iter().enumerate() {
    let mut file = std::fs::File::create(format!(
      "./dataset/{}/{}.txt",
      folder_name, index
    ))?;
    file.write_all(document.text.as_bytes())?;
  }
  Ok(())
}

fn reorder_urls(urls: Vec<String>) -> Vec<String> {
  let extractor = TldExtractor::new(TldOption::default());
  let domain_url_pairs: Vec<(String, String)> = urls
    .into_iter()
    .filter_map(|url| {
      if url.is_empty() || url.starts_with("/") {
        None
      } else {
        match extractor.extract(&url) {
          Ok(extracted) => match extracted.domain {
            Some(domain) => Some((domain, url)),
            None => None,
          },
          Err(_) => None,
        }
      }
    })
    .collect();
  let mut domain_buckets: HashMap<String, Vec<String>> = HashMap::new();
  for (domain, url) in domain_url_pairs {
    match domain_buckets.get_mut(&domain) {
      Some(bucket) => bucket.push(url),
      None => {
        domain_buckets.insert(domain, vec![url]);
      }
    }
  }
  let solo_bucket_count = domain_buckets
    .values()
    .filter(|bucket| bucket.len() == 1)
    .count();
  let mut solo_bucket_index = 0;
  let mut precedences_and_domains = domain_buckets
    .values()
    .enumerate()
    .map(|(bucket_index, bucket)| {
      let bucket_size = bucket.len();
      bucket
        .into_iter()
        .enumerate()
        .map(|(url_index, url)| {
          (
            (bucket_index as f32) * 0.0001
              + if bucket_size == 1 {
                let precedence =
                  solo_bucket_index as f32 / (solo_bucket_count - 1) as f32;
                solo_bucket_index += 1;
                0.01 + 0.98 * precedence
              } else {
                url_index as f32 / (bucket_size - 1) as f32
              },
            url.to_string(),
          )
        })
        .collect::<Vec<(f32, String)>>()
    })
    .flatten()
    .collect::<Vec<_>>();
  precedences_and_domains
    .sort_by_key(|(precedence, _)| OrderedFloat(*precedence));
  precedences_and_domains
    .into_iter()
    .map(|(_, url)| url)
    .collect()
}

async fn scrape_urls(
  index_offset: usize,
  urls: Vec<String>,
  options: &ScrapeOptions,
) {
  let mut document_processing_join_handles: Vec<JoinHandle<()>> = vec![];

  for (index, url) in urls.into_iter().enumerate() {
    println!("{}: {}", index_offset + index, url);
    let folder_name = format!("{}", index);
    scrape_url(&url, &folder_name, &options);
    document_processing_join_handles.push(tokio::spawn(async move {
      match gather_documents_from_crawl(index) {
        Ok(documents) => {
          save_documents(&folder_name, documents)
            .expect("Failed to save documents");
        }
        Err(scrape_error) => {
          println!("failed!");
          let mut log_file = OpenOptions::new()
            .create_new(!Path::new(FAILURE_LOG_NAME).exists())
            .write(true)
            .append(true)
            .open(FAILURE_LOG_NAME)
            .unwrap();
          writeln!(log_file, "{}", scrape_error.description())
            .expect("Couldn't open error_log.txt");
        }
      }
    }));
  }

  for handle in document_processing_join_handles {
    handle
      .await
      .expect("Failed to join document processing handle");
  }
}

#[tokio::main]
async fn main() {
  let options = ScrapeOptions::parse();
  let mut url_lines = BufReader::new(
    File::open(options.url_file.clone())
      .expect(&format!("Failed to load file '{}'", options.url_file)),
  )
  .lines()
  .peekable();
  let mut lines_read = 0;
  while url_lines.by_ref().peek().is_some() {
    let line_chunk: Vec<_> = url_lines
      .by_ref()
      .take(LINES_PER_CHUNK)
      .map(|maybe_line| maybe_line.expect("Failed to get line of url file"))
      .collect();
    let line_count = line_chunk.len();
    /*println!(
      "\n{:?}\n\n{:?}\n\n",
      line_chunk.clone(),
      reorder_urls(line_chunk.clone())
    );*/
    scrape_urls(lines_read, reorder_urls(line_chunk), &options).await;
    lines_read += line_count;
  }
}

use std::{
  collections::HashMap,
  fs::File,
  io::{BufRead, BufReader, Write},
  os::unix::process::CommandExt,
  process::Command,
};

use ordered_float::OrderedFloat;

use tldextract::{TldExtractor, TldOption};
use tokio::task::JoinHandle;

use clap::{arg, Parser};

const LINES_PER_CHUNK: usize = 2;
const TIMEOUT_SECONDS: u64 = 10;

const EXTENSIONS: [&str; 2] =
  ["uBlock0.chromium", "bypass-paywalls-chrome-clean-v3.6.1.0"];

/*enum ScrapeError {
  NoPagesJson(usize),
  ParsePagesJsonLineFailure(usize, usize),
}

impl ScrapeError {
  fn description(&self) -> String {
    match self {
      Self::NoPagesJson(chunk_index) => {
        format!("No pages.jsonl file found for chunk `{}`", chunk_index)
      }
      ScrapeError::ParsePagesJsonLineFailure(chunk_index, sub_index) => {
        format!(
          "Failed to parse json document for chunk {}, url {}",
          chunk_index, sub_index
        )
      }
    }
  }
}*/

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ScrapeOptions {
  #[arg(long, default_value_t = 1)]
  workers: u16,
  #[arg(long, default_value_t = false)]
  descend_urls: bool,
  #[arg(long, default_value = "urls.txt")]
  url_file: String,
  #[arg(long, default_value = None)]
  uid: Option<u32>,
  #[arg(long, default_value = None)]
  chunk: Option<usize>,
  #[arg(long, default_value_t = false)]
  count_documents: bool,
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

/*#[derive(Deserialize, Debug)]
struct ParsedDocument {
  url: String,
  text: Option<String>,
  status: i32,
}*/

fn scrape_url_file(
  url_file_name: &str,
  folder_name: &str,
  options: &ScrapeOptions,
) {
  let mut browsertrix_command = Command::new("docker");
  if let Some(uid) = options.uid {
    browsertrix_command.uid(uid);
  }
  println!("{}", url_file_name);
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
    .args(["-v", "./url_chunks/:/url_chunks/"])
    .args(["-d", "webrecorder/browsertrix-crawler"])
    .arg("crawl")
    .args(["--profile", "\"/chrome_profile/profile.tar.gz\""]);
  if !options.descend_urls {
    browsertrix_command.args(["--depth", "0"]);
  }
  browsertrix_command
    .args(["--workers", options.workers.to_string().as_ref()])
    .args(["--urlFile", url_file_name])
    .args(["--text", "to-pages"])
    .args(["--behaviors", "autoscroll"])
    .args(["--timeout", &format!("{}", TIMEOUT_SECONDS)])
    .args(["--collection", folder_name]);
  let browsertrix_output = browsertrix_command
    .output()
    .expect("failed to execute browsertrix through docker");
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
      panic!(
        "Failed to get exit status code for docker process. \
        Full status:\n\n{:?}",
        browsertrix_output.status
      );
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
      panic!(
        "Failed to get exit status code for docker wait process. \
        Full status:\n\n{:?}",
        docker_wait_output.status
      );
    }
  }
}

/*fn gather_documents_from_crawl(
  chunk_index: usize,
) -> Result<Vec<Result<ParsedDocument, ScrapeError>>, ScrapeError> {
  let file_contents = fs::read_to_string(format!(
    "./crawls/collections/{}/pages/pages.jsonl",
    chunk_index.to_string()
  ))
  .map_err(|_err| ScrapeError::NoPagesJson(chunk_index))?;
  let mut lines = file_contents.lines().into_iter();
  lines.next();
  let mut line_strings = lines
    .map(|line| String::from(line))
    .collect::<Vec<String>>();
  let documents: Vec<Result<ParsedDocument, ScrapeError>> = line_strings
    .iter_mut()
    .enumerate()
    .filter_map(|(i, line)| {
      if line.is_empty() {
        None
      } else {
        let document_hopefuly: Result<ParsedDocument, _> =
          unsafe { simd_json::from_str(line) };
        Some(document_hopefuly.map_err(|_err| {
          ScrapeError::ParsePagesJsonLineFailure(chunk_index, i)
        }))
      }
    })
    .collect();
  Ok(documents)
}*/

fn ensure_directory_exists(path: &str) {
  match std::fs::create_dir_all(path) {
    Ok(_) => {}
    Err(err) => match err.kind() {
      std::io::ErrorKind::AlreadyExists => {}
      _ => panic!("{:?}", err),
    },
  }
}

/*fn save_documents(
  folder_name: &str,
  documents: Vec<Result<ParsedDocument, ScrapeError>>,
) -> std::io::Result<()> {
  ensure_directory_exists(&format!("./dataset/{}/", folder_name));
  for (index, document_or_error) in documents.into_iter().enumerate() {
    if let Ok(document) = document_or_error {
      if let Some(document_text) = document.text {
        let mut file =
          File::create(format!("./dataset/{}/{}.txt", folder_name, index))?;
        file.write_all(document_text.as_bytes())?;
      }
    }
  }
  Ok(())
}*/

fn preprocess_urls(urls: Vec<String>) -> Vec<String> {
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
  chunk_index: usize,
  urls: Vec<String>,
  options: &ScrapeOptions,
) {
  if urls.len() > 0 {
    let mut document_processing_join_handles: Vec<JoinHandle<()>> = vec![];
    let folder_name = format!("{}", chunk_index);
    ensure_directory_exists("./url_chunks/");
    let url_chunk_file_name = format!("./url_chunks/{}.txt", chunk_index);
    let mut temp_url_file = File::create(&url_chunk_file_name).unwrap();
    temp_url_file
      .write_all(
        urls
          .into_iter()
          .reduce(|a, b| a + "\n" + &b)
          .unwrap()
          .as_bytes(),
      )
      .unwrap();
    scrape_url_file(
      &format!("/url_chunks/{}.txt", chunk_index),
      &folder_name,
      &options,
    );
    document_processing_join_handles.push(tokio::spawn(async move {
      /*match gather_documents_from_crawl(chunk_index) {
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
      }*/
    }));

    for handle in document_processing_join_handles {
      handle
        .await
        .expect("Failed to join document processing handle");
    }
  }
}

fn count_documents() {
  let collections_entries = std::fs::read_dir("./crawls/collections/").expect(
    "Failed to open directory /crawls/collections/ while counting documents",
  );
  let mut total_document_count = 0;
  for maybe_collections_entry in collections_entries {
    if let Ok(collections_entry) = maybe_collections_entry {
      let chunk_directory_name = collections_entry.file_name();
      let dir_name = chunk_directory_name.to_str().unwrap();
      match std::fs::read_to_string(format!(
        "crawls/collections/{}/pages/pages.jsonl",
        dir_name
      )) {
        Ok(file_contents) => {
          total_document_count += file_contents.matches("\"text\":").count();
        }
        Err(_) => {
          println!("Failed to read pages.jsonl for chunk {}", dir_name);
        }
      }
    }
  }
  println!("Total document count: {}", total_document_count);
}

async fn scrape(options: ScrapeOptions) {
  let skipped_chunk_count = match options.chunk {
    Some(chunk_arg) => chunk_arg.checked_sub(1).unwrap_or(0),
    None => {
      match std::fs::read_dir("./url_chunks/").map(|url_chunks_entries| {
        url_chunks_entries.fold(
          None,
          |current_highest_chunk_index: Option<usize>, maybe_chunk_entry| {
            if let Ok(chunk_entry) = maybe_chunk_entry {
              let chunk_entry_name_os = chunk_entry.file_name();
              let chunk_entry_name = chunk_entry_name_os.to_str().unwrap();
              let chunk_name = chunk_entry_name.split(".").next().unwrap();
              if let Ok(x) = chunk_name.parse::<usize>() {
                Some(match current_highest_chunk_index {
                  Some(current_highest_chunk_index) => {
                    current_highest_chunk_index.max(x)
                  }
                  None => x,
                })
              } else {
                current_highest_chunk_index
              }
            } else {
              current_highest_chunk_index
            }
          },
        )
      }) {
        Ok(Some(highest_found_chunk_index)) => {
          println!(
            "Automatically starting from chunk {}, the highest index found in \
            the url_chunks directory",
            highest_found_chunk_index
          );
          highest_found_chunk_index.checked_sub(1).unwrap_or(0)
        }
        _ => 0,
      }
    }
  };
  let mut url_lines = BufReader::new(
    File::open(options.url_file.clone())
      .expect(&format!("Failed to load file '{}'", options.url_file)),
  )
  .lines()
  .skip(skipped_chunk_count * LINES_PER_CHUNK)
  .peekable();
  let mut chunk_index = skipped_chunk_count;
  let mut urls_attempted = 0;
  while url_lines.by_ref().peek().is_some() {
    chunk_index += 1;
    let line_chunk: Vec<_> = url_lines
      .by_ref()
      .take(LINES_PER_CHUNK)
      .map(|maybe_line| maybe_line.expect("Failed to get line of url file"))
      .collect();
    let preprocessed_urls = preprocess_urls(line_chunk.clone());
    let url_count = preprocessed_urls.len();
    println!(
      "\nChunk {}, attempted {} urls since startup",
      chunk_index, urls_attempted
    );
    scrape_urls(chunk_index, preprocessed_urls, &options).await;
    urls_attempted += url_count;
  }
}

#[tokio::main]
async fn main() {
  let options = ScrapeOptions::parse();
  if options.count_documents {
    count_documents();
  } else {
    scrape(options).await;
  }
}

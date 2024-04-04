use std::{fs, io::Write, process::Command};

use serde::Deserialize;

use tokio::task::JoinHandle;

use clap::{arg, Parser};

const EXTENSIONS: [&str; 2] =
  ["uBlock0.chromium", "bypass-paywalls-chrome-clean-v3.6.1.0"];

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ScrapeOptions {
  #[arg(long, default_value_t = 1)]
  worker_count: u16,
  #[arg(long, default_value_t = false)]
  descend_urls: bool,
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
    .args(["--workers", options.worker_count.to_string().as_ref()])
    .args(["--url", url])
    .arg("--text")
    .args(["--collection", folder_name]);
  //println!("{:?}", browsertrix_command);
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

fn gather_documents_from_crawl(folder_name: &str) -> Vec<ParsedDocument> {
  let file_contents = fs::read_to_string(format!(
    "./crawls/collections/{}/pages/pages.jsonl",
    folder_name
  ))
  .expect(&format!(
    "Failed to open pages.jsonl file for {}",
    folder_name
  ));
  let mut lines = file_contents.lines().into_iter();
  lines.next();
  let mut line_strings = lines
    .map(|line| String::from(line))
    .collect::<Vec<String>>();
  let documents: Vec<ParsedDocument> = line_strings
    .iter_mut()
    .filter_map(|line| {
      if line.is_empty() {
        None
      } else {
        Some(unsafe { simd_json::from_str(line) }.expect(&format!(
          "Failed to parse json document for {}",
          folder_name
        )))
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

async fn scrape_urls(urls: &[&str], options: ScrapeOptions) {
  //println!("options: {:?}", options);
  let mut document_processing_join_handles: Vec<JoinHandle<()>> = vec![];

  for (index, url) in urls.into_iter().enumerate() {
    println!("{}: {}", index, url);
    let folder_name = format!("{}", index);
    scrape_url(url, &folder_name, &options);
    document_processing_join_handles.push(tokio::spawn(async move {
      let documents = gather_documents_from_crawl(&folder_name);
      save_documents(&folder_name, documents)
        .expect("Failed to save documents");
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
  let urls = [
    "https://example.com/",
    "https://kristenrankin.art/",
    //"https://webscraper.io/test-sites/e-commerce/allinone/",
  ];
  scrape_urls(&urls, ScrapeOptions::parse()).await;
}

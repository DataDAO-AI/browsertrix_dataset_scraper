use std::{fs, process::Command};

use serde::Deserialize;

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
}

fn scrape_url(url: &str, folder_name: &str) {
  let browsertrix_output = Command::new("docker")
    .arg("run")
    .args(["-v", "./crawls:/crawls/"])
    .args(["-d", "webrecorder/browsertrix-crawler"])
    .arg("crawl")
    .args(["--url", url])
    .arg("--text")
    .args(["--collection", folder_name])
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
  println!(
    "browsertrix docker process id: [{}]",
    browsertrix_stdout.trim()
  );
  let docker_wait_output = Command::new("docker")
    .arg("wait")
    .arg(browsertrix_stdout.trim())
    .output()
    .expect("failed to execute docker wait for browsertrix container");
  let docker_wait_stdout = match docker_wait_output.status.code() {
    Some(exit_code) => {
      if exit_code == 0 {
        if !docker_wait_output.stderr.is_empty() {
          panic_with_stderr!(docker_wait_output, "docker wait");
        }
        String::from_utf8(docker_wait_output.stdout)
          .expect("failed to interpret stdout of docket wait command as String")
      } else {
        println!("Recieved exit code {} from docker wait process", exit_code);
        panic_with_stderr!(docker_wait_output, "docker wait");
      }
    }
    None => {
      panic!("Failed to get exit status code for docker wait process");
    }
  };
  println!("docker wait stdout: {}", docker_wait_stdout);
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
  println!("\nFile contents:\n\n{}", file_contents);
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
  println!("documents: {}", documents.len());
  documents
}

fn main() {
  let folder_name = "0";
  scrape_url("https://example.com/", folder_name);
  let documents = gather_documents_from_crawl(folder_name);
  for document in documents {
    println!("{}: {}\n\n", document.url, document.text);
  }
}

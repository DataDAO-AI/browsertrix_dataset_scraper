use std::process::{Command, Output};

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

fn main() {
  let browsertrix_output = Command::new("docker")
    .arg("run")
    .args(["-v", "./crawls:/crawls/"])
    .args(["-d", "webrecorder/browsertrix-crawler"])
    .arg("crawl")
    .args(["--url", "https://example.com/"])
    .arg("--text")
    .args(["--collection", "test"])
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

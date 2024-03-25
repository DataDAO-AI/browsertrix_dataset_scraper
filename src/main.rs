use std::process::Command;

/*
docker run
       -v ./crawls:/crawls/
       -d webrecorder/browsertrix-crawler
       crawl
       --url https://example.com/
       --text
       --collection test
*/
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
    .expect("failed to execte ls");
  match browsertrix_output.status.code() {
    Some(exit_code) => {
      if exit_code == 0 {
        if !browsertrix_output.stderr.is_empty() {
          panic!(
            "browsertrix docker process returned stderr:\n{}",
            String::from_utf8(browsertrix_output.stderr).expect(
              "failed to interpret stderr of browsertrix command as String",
            )
          )
        }
        let browsertrix_stdout = String::from_utf8(browsertrix_output.stdout)
          .expect(
            "failed to interpret stdout of browsertrix command as String",
          );
        println!("stdout: {}", browsertrix_stdout);
      } else {
        panic!(
          "Recieved exit code {} from browsertrix docker process",
          exit_code
        )
      }
    }
    None => {
      panic!("Failed to get exit status code for browsertrix docker process")
    }
  }
}

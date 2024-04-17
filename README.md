Rust code for scraping with Browsertrix.

## to run:

First, ensure that docker and [Browsertrix Crawler](https://crawler.docs.browsertrix.com/user-guide/) are installed. Run `cargo build --release` to build the executable, then run `./target/release/browsertrix_scraper` to run the scraper. The following args can be used:
  * `--url-file <path>` sets the path to the file containing the list of URLs to scrape. The default is `./urls.txt`. This should be a `.txt` file where each line is a URL to scrape. Empty lines and lines starting with `/` will be ignored.
  * `--uid <uid>` sets the uid for the process that spawns the docker container for browsertrix. This can be helpful for getting the right permissions for running docker.
  * `--workers <N>` sets the number of threads to use
  * `--descend-urls` will attempt to crawl sub-directories of each URL in the list, rather than just scraping the listed URLs (which is the default behavior).
  * `--chunk <N>` skips the first `N` chunks. This is useful for starting the scraper back up at a specific part of the url list after interruption.
  * `--chunk-documents` will count the number of documents scraped so far, by looking over the `pages.jsonl` files in `./crawls/collections/`. If this argument as used the scraper won't run, the program will just count the number of scraped documents and then exit.
# plugins

To redownload the extensions that browsertrix will use in the crawl, run `download_plugins.sh`. After freshly downloading the extensions, they need to be manually configured. Current configuration:
* `ublock`: With browsertrix's interactive mode active, go to the extension's settings.
  * Under the `Settings` tab (opens by default) enable:
    * check "Block media elements larger than <N> kb", set N to 5 or smth
    * check "Block remote fonts"
  * Under the `Filter list` tab:
    * check everything that seems worthwhile.
* `Bypass Paywalls Clean`: in the extension's `manifest.json`, add `"http://*/*","https://*/*",` to the `permissions` field.

### run in browsertrix's interactive mode
The following command starts a local server at `localhost:9223` that displays the brave browser, and can save a profile to be used during the crawl. This can be used to configure extensions. The current profile at `./chrome_profile/profile.tar.gz` is used as a starting point, and the new profile is saved to `./chrome_profile/new_profile.tar.gz`. The main scraper script uses `profile.tar.gz`, so make sure to delete the old and rename the new profile after making modifications to the profile.

```
docker run -e CHROME_FLAGS="--disable-extensions-except=/ext/uBlock0.chromium/,/ext/bypass-paywalls-chrome-clean-v3.6.1.0/" -p 6080:6080 -p 9223:9223 -v ./chrome_plugins/:/ext/ -v ./chrome_profile/:/chrome_profile/ -it webrecorder/browsertrix-crawler create-login-profile --url "https://example.com/" --profile "/chrome_profile/profile.tar.gz" --filename "/chrome_profile/new_profile.tar.gz"
```

## todo
* fix `--chunk` off-by-one weirdness
* make `--chunk` just skip the first urls rather than just rejecting them in the while loop, so startup doesn't take as long
* on startup, if `--chunk` isn't specified, look for the `/url_chunks/` file with the largest index, and start from there (to allow for startup after crashes without manually changing the `--chunk` arg)
  * if `/url_chunks/` is empty or doesn't exist, start at 0
* On the scraping server, chaning the service to restart itself on a crash, and remove the `--chunk` arg from `startup.sh`
* for each domain, try headless vs headful, and also with/without extensions, and find the cheapest setup that works
  * can any extensions be used in headless mode? Would be nice to ublock

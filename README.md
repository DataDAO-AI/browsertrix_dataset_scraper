Rust code for scraping with Browsertrix.

## to run:

First, ensure that docker and [Browsertrix Crawler](https://crawler.docs.browsertrix.com/user-guide/) are installed.

Run `download_plugins.sh` to download the extensions that browsertrix will use in the crawl.

### run in browsertrix's interactive mode
This starts a local server at `localhost:9223` that displays the brave browser, and can save a profile to be used during the crawl. This can be used to configure extensions.

```
docker run -e CHROME_FLAGS="--disable-extensions-except=/ext/uBlock0.chromium/,/ext/Consent-O-Matic-1.0.13/Extension/,/ext/bypass-paywalls-chrome-clean-v3.6.1.0/" -p 6080:6080 -p 9223:9223 -v ./chrome_plugins/:/ext/ -v $PWD/chrome_profile/:/chrome_profile/ -it webrecorder/browsertrix-crawler create-login-profile --url "https://example.com/" --profile "/chrome_profile/profile.tar.gz" --filename "/chrome_profile/newProfile.tar.gz"
```

## todo
* given a list of urls, order them such that requests to any single domain are as spread out as possible to avoid rate limiting
* log failures
  * status code
  * very short documents
    * or maybe, shorter than OpenWebTextv2's version, if it exists
* command line args for
* for each domain, try headless vs headful, and also with/without extensions, and find the cheapest setup that works

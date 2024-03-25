Goal: take a big list of urls from a json file, and scrape all of them, ultimately turning them into one big dataset. Ideally this should be in the same format as OpenWebText so that the RedPajamaV2 code for OWT can be applied directly to it.

Default suggested command from browsertrix documentation:
`docker run -v $PWD/crawls:/crawls/ -it webrecorder/browsertrix-crawler crawl --url [URL] --generateWACZ --text --collection test`
  * `-it` is a combo of `-i` and `-t`
    * `-t` attaches the container output to the terminal (prob don't want)
    * `-i` attaches terminal input to the docker process (can be escaped with `ctrl-D`) (prob don't want)
    * `-v $PWD/crawls:/crawls/` identifies `./crawls` with the `/crawls/` directory in the docker container, so that the output is saved to the local `./crawls` directory

current draft command:
`docker run -v ./crawls:/crawls/ -d webrecorder/browsertrix-crawler crawl --url https://example.com/ --text --collection test`

The above command scrapes everything from a given url, and will output in a file at `./crawls/collections/[CRAWL_NAME]/pages/pages.jsonl`
  * `[CRAWL_NAME]` is whatever is passed to the browsertrix `--collection` argument
  * Each line is a json object, where the relevant fields seem to be `url` and `text` (which seems to be the extracted text, not sure if there are options for this)
  * it outputs a docker container id (a 64 hex character string), which can then be used with `docker wait [CONTAINER_ID]` to wait for that container to finish running, returns the exit code (should be 0 if everything's ok, should mark any that don't end with 1 to be rerun later)

todo:
* figure out the format of OWT, and write some rust/python code for transforming the browsertrix outputs into that format
  * looks like just a folder of (.xz compressed) folders of raw text files
  * so I guess it would work fine if we just put everything in one big folder? So it's just like:
    * ```
      - openwebtextv3
        - urlsf_subset00-1_data.xz
          - 1.txt
          - 2.txt
          ...
      ```
        * idk what the name of the inner folder should be, just took that one from openwebtext chunk 0. Does the name matter for RPv2's purposes?
* figure out how to install plugins in browsertrix
  * from the old documentation (only seems to exist in an old version of the README from ~6 months ago, not included in the current documentation, hope this still works?):
    * **Install uBlock Origin adblocker or any other browser extension**

      ```bash
      wget https://github.com/gorhill/uBlock/releases/download/1.41.8/uBlock0_1.41.8.chromium.zip
      unzip uBlock0_1.41.8.chromium.zip
      docker run -e CHROME_FLAGS="--disable-extensions-except=/ext/ublock --load-extension=/ext/ublock" -v $PWD/uBlock0.chromium:/ext/ublock ...
      ```
    * comes from here: https://github.com/webrecorder/browsertrix-crawler/tree/debfe8945fad30d442687161da2ba6dbd55dfa27
* write rust/bash code for:
  * spawing a bunch browsertrix containers
    * waiting for them to finish
      * merge and serialize the outputs
  * formatting the text in the OWT format

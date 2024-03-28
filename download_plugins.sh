#!/bin/bash
mkdir chrome_plugins
cd chrome_plugins
wget https://github.com/gorhill/uBlock/releases/download/1.56.0/uBlock0_1.56.0.chromium.zip
unzip -o uBlock0_1.56.0.chromium.zip
rm uBlock0_1.56.0.chromium.zip
wget https://github.com/cavi-au/Consent-O-Matic/archive/refs/tags/v1.0.13.zip
unzip -o v1.0.13.zip
rm v1.0.13.zip
wget https://gitlab.com/magnolia1234/bypass-paywalls-chrome-clean/-/archive/v3.6.1.0/bypass-paywalls-chrome-clean-v3.6.1.0.zip
unzip -o bypass-paywalls-chrome-clean-v3.6.1.0.zip
rm bypass-paywalls-chrome-clean-v3.6.1.0.zip

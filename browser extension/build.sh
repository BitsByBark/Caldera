#!/bin/bash
# builds chrome and firefox extension zips

# chrome
cp manifest.chrome.json manifest.json
zip -r caldera-extension-chrome.zip manifest.json content.js background.js dev.js icon*.png

# firefox
(cd firefox && zip -r caldera-extension-firefox.zip manifest.json content.js background.js icon*.png)

echo "done"

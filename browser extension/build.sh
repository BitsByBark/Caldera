#!/bin/bash
# builds chrome and firefox extension zips

# chrome
cp manifest.chrome.json manifest.json
zip -r caldera-extension-chrome.zip manifest.json content.js background.js icon*.png
rm manifest.json

# firefox
cp manifest.firefox.json manifest.json
zip -r caldera-extension-firefox.zip manifest.json content.js background.js icon*.png
cp manifest.firefox.json manifest.json

echo "done"

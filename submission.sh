#!/bin/bash
zip -r hello-os.zip . \
    -x ".git/*" \
    -x "build/*"\
    -x "target/*"\
    -x "*.lock"\
    -x ".direnv/*"

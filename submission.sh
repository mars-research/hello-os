#!/bin/bash
zip -r hello-os.zip . \
    -x ".git/*" \
    -x "build/*"\
    -x ".cargo/*"\
    -x "target/*"\
    -x "*.lock"\
    -x ".direnv/*"

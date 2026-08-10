#!/bin/bash
if [[ $BASH_SOURCE = */* ]]; then
    thisdir=${BASH_SOURCE%/*}/
else
    thisdir=./
fi
# "${thisdir}/run-project-script.mjs" -- "$1" --unroll_loops 0 | grep "project stopped"
"${thisdir}/run-project-script.mjs" -- "$1" #2>&1 >/dev/null | grep Error
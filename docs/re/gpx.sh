#!/bin/sh
# gpx.sh OFF  — list instructions referencing gp-relative OFF (e.g. -0x65d0)
grep -- "$1(\$gp)\|\$gp, $1\$" "$(dirname "$0")/out/code.asm"

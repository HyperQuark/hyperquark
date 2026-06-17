# build script for hyperquark
# a lot of code here was adapted from https://www.shellscript.sh/examples/getopts/

trap "err" ERR # exit if any command returns a non-zero exit code


err()
{
  echo;
  echo Exiting early since previous build step failed!;
  exit 1;
}

usage()
{
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  -h -?  show this help screen"
  echo "  -d     build for development"
  echo "  -p     build for production"
  echo "  -W     build wasm"
  echo "  -o     do not run wasm-opt"
  echo "  -s     run wasm-opt with -Os"
  echo "  -z     run wasm-opt with -Oz"
  echo "  -v     verbose output"
  echo "  -P     enable DWARF debugging and panicking"
  echo "  -D     also build rustdocs"
  exit 1
}

set_variable()
{
  local varname=$1
  shift
  if [ -z "${!varname}" ]; then
    eval "$varname=\"$@\""
  else
    echo "Error: $varname already set. This probably means that you've used two conflicting flags."
    echo
    usage
  fi
}

unset VITE WASM PROD RUSTDOC;
QUIET=1;
while getopts 'dpWoszvhPD' c
do
  case $c in
    d) set_variable PROD 0 ;;
    p) set_variable PROD 1 ;;
    W) set_variable WASM 1 ;;
    o) set_variable WOPT 0 ;;
    s) set_variable WOPT 1 ;;
    z) set_variable WOPT 2 ;;
    P) set_variable DWARF 1 ;;
    D) set_variable RUSTDOC 1;;
    v) unset QUIET ;;
    h|?) usage ;;
  esac
done

[ -z $WASM ] && set_variable WASM 0;

if [ $WASM = "1" ]; then
  [ -z $PROD ] && usage;
  if [ -z DWARF ]; then
    set_variable DWARF 0;
  fi
fi

if [ -z $WOPT ]; then
  if [[ "$PROD" = "1" ]]; then
    set_variable WOPT 2;
  else
    set_variable WOPT 0;
  fi
fi

[[ "$WASM" == "0" && "$WOPT" == "0" && "$RUSTDOC" != "1" ]] && echo "exiting (nothing to build)" && exit 0

if [[ "$WASM" == "1" ]]; then
  mkdir -p /tmp/hq-build/js/compiler;
  mkdir -p /tmp/hq-build/js/no-compiler;
  if [[ "$PROD" == "1" ]]; then
    echo "building hyperquark (compiler) for production..."
    cargo build --target=wasm32-unknown-unknown --release ${QUIET:+--quiet} ${DWARF:+--features="compiler panic"}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=/tmp/hq-build/js/compiler ${DWARF:+--keep-debug}
    echo "building hyperquark (no compiler) for production..."
    cargo build --target=wasm32-unknown-unknown --release ${QUIET:+--quiet} --no-default-features ${DWARF:+--features=panic}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=/tmp/hq-build/js/no-compiler ${DWARF:+--keep-debug}
  else
    echo "building hyperquark (compiler) for development..."
    cargo build --target=wasm32-unknown-unknown ${QUIET:+--quiet} ${DWARF:+--features="compiler panic"}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=/tmp/hq-build/js/compiler ${DWARF:+--keep-debug}
    echo "building hyperquark (no compiler) for development..."
    cargo build --target=wasm32-unknown-unknown ${QUIET:+--quiet} --no-default-features ${DWARF:+--features=panic}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=/tmp/hq-build/js/no-compiler ${DWARF:+--keep-debug}
  fi
  mv $(cargo outdir --no-names --quiet)/imports.ts /tmp/hq-build/js/imports.ts
  node opcodes.mjs
fi

if [[ "$WOPT" == "1" ]]; then
  echo running wasm-opt -Os...
  wasm-opt -Os -g /tmp/hq-build/js/compiler/hyperquark_bg.wasm -o /tmp/hq-build/js/compiler/hyperquark_bg.wasm
  wasm-opt -Os -g /tmp/hq-build/js/no-compiler/hyperquark_bg.wasm -o /tmp/hq-build/js/no-compiler/hyperquark_bg.wasm
fi
if [[ "$WOPT" == "2" ]]; then
  echo running wasm-opt -Oz...
  wasm-opt -Oz -g /tmp/hq-build/js/compiler/hyperquark_bg.wasm -o /tmp/hq-build/js/compiler/hyperquark_bg.wasm
  wasm-opt -Oz -g /tmp/hq-build/js/no-compiler/hyperquark_bg.wasm -o /tmp/hq-build/js/no-compiler/hyperquark_bg.wasm
fi

if [[ "$WASM" == "1" ]] || [[ "$WOPT" == "1" ]] || [[ "$WOPT" == "2" ]]; then
  mkdir -p /tmp/hq-build/js-new
  cp -a js/* /tmp/hq-build/js-new/
  cp -a /tmp/hq-build/js/* /tmp/hq-build/js-new/
  rm -rf js
  mv -fT /tmp/hq-build/js-new js
fi

if [[ "$RUSTDOC" == "1" ]]; then
  echo building rust docs
  # cargo doc --workspace --exclude sb3
  RUSTDOCFLAGS="--html-in-header typst-header.html" cargo doc --no-deps --document-private-items
  rm -rf playground/public/docs/internal
  cp -ra target/doc playground/public/docs/internal
fi
echo finished!
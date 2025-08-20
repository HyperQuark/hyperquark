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
  echo "  -V     build the website with vite"
  echo "  -W     build wasm"
  echo "  -o     do not run wasm-opt"
  echo "  -s     run wasm-opt with -Os"
  echo "  -z     run wasm-opt with -Oz"
  echo "  -v     verbose output"
  echo "  -D     enable DWARF debuggin and panicking"
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

unset VITE WASM PROD;
QUIET=1;
while getopts 'dpVWoszvhD' c
do
  case $c in
    d) set_variable PROD 0 ;;
    p) set_variable PROD 1 ;;
    V) set_variable VITE 1 ;;
    W) set_variable WASM 1 ;;
    o) set_variable WOPT 0 ;;
    s) set_variable WOPT 1 ;;
    z) set_variable WOPT 2 ;;
    D) set_variable DWARF 1 ;;
    v) unset QUIET ;;
    h|?) usage ;;
  esac
done

[ -z $WASM ] && set_variable WASM 0;
[ -z $VITE ] && set_variable VITE 0;

[ -z $PROD ] && usage;

if [ -z $WOPT ]; then
  if [ $PROD = "1" ]; then
    set_variable WOPT 2;
  else
    set_variable WOPT 0;
  fi
fi

if [ -z DWARF ]; then
  if [ $PROD = "0" ]; then
    set_variable DWARF 1;
  else 
    set_variable DWARF 0;
  fi
fi

[ $VITE = "0" ] && [ $WASM = "0" ] && [ $WOPT = "0" ] && echo "exiting (nothing to build)" && exit 0

if [ $WASM = "1" ]; then
  if [ $PROD = "1" ]; then
    echo "building hyperquark (compiler) for production..."
    cargo build --target=wasm32-unknown-unknown --release ${QUIET:+--quiet} ${DWARF:+--features="compiler panic"}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=js/compiler ${DWARF:+--keep-debug}
    echo "building hyperquark (no compiler) for production..."
    cargo build --target=wasm32-unknown-unknown --release ${QUIET:+--quiet} --no-default-features ${DWARF:+--features=panic}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=js/no-compiler ${DWARF:+--keep-debug}
  else
    echo "building hyperquark (compiler) for development..."
    cargo build --target=wasm32-unknown-unknown ${QUIET:+--quiet} ${DWARF:+--features="compiler panic"}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=js/compiler ${DWARF:+--keep-debug}
    echo "building hyperquark (no compiler) for development..."
    cargo build --target=wasm32-unknown-unknown ${QUIET:+--quiet} --no-default-features ${DWARF:+--features=panic}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=js/no-compiler ${DWARF:+--keep-debug}
  fi
  mv $(cargo outdir --no-names --quiet)/imports.ts js/imports.ts
  node opcodes.mjs
fi
if [ $WOPT = "1" ]; then
  echo running wasm-opt -Os...
  wasm-opt -Os -g js/compiler/hyperquark_bg.wasm -o js/compiler/hyperquark_bg.wasm
  wasm-opt -Os -g js/no-compiler/hyperquark_bg.wasm -o js/no-compiler/hyperquark_bg.wasm
fi
if [ $WOPT = "2" ]; then
  echo running wasm-opt -Oz...
  wasm-opt -Oz -g js/compiler/hyperquark_bg.wasm -o js/compiler/hyperquark_bg.wasm
  wasm-opt -Oz -g js/no-compiler/hyperquark_bg.wasm -o js/no-compiler/hyperquark_bg.wasm
fi
if [ $VITE = "1" ]; then
  echo running npm build...
  npm run build
fi
echo finished!
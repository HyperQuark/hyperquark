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
  echo "  -O     run wasm-opt"
  echo "  -s     run wasm-opt with -Os"
  echo "  -z     run wasm-opt with -Oz"
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
while getopts 'dpwvoWVszhi' c
do
  case $c in
    d) set_variable PROD 0 ;;
    p) set_variable PROD 1 ;;
    V) set_variable VITE 1 ;;
    W) set_variable WASM 1 ;;
    o) set_variable WOPT 0 ;;
    s) set_variable WOPT 1 ;;
    z) set_variable WOPT 2 ;;
    i) unset QUIET ;;
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
[ $VITE = "0" ] && [ $WASM = "0" ] && [ $WOPT = "0" ] && echo "exiting (nothing to build)" && exit 0

if [ $WASM = "1" ]; then
  if [ $PROD = "1" ]; then
    echo building hyperquark for production...
    cargo build --target=wasm32-unknown-unknown --release ${QUIET:+--quiet}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=js
  else
    echo building hyperquark for development...
    cargo build --target=wasm32-unknown-unknown ${QUIET:+--quiet}
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=js
  fi
fi
if [ $WOPT = "1" ]; then
  echo running wasm-opt -Os...
  wasm-opt -Os -g js/hyperquark_bg.wasm -o js/hyperquark_bg.wasm
fi
if [ $WOPT = "2" ]; then
  echo running wasm-opt -Oz...
  wasm-opt -Oz -g js/hyperquark_bg.wasm -o js/hyperquark_bg.wasm
fi
if [ $VITE = "1" ]; then
  echo running npm build...
  npm run build
fi
echo finished!
# build script for hyperquark
# a lot of code here was adapted from https://www.shellscript.sh/examples/getopts/

usage()
{
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  -h -?  show this help screen"
  echo "  -d     build for development"
  echo "  -p     build for production"
  echo "  -V     build the website with vite"
  echo "  -v     do not build the website with vite"
  echo "  -W     build wasm"
  echo "  -w     do not build wasm"
  echo "  -o     do not run wasm-opt"
  echo "  -O     run wasm-opt"
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

unset PROD VITE WASM
while getopts 'dpwvoWVO' c
do
  case $c in
    d) set_variable PROD 0 ;;
    p) set_variable PROD 1 ;;
    v) set_variable VITE 0 ;;
    w) set_variable WASM 0 ;;
    V) set_variable VITE 1 ;;
    W) set_variable WASM 1 ;;
    o) set_variable WOPT 0 ;;
    O) set_variable WOPT 1 ;;
    h|?) usage ;;
  esac
done

[ -z $PROD ] && usage
[ -z $VITE ] && usage
[ -z $WASM ] && usage
if [ -z $WOPT ]; then
  if [ $PROD = "1" ]; then
    set_variable WOPT 1;
  else
    set_variable WOPT 0;
  fi
fi
[ $VITE = "0" ] && [ $WASM = "0" ] && [ $WOPT = "0" ] && echo "exiting (nothing to build)" && exit 0

if [ $WASM = "1" ]; then
  if [ $PROD = "1" ]; then
    echo building hyperquark for production...
    cargo build --target=wasm32-unknown-unknown --release --quiet
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=js
  else
    echo building hyperquark for development...
    cargo build --target=wasm32-unknown-unknown --quiet
    echo running wasm-bindgen...
    wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=js
  fi
fi
if [ $WOPT = "1" ]; then
  echo running wasm-opt...
  wasm-opt -Oz js/hyperquark_bg.wasm -o js/hyperquark_bg.wasm
fi
if [ $VITE = "1" ]; then
  echo running npm build...
  npm run build
fi
echo done!
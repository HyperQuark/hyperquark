PROD=1
while [ "$#" -gt "0" ]
do
  if [ $1 = "--dev" ]; then
    PROD=0
    break
  else
    shift
  fi
done
if [ PROD = "1" ]; then
  echo building hyperquark for production...
  cargo build --target=wasm32-unknown-unknown --release --quiet
  echo running wasm-bindgen...
  wasm-bindgen target/wasm32-unknown-unknown/release/hyperquark.wasm --out-dir=js
  echo running wasm-opt...
  wasm-opt -Oz js/hyperquark_bg.wasm -o js/hyperquark_bg.wasm
  echo done
else
  echo building hyperquark for devlopment...
  cargo build --target=wasm32-unknown-unknown --quiet
  echo running wasm-bindgen...
  wasm-bindgen target/wasm32-unknown-unknown/debug/hyperquark.wasm --out-dir=js
  echo done
fi
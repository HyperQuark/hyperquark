# wasm-gen

This is an internal crate used to generate WebAssembly instructions for blocks. It relies upon HyperQuark's internal types so cannot be used outside of HyperQuark.

## Usage

The `wasm![]` macro produces a `Vec<wasm_encoder::Instruction<'static>>`. Inputs to the macro are (non-namespaced) wasm_encoder [`Instruction`](https://docs.rs/wasm-encoder/latest/wasm_encoder/enum.Instruction.html)s, or a 'special' instruction. Special instructions are currently:
- `@nanreduce(input_name)` - checks the top value on the stack for NaN-ness (and replaces it with a zero if it is), only if `input_name` (must be an in-scope [`IrType`](../src/ir/types.rs)) could possibly be `NaN`. Assumes (rightly so) that the top item on the stack is an `f64`.
- `@box(input_name)` - boxes the top value on the stack if `input_name` is a base type
- `@isnan(input_name)` - checks if the top value on the stack for NaN-ness, only if `input_name` could possibly be NaN. Assumes the top item on the stack is an `f64`.
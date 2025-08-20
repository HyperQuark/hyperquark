[![Discord](https://img.shields.io/discord/1095993821076131950?label=chat&logo=discord)](https://discord.gg/w5C8fdb5EQ)

# HyperQuark
Compile scratch projects to WASM

## Prerequisites

- [Rust](https://rust-lang.org) v1.65.0 or later
- the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- wasm-bindgen-cli (`cargo install -f wasm-bindgen-cli`)
- wasm-opt (install binaryen using whatever package manager you use)
- `cargo-outdir` (`cargo install cargo-outdir`)

## Building

```bash
./build.sh -Wp # use -Wd for a debug build without optimisation
```

You may need to run `chmod +x build.sh` if it says it doesn't have permission.

The build script has additonal configuration options; run `./build.sh -h` for info on these.

If you experience runtime stack overflow errors in debug mode, try using the `-s` or `-z` options to enable wasm-opt, or build in production mode; weird wasm errors in production mode may conversely be solved by *disabling* wasm-opt using the `-o` flag.

If an error occurs during execution and you need a stack trace, run with the `-D` flag to enable panicking with DWARF debug symbols. You need to [set up your browser to display a proper stack trace using these debug symbols](https://users.rust-lang.org/t/getting-raw-wasm-debugging-working-nicely-in-chrome-devtools/94646). You'll need to disable wasm-opt using `-o` for this to work, as wasm-opt currently crashes when it encounters DWARF.

To preview the website (e.g. to run a project), use `npm run watch`. Do not just run `vite` as this will build in debug mode which doesn't work.

## Adding a new block

To add a new block named `category_opcode`, if it cannot be reduced to simpler blocks:
1. create `src/instructions/category/opcode.rs`. Make sure to `use super::super::prelude::*` and create the relevant `pub` items:
- (optional) `pub struct Fields` (must be `Debug` and `Clone`)
- `pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>, (fields: &Fields)?) -> HQResult<Vec<InternalInstruction>>;`
- - wasm is generated using the `wasm_gen::wasm` macro. See [its README](./wasm-gen/README.md) for usage instructions, or e.g. [say.rs](./src/instructions/looks/say.rs) for an example.
- `pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>>;`
- - these should really be base types (see BASE_TYPES in [types.rs](./src/ir/types.rs))
- `pub fn output_type(inputs: Rc<[IrType]>, (fields: &Fields)?) -> HQResult<ReturnType>;`
- - the output type should be as restrictive as possible; loose output types can cause us to lose out on some optimisations
- - Most output types should be either `ReturnType::None` or `Singleton(IrType)` (included in the module prelude); blocks can return multiple values via `MultiValue(Rc<[IrType]>)` but probably shouldn't.
- ensure to add relevant `instructions_test!`s - see [instructions/tests.rs](./src/instructions/tests.rs) for usage
2. add `pub mod opcode;` to `src/instructions/category.rs`, creating the file if needed
- if you're creating the category file, add `mod category;` to `src/instructions.rs`
3. add the block to `from_normal_block` in `src/ir/blocks.rs`; in most cases this should be a direct mapping of `BlockOpcode::category_opcode => IrOpcode::category_opcode`
4. add the block's input names to `input_names` in `src/ir/blocks.rs`

If the block *can* be reduced to simpler steps, only carry out steps 3 and 4 above.

## generared WASM module memory layout

|    name       |                           number of bytes                            | optional? | description                                                                                                                                                                                                                                                                                                        |
| :-----------: | :------------------------------------------------------------------: | :-------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------: |
| sprite_info | 80 \* number of sprites (i.e. `target num - 1`) | yes | see [src/wasm/sprite.rs](./src/wasm/sprite.rs)
| threads | 4 \* thread_num | present iff using the CallIndirect scheduler | indices of the next step funcs of currently running threads |
<!--|    pen        |                       360 \* 480 \* 4 = 691200                       |    yes    | present if pen is used; the pen layer: 4 bytes for each rgba pixel, from left to right, top to bottom                                                                                                                                                                                                              |
| spriteData    |                      43(?) \* number of sprites                      |    yes    | for each sprite (**not target**), 4 bytes each (1 f32 each) for: x, y, size, direction, costume number, pitch, pan,layer number; plus 1 byte each for: colour effect, ghost effect, mosaic effect, whirl effect, pixelate effect, fisheye effect, brightness effect, volume, visibility, rotation style, draggable |
| stageData     |                                  8                                   |    no     | 4 bytes each for: backdrop number; plus 1 byte each for: volume, video state, tempo, video transparency                                                                                                                                                                                                            |
| cloneData     |                         43(?) \* 300 = 12900                         |    yes    | if a `create clone of ()` block is present, same as above, but for each clone.                                                                                                                                                                                                                                     |
-->
<!--| cloneVars     | 300 \* 12 \* max amount of local variables in any one sprite |    yes    | if clones can be present, local variables for those clones                                                                                                                                                                                                                                                         |
-->


### Memory layout guarantees

All values should be guaranteed to have the maximum possible alignment for that value type. Unused/padding bytes are not guaranteed to have any specific values and may be overwritten.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
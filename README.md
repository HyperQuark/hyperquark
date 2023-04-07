# HyperQuark
Compile scratch projects to WASM

## Prerequisites

- [Rust](https://rust-lang.org) (v1.65.0 or later)
- wasm-bindgen-cli (`cargo install -f wasm-bindgen-cli`)
- wasm-opt (install binaryen using whatever oackage manager you use)

## Building

```bash
./build.sh
```

You may need to run `chmod +x build.sh` if it says it doesn't have permission.

## generared WASM module memory layout

|    name       |                           number of bytes                            | optional? | description                                                                                                                                                                                                                                                                                                        |
| :-----------: | :------------------------------------------------------------------: | :-------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------: |
| redraw_requested |                                 4                                  |    no     | if a redraw has been requested or not                                                                                                                                                                                                                                                                            |
| thread_num | 4 | no | the number of currently running threads |
| threads | 4 * thread_num | imdices of the next step funcs of currently running threads |
<!--|    pen        |                       360 \* 480 \* 4 = 691200                       |    yes    | present if pen is used; the pen layer: 4 bytes for each rgba pixel, from left to right, top to bottom                                                                                                                                                                                                              |
| spriteData    |                      43(?) \* number of sprites                      |    yes    | for each sprite (**not target**), 4 bytes each (1 f32 each) for: x, y, size, direction, costume number, pitch, pan,layer number; plus 1 byte each for: colour effect, ghost effect, mosaic effect, whirl effect, pixelate effect, fisheye effect, brightness effect, volume, visibility, rotation style, draggable |
| stageData     |                                  8                                   |    no     | 4 bytes each for: backdrop number; plus 1 byte each for: volume, video state, tempo, video transparency                                                                                                                                                                                                            |
| cloneData     |                         43(?) \* 300 = 12900                         |    yes    | if a `create clone of ()` block is present, same as above, but for each clone.                                                                                                                                                                                                                                     |
-->
| vars | 12 \* number of global & lpcal variables | yes | see [variables](#variables) |
<!--| cloneVars     | 300 \* 12 \* max amount of local variables in any one sprite |    yes    | if clones can be present, local variables for those clones                                                                                                                                                                                                                                                         |
-->
### Variables

| byte | description                          |
| :--: | :-----------------------------------: |
| 0-3  | identifies the [type](#variable-types) of the variable  |
| 4-11 | identifies the value of the variable |

### Variable types

| value |            type           | variable value type | value description                                                         |
| :---: | :-----------------------: | :-----------------: | :-----------------------------------------------------------------------: |
| 0x00  |           float64         |        `f64`        |                            read as a float                               |
| 0x01  |           bool64          |        `i64`        |   read as an int; converted to `f64` for calculations involving numbers   |
| 0x02  | externref string (64 bit) |        `i64`        | wrapped to a 32 bit pointer to an `externref` value in the `anyref` table |

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
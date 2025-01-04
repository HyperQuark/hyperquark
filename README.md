[![Discord](https://img.shields.io/discord/1095993821076131950?label=chat&logo=discord)](https://discord.gg/w5C8fdb5EQ)

# HyperQuark
Compile scratch projects to WASM

## Prerequisites

- [Rust](https://rust-lang.org) (v1.65.0 or later)
- the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- wasm-bindgen-cli (`cargo install -f wasm-bindgen-cli`)
- ezno (`cargo install ezno`)
- wasm-opt (install binaryen using whatever package manager you use)

## Building

```bash
./build.sh -pVW # use -dVW for a debug build without optimisation
```

You may need to run `chmod +x build.sh` if it says it doesn't have permission.

The build script has additonal configuration options; run `./build.sh -h` for info on these.

If you experience runtime stack overflow errors in debug mode, try using the `-O` option to enable wasm-opt.

## generared WASM module memory layout

|    name       |                           number of bytes                            | optional? | description                                                                                                                                                                                                                                                                                                        |
| :-----------: | :------------------------------------------------------------------: | :-------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------: |
| redraw_requested |                                 4                                  |    no     | if a redraw has been requested or not                                                                                                                                                                                                                                                                            |
| thread_num | 4 | no | the number of currently running threads |
| vars | 16 \* number of global & local variables | yes | see [variables](#variables) |
| sprite_info | 80 \* number of sprites (i.e. `target num - 1`) | yes | see [sprite info](#sprite-info)
| threads | 4 \* thread_num | no | imdices of the next step funcs of currently running threads |
<!--|    pen        |                       360 \* 480 \* 4 = 691200                       |    yes    | present if pen is used; the pen layer: 4 bytes for each rgba pixel, from left to right, top to bottom                                                                                                                                                                                                              |
| spriteData    |                      43(?) \* number of sprites                      |    yes    | for each sprite (**not target**), 4 bytes each (1 f32 each) for: x, y, size, direction, costume number, pitch, pan,layer number; plus 1 byte each for: colour effect, ghost effect, mosaic effect, whirl effect, pixelate effect, fisheye effect, brightness effect, volume, visibility, rotation style, draggable |
| stageData     |                                  8                                   |    no     | 4 bytes each for: backdrop number; plus 1 byte each for: volume, video state, tempo, video transparency                                                                                                                                                                                                            |
| cloneData     |                         43(?) \* 300 = 12900                         |    yes    | if a `create clone of ()` block is present, same as above, but for each clone.                                                                                                                                                                                                                                     |
-->
<!--| cloneVars     | 300 \* 12 \* max amount of local variables in any one sprite |    yes    | if clones can be present, local variables for those clones                                                                                                                                                                                                                                                         |
-->
### Sprite info

| byte | type | name | description |
| :--: | :--: | :--: | :---------: |
| 0-7  | f64  |  x   | x pos       |
| 8-15  | f64 |  y   | y pos       |
| 16-19 | f32 | pen_color | hue of pen (0-100) |
| 20-23 | f32 | pen_saturation | saturation of pen (0-100) |
| 24-27 | f32 | pen_brightness | value of pen (0-100) |
| 28-31 | f32 | pen_transparency | transparency of pen (0-100) |
| 32-47 | f32(x4) | pen_color4f | rgba color of pen [(0-1)x4] |
| 48-55 | f64 | pen_size | pen radius |
| 56 | i8 | pen_down | `1` if pen down else `0` |
| 57 | i8 | visible | `1` if sprite is visible else `0` |
| 58-59 | - | padding | reserved |
| 60-63 | i32 | costume | the current costume number, 0-indexed |
| 64-71 | f64 | size | sprite size |
| 72-79 | f64 | rotation | sprite rotation, in scratch angles (0 = up, 90 = right) |
<!--| 56-57 | ?   | padding | padding |--> 

### Variables

| byte | description                          |
| :--: | :-----------------------------------: |
| 0-3  | identifies the [type](#variable-types) of the variable  |
| 4-7 | padding
| 8-15 | identifies the value of the variable |

#### Variable types

| value |            type           | variable value type | value description                                                         |
| :---: | :-----------------------: | :-----------------: | :-----------------------------------------------------------------------: |
| 0x00  |           float64         |        `f64`        |                            a float                               |
| 0x01  |           bool64          |        `i64`        |   an integer - only the least significant bit is used   |
| 0x02  | externref string (64 bit) |        `i64`        | wrapped to a 32 bit pointer to an `externref` value in the `anyref` table |
| 0x03 | int64 | `i64` | a 64-bit integer |

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
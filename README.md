# HyperQuark
Compile scratch projects to WASM

## generared WASM module memory layout

|    name       |                           number of bytes                            | optional? | description                                                                                                                                                                                                                                                                                                        |
| :-----------: | :------------------------------------------------------------------: | :-------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    flag       |                                  1                                   |    yes    | reserved for multithreading use                                                                                                                                                                                                                                                                                    |
|    pen        |                       360 \* 480 \* 4 = 691200                       |    yes    | present if pen is used; the pen layer: 4 bytes for each rgba pixel, from left to right, top to bottom                                                                                                                                                                                                              |
| spriteData    |                      43(?) \* number of sprites                      |    yes    | for each sprite (**not target**), 4 bytes each (1 f32 each) for: x, y, size, direction, costume number, pitch, pan,layer number; plus 1 byte each for: colour effect, ghost effect, mosaic effect, whirl effect, pixelate effect, fisheye effect, brightness effect, volume, visibility, rotation style, draggable |
| stageData     |                                  8                                   |    no     | 4 bytes each for: backdrop number; plus 1 byte each for: volume, video state, tempo, video transparency                                                                                                                                                                                                            |
| cloneData     |                         43(?) \* 300 = 12900                         |    yes    | if a `create clone of ()` block is present, same as above, but for each clone.                                                                                                                                                                                                                                     |
| globalVars    |        (5 (or 9 if using f64)) \* number of global variables         |    yes    | see [variables](#variables)                                                                                                                                                                                                                                                                                        |
| localVars     |            (5 _or_ 9) \* total number of local variables             |    yes    | ditto                                                                                                                                                                                                                                                                                                              |
| cloneVars     | 300 \* (5 _or_ 9) \* max amount of local variables in any one sprite |    yes    | if clones can be present, local variables for those clones                                                                                                                                                                                                                                                         |
| nextSteps     | >= 4 * number of hat blocks                                          |    yes    | a vector of indices for the `step_funcs` funcref table, of the steps to be ran on the next tick                                                                                                                                                                                                                    |

### Variables

|       byte       | description                          |
| :--------------: | :----------------------------------- |
|        0         | identifies the type of the variable  |
| 1 - 4 _or_ 1 - 8 | identifies the value of the variable |

### Variable types

| value |  type  | variable value type | value description                                                             |
| :---: | :----: | :-----------------: | :---------------------------------------------------------------------------- |
| 0x00  | number |  `f32` _or_ `f64`   | read as a number                                                              |
| 0x01  |  bool  |        `i8`         | read as a number; converted to `f32`/`f64` for calculations involving numbers |
| 0x02  | string |        `i32`        | a pointer to an `externref` value in the `anyref` table                       |

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
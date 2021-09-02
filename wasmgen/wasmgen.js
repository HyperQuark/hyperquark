
import * as Sections from "./sections.js";
import * as  Types from "./types.js";

class WasmUint8Array extends Uint8Array {
  constructor({ types }) {
    let typeSec = new Sections.TypeSection(
      ...types.map(t => new FuncType(t.params, t.returns))
    );
    super([...WasmUint8Array.WasmHeader, typeSec]);
  }
  static MagicNumber = [0, 97, 115, 109];
  static Version = [1, 0, 0, 0];
  static WasmHeader = [
    ...WasmUint8Array.MagicNumber,
    ...WasmUint8Array.Version
  ];
}

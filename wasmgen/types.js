import { Vector, StringVector, sLeb128 } from "./encoders.js";

/**
 * Possible variable types
 */
export const NumTypes = {
  i32: 0x7f,
  i64: 0x7e,
  f32: 0x7d,
  f64: 0x7c,
  v128: 0x7b
};
/**
 * reference types
 */
export const RefTypes = {
  funcref: 0x70,
  externref: 0x6f
};
/** 
 * Function type
 */
export class FuncType extends Array {
  /**
   * @param {number[]} paramTypes - parameter type(s)
   * @param {number[]} returnTypes - return type(s)
   */
  constructor(paramTypes, returnTypes) {
    super(0x60, ...new Vector(...paramTypes), ...new Vector(...returnTypes));
  }
}
/**
 * Memory type
 */
export class MemoryType extends Array {
  /**
   * @param {number} min - minimum memory
   * @param {number} [max] - maximum memory
   * @param {boolean} [shared] - whether the memory should be shared or not
   */
  constructor(min, max, shared) {
    shared
      ? super(0x03, ...sLeb128(min), ...sLeb128(max), 0x01)
      : max
      ? super(0x01, ...sLeb128(min), ...sLeb128(max))
      : super(0x00, ...sLeb128(min));
  }
}
/**
 * possible import tags
 */
export const ImportTags = {
  func: 0,
  table: 1,
  mem: 2,
  global: 3
};

/**
 * Import type
 */
export class ImportType extends Array {
  /**
   * @param {string} module - the name of the module to import from
   * @param {string} name - the name of the the item to import
   * @param tag {ImportTags} tag - the type of import
   * @param {number|any[]} description
   */
  constructor(module, name, tag, description) {
    super(
      ...new StringVector(module),
      ...new StringVector(name),
      tag,
      ...[description].flat(1) // for a function import description may not be an array
    );
  }
}

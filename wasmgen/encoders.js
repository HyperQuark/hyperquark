/**
 * encodes integers in the leb128 format
 * this is for signed ints but it should work the same way for unsigned ints
 * unsigned version is probably slightly faster but i'm not too bothered about speed atm
 * @param {number} value
 * @return {number[]}
 */
export const sLeb128 = value => {
  value |= 0;
  const result = [];
  while (true) {
    const byte = value & 0x7f;
    value >>= 7;
    if (
      (value === 0 && (byte & 0x40) === 0) ||
      (value === -1 && (byte & 0x40) !== 0)
    ) {
      result.push(byte);
      return result;
    }
    result.push(byte | 0x80);
  }
};

/**
 * Encodes an array as a vector
 */
export class Vector extends Array {
  /**
   * @param {...any} [array] - the array to encode
   */
  constructor(...array) {
    array ||= []; // eslint-disable-line
    if (array.length) super(...sLeb128(array.length), ...array);
    else {
      super(0);
      this.push(0);
    }
  }
}

export class StringVector extends Vector {
  constructor(str) {
    super(...StringVector.encoder.encode(str));
  }
  static encoder = new TextEncoder();
}
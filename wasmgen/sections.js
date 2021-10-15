import { Vector, StringVector } from "./encoders.js";

/**
 @typedef {0x00|0x01|0x02|0x03|0x04|0x05|0x06|0x07|0x08|0x09|0x0a|0x0b|0x0c} sectionID
 */

/**
 * Base class for sections
 */
class Section extends Array {
  /**
    * @constructor
    * @param {sectionID} id - the section id
 * @param {...any} content - the section content

   */
  constructor(id, ...content) {
    super(id, ...new Vector(...content));
  }
}
export class CustomSection extends Section {
  constructor(name, info) {
    super(0x00, ...new StringVector(name), ...info);
  }
}
export class TypeSection extends Section {
  constructor(types) {
    super(0x01, ...new Vector(types).flat(1));
  }
}
export class ImportSection extends Section {
  constructor(...imports) {
    super(0x02, ...new Vector(imports).flat(1));
  }
}
export class FunctionSection extends Section {
  constructor(...types) {
    super(0x02, ...new Vector(types));
  }
}


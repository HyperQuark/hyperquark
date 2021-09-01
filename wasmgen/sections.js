import { Vector, StringVector } from "./encoders.js";

class Section extends Array {
  constructor(type, content) {
    super(type, ...new Vector(...content));
  }
}
export class CustomSection extends Section {
  constructor(name, info) {
    super(0x00, ...new StringVector(name), ...info);
  }
}
export class TypeSection extends Section {
  constructor(types) {
    super(0x01, types);
  }
}
export class ImportSection extends Section {
  constructor(...imports) {
    super(0x02, ...new Vector(imports).flat(1));
  }
}

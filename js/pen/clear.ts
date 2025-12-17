import { renderer, pen_skin } from "../shared";

export function clear() {
  renderer().penClear(pen_skin());
}

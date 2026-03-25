import { get_key_pressed } from "../shared";

export function keypressed(key: string): boolean {
  return get_key_pressed(key);
}

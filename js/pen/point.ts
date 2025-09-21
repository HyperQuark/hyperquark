import { renderer, target_skins, pen_skin } from '../shared';

export function point(radius: number, x: number, y: number, r: number, g: number, b: number, a: number) {
  console.log('called pen point with rgba color %n %n %n %n', r, g, b, a);
  renderer().penPoint(
    pen_skin(),
      {
        diameter: radius, // awkward variable naming moment
        color4f: [r, g, b, a],
      },
      x,
      y
  );
}

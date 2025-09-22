import { renderer, pen_skin } from "../shared";

export function line(
    radius: number,
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    r: number,
    g: number,
    b: number,
    a: number
) {
    console.log('called pen line with rgba color %n %n %n %n', r, g, b, a);
    renderer().penLine(
        pen_skin(),
        {
            diameter: radius,
            color4f: [r, g, b, a],
        },
        x1,
        y1,
        x2,
        y2
    );
}

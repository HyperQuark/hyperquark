export function tan(angle: number) {
  angle = angle % 360;
  switch (angle) {
    case -270:
    case 90:
      return Infinity;
    case -90:
    case 270:
      return -Infinity;
    default:
      return Math.tan((Math.PI * angle) / 180);
  }
}

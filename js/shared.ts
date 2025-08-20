let _target_names: Array<string>;
let _setup = false;
let _target_bubbles: Array<object | null>;
let _renderer;
let _pen_skin: number;
let _target_skins: Array<[number, number]>;
let _costumes: Array<Array<Costume>>;

type Costume = {
  data: string,
  dataFormat: string,
}

export function setup(
  target_names: Array<string>,
  renderer: object,
  pen_skin: number,
  target_skins: Array<[number, number]>,
  costumes: Array<Array<Costume>>,
) {
  _target_names = target_names;
  _target_bubbles = _target_names.map((_) => null);
  console.log(_target_names, _target_bubbles);
  _renderer = renderer;
  _pen_skin = pen_skin;
  _target_skins = target_skins;
  _costumes = costumes;
  _setup = true;
}

export function is_setup(): boolean {
  return _setup;
}

function check_setup() {
  if (!_setup) {
    throw new Error("shared state must be set up before use!");
  }
}

export function target_names(): Array<string> {
  check_setup();
  return _target_names;
}

export function renderer(): object {
  check_setup();
  return _renderer;
}

export function pen_skin(): number {
  check_setup();
  return _pen_skin;
}

export function target_skins(): Array<[number, number]> {
  check_setup();
  return _target_skins;
}

export function costumes(): Array<Array<Costume>> {
  check_setup();
  return _costumes;
}

export function update_bubble(
  target_index: number,
  verb: "say" | "think",
  text: string
) {
  check_setup();
  if (!_target_bubbles[target_index]) {
    _target_bubbles[target_index] = _renderer.createSkin(
      "text",
      "sprite",
      verb,
      text,
      false
    );
  } else {
    _renderer.updateTextSkin(
      _target_bubbles[target_index][0],
      verb,
      text,
      false
    );
  }
}

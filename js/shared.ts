let _target_names: Array<string>;
let _setup = false;
let _target_bubbles: Array<object | null>;
let _renderer: object;
let _pen_skin: number;
let _target_skins: Array<[number, number]>;
let _costumes: Array<Array<Costume>>;
let _queue_question: (question: string, struct: object) => void = () => {};
let _stageIndex: number;
let _update_var_val: (id: string, val: any) => void = () => {};
let _update_var_visible: (id: string, visible: boolean) => void = () => {};
let _get_key_pressed: (key: string) => boolean = (_) => false;

type Costume = {
  data: string;
  dataFormat: string;
};

export function unsetup() {
  _target_names = null;
  _target_bubbles = null;
  _renderer = null;
  _pen_skin = null;
  _target_skins = null;
  _costumes = null;
  _queue_question = () => {};
  _stageIndex = null;
  _update_var_val = () => {};
  _update_var_visible = () => {};
  _get_key_pressed = (_) => false;
  _setup = false;
}

export function setup(
  target_names: Array<string>,
  renderer: object,
  pen_skin: number,
  target_skins: Array<[number, number]>,
  costumes: Array<Array<Costume>>,
  stageIndex: number,
  {
    queue_question,
    update_var_val,
    update_var_visible,
    get_key_pressed,
  }: {
    queue_question: (question: string, struct: object) => void;
    update_var_val: (id: string, val: any) => void;
    update_var_visible: (id: string, visible: boolean) => void;
    get_key_pressed: (key: string) => boolean;
  },
) {
  _target_names = target_names;
  _target_bubbles = _target_names.map((_) => null);
  console.log(_target_names, _target_bubbles);
  _renderer = renderer;
  _pen_skin = pen_skin;
  _target_skins = target_skins;
  _costumes = costumes;
  _queue_question = queue_question;
  _stageIndex = stageIndex;
  _update_var_val = update_var_val;
  _update_var_visible = update_var_visible;
  _get_key_pressed = get_key_pressed;
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

export function stageIndex(): number {
  check_setup();
  return _stageIndex;
}

export function update_bubble(
  target_index: number,
  verb: "say" | "think",
  text: string,
) {
  check_setup();
  if (!_target_bubbles[target_index] && text !== "") {
    _target_bubbles[target_index] = _renderer.createSkin(
      "text",
      "sprite",
      verb,
      text,
      false,
    );
  } else if (text == "") {
    if (!_target_bubbles[target_index]) return;
    _renderer.destroyDrawable(_target_bubbles[target_index][1], "sprite");
    _renderer.destroySkin(_target_bubbles[target_index][0]);
    _target_bubbles[target_index] = null;
  } else {
    _renderer.updateTextSkin(
      _target_bubbles[target_index][0],
      verb,
      text,
      false,
    );
  }
}

export function queue_question(question: string, struct: object) {
  check_setup();
  _queue_question(question, struct);
}

export function update_var_val(id: string, val: any) {
  check_setup();
  _update_var_val(id, val);
}

export function update_var_visible(id: string, visible: boolean) {
  check_setup();
  _update_var_visible(id, visible);
}

export function get_key_pressed(key: string): boolean {
  check_setup();
  return _get_key_pressed(key);
}
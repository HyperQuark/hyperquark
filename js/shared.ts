let _target_names: Array<string> = [];
let _setup = false;
let _target_bubbles;
let _renderer;

export function setup(new_target_names: Array<string>, renderer: object) {
    _target_names = new_target_names;
    _target_bubbles = _target_names.map(_ => null);
    console.log(_target_names, _target_bubbles)
    _renderer = renderer;
    _setup = true;
}

export function is_setup(): boolean {
    return _setup;
}

function check_setup() {
    if (!_setup) {
        throw new Error("shared state must be set up before use!")
    }
}

export function target_names(): Array<string> {
    check_setup();
    return _target_names
}

export function update_bubble(target_index: number, verb: "say" | "think", text: string) {
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
        _renderer.updateTextSkin(_target_bubbles[target_index][0], verb, text, false);
    }
}
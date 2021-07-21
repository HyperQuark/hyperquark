export * from '../node_modules/asdom/assembly/glue'

import {document} from '../node_modules/asdom/assembly/index'

export function start (): void {
  const el = document.createElement('h1')

  el.setAttribute('foo', 'bar')

  const s: string = el.getAttribute('foo')! // returns "bar"

  el.innerHTML = /*html*/ `
    <span style="font-weight: normal; position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%)">
      <em>hello</em> from <strong>AssemblyScript</strong>
    </span>
  `
  document.body!.appendChild(el)
}

export function add(a: i32, b: i32): i32 {
  return a + b;
}
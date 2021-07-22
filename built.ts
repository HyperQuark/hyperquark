
@external('asDOM', 'trackNextRef')
export declare function trackNextRef(id: usize): void
@external('asDOM', 'log')
export declare function log(msg: string): void
@external('asDOM_Window', 'getCustomElements')
export declare function getCustomElements(id: usize, ceId: usize): void
@external('asDOM_Window', 'trackWindow')
export declare function trackWindow(id: usize): void
@external('asDOM_CustomElementRegistry', 'define')
export declare function define(id: usize, tag: string, factoryIndex: i32, attributes: string[]): void
@external('asDOM_Document', 'getUrl')
export declare function getUrl(id: usize): string
@external('asDOM_Document', 'setDocument')
export declare function setDocument(id: usize): void
@external('asDOM_Document', 'setElement')
export declare function setElement(docId: usize, elId: usize, tag: string): void
@external('asDOM_Document', 'documentHasBody')
export declare function documentHasBody(doc: usize): boolean
@external('asDOM_Document', 'createTextNode')
export declare function createTextNode(docId: usize, textId: usize, data: string): void
@external('asDOM_Node', 'nodeAppendChild')
export declare function nodeAppendChild(parentId: usize, childId: usize): void
@external('asDOM_Node', 'nodeRemoveChild')
export declare function nodeRemoveChild(parentId: usize, childId: usize): void
@external('asDOM_Node', 'getParentNode')
export declare function getParentNode(id: usize): i32
@external('asDOM_Node', 'getParentElement')
export declare function getParentElement(id: usize): i32
@external('asDOM_Node', 'getFirstChild')
export declare function getFirstChild(id: usize): i32
@external('asDOM_Node', 'getLastChild')
export declare function getLastChild(id: usize): i32
@external('asDOM_Node', 'getNextSibling')
export declare function getNextSibling(id: usize): i32
@external('asDOM_Node', 'getPreviousSibling')
export declare function getPreviousSibling(id: usize): i32
@external('asDOM_Node', 'cloneNode')
export declare function cloneNode(id: usize, deep?: boolean): i32
@external('asDOM_Node', 'getChildNodes')
export declare function getChildNodes(nodeId: usize, listId: usize): void
@external('asDOM_Element', 'getTagName')
export declare function getTagName(id: usize): string
@external('asDOM_Element', 'elSetAttribute')
export declare function elSetAttribute(id: usize, attr: string, value: string | null): void
@external('asDOM_Element', 'elGetAttribute')
export declare function elGetAttribute(id: usize, attr: string): string | null
@external('asDOM_Element', 'setInnerHTML')
export declare function setInnerHTML(id: usize, value: string | null): void
@external('asDOM_Element', 'getInnerHTML')
export declare function getInnerHTML(id: usize): string
@external('asDOM_Element', 'elSetInnerText')
export declare function elSetInnerText(id: usize, value: string | null): void
@external('asDOM_Element', 'elGetInnerText')
export declare function elGetInnerText(id: usize): string
@external('asDOM_Element', 'getChildren')
export declare function getChildren(nodeId: usize, listId: usize): void
@external('asDOM_Element', 'getFirstElementChild')
export declare function getFirstElementChild(id: usize): i32
@external('asDOM_Element', 'getLastElementChild')
export declare function getLastElementChild(id: usize): i32
@external('asDOM_Element', 'getNextElementSibling')
export declare function getNextElementSibling(id: usize): i32
@external('asDOM_Element', 'getPreviousElementSibling')
export declare function getPreviousElementSibling(id: usize): i32
@external('asDOM_Element', 'elClick')
export declare function elClick(id: usize): void
@external('asDOM_Element', 'elOnClick')
export declare function elOnClick(id: usize, ptr: number): void
@external('asDOM_Element', 'remove')
export declare function remove(id: usize): void
@external('asDOM_Element', 'querySelector')
export declare function querySelector(id: usize, selectors: string): i32
@external('asDOM_Element', 'querySelectorAll')
export declare function querySelectorAll(id: usize, selectors: string): i32
@external('asDOM_Element', 'getShadowRoot')
export declare function getShadowRoot(id: usize): i32
@external('asDOM_Element', 'attachShadow')
export declare function attachShadow(id: usize, rootId: usize, mode: string): i32
@external('asDOM_Audio', 'initAudio')
export declare function initAudio(id: usize, src: string): void
@external('asDOM_Audio', 'pauseAudio')
export declare function pauseAudio(id: usize): void
@external('asDOM_Audio', 'playAudio')
export declare function playAudio(id: usize): void
@external('asDOM_Audio', 'getAutoplay')
export declare function getAutoplay(id: usize): boolean
@external('asDOM_Audio', 'setAutoplay')
export declare function setAutoplay(id: usize, toggle: boolean): void
@external('asDOM_HTMLTemplateElement', 'getContent')
export declare function getContent(id: usize, fragId: usize): void
@external('asDOM_NodeList', 'getLength')
export declare function getLength(id: usize): i32
@external('asDOM_NodeList', 'item')
export declare function item(id: usize, index: i32): i32
export enum ObjectType {
	unknown = 1,
	body = 2,
	div = 3,
	span = 4,
	p = 5,
	a = 6,
	script = 7,
	template = 8,
	audio = 9,
	img = 10,
	h1 = 11,
	h2 = 12,
	h3 = 13,
	h4 = 14,
	h5 = 15,
	h6 = 16,
	text = 100,
	htmlCollection = 200,
	nodeListOfNode = 201,
	nodeListOfElement = 202,
}
export class Object {
	__ptr__: usize = changetype<usize>(this)
	constructor() {
		refs.set(this.__ptr__, this)
	}
}
export function unbind(o: Object): void {
	refs.delete(o.__ptr__)
}
export const refs: Map<usize, Object> = new Map()
export class CustomElementRegistry extends Object {
	private __defs: Map<string, () => HTMLElement> = new Map()
	define(tag: string, factory: () => HTMLElement, attributes: string[]): void {
		define(this.__ptr__, tag, factory.index, attributes)
		this.__defs.set(tag, factory)
	}
	whenDefined(): void {
	}
}
export class Window extends Object {
	private __ceRegistry: CustomElementRegistry | null = null
	get customElements(): CustomElementRegistry {
		let reg = this.__ceRegistry
		if (!reg) {
			this.__ceRegistry = reg = new CustomElementRegistry()
			getCustomElements(this.__ptr__, reg.__ptr__)
		}
		return reg
	}
}
export const window = new Window()
trackWindow(window.__ptr__)
export const customElements = window.customElements
export class HTMLAudioElement extends HTMLElement {
	constructor(src: string | null = null) {
		super()
		if (src) initAudio(this.__ptr__, src)
	}
	play(): void {
		playAudio(this.__ptr__)
	}
	pause(): void {
		pauseAudio(this.__ptr__)
	}
	set autoplay(toggle: boolean) {
		setAutoplay(this.__ptr__, toggle)
	}
	get autoplay(): boolean {
		return getAutoplay(this.__ptr__) ? true : false
	}
}
export class Audio extends HTMLAudioElement {}
export class NodeList<T extends Node> extends Object {
	get length(): i32 {
		return getLength(this.__ptr__)
	}
	item(index: i32): T | null {
		const id: i32 = item(this.__ptr__, index)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as T
	}
	@operator('[]')
	private arrayRead(index: i32): T | null {
		return this.item(index)
	}
	@operator('[]=')
	private arrayWrite(index: i32, value: T): void {
		ERROR('NodeList is not writable.')
	}
	readonly [key: number]: T | null
}
): Element {
		let el: Element
		if (tag == 'body') el = new HTMLBodyElement()
		else if (tag == 'div') el = new HTMLDivElement()
		else if (tag == 'span') el = new HTMLSpanElement()
		else if (tag == 'p') el = new HTMLParagraphElement()
		else if (tag == 'a') el = new HTMLAnchorElement()
		else if (tag == 'script') el = new HTMLScriptElement()
		else if (tag == 'template') el = new HTMLTemplateElement()
		else if (tag == 'audio') el = new Audio()
		else if (tag == 'img') el = new Image()
		else if (tag == 'h1') el = new HTMLHeadingElement()
		else if (tag == 'h2') el = new HTMLHeadingElement()
		else if (tag == 'h3') el = new HTMLHeadingElement()
		else if (tag == 'h4') el = new HTMLHeadingElement()
		else if (tag == 'h5') el = new HTMLHeadingElement()
		else if (tag == 'h6') el = new HTMLHeadingElement()
		else if (tag.indexOf('-') > -1)
			throw new Error('TODO: Elements with hyphens or custom elements not supported yet.')
		else el = new HTMLUnknownElement()
		setElement(this.__ptr__, el.__ptr__, tag)
		return el
	}
	createTextNode(data: string): Text {
		const text = new Text()
		createTextNode(this.__ptr__, text.__ptr__, data)
		return text
	}
	private __children: HTMLCollection | null = null
	get children(): HTMLCollection {
		let children = this.__children
		if (!children) {
			children = new HTMLCollection()
			this.__children = children
		}
		getChildren(this.__ptr__, children.__ptr__)
		return children
	}
	get firstElementChild(): Element | null {
		const id: i32 = getFirstElementChild(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get lastElementChild(): Element | null {
		const id: i32 = getLastElementChild(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelector(selectors: string): Element | null {
		const id = querySelector(this.__ptr__, selectors)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelectorAll(selectors: string): NodeList<Element> {
		const id = querySelectorAll(this.__ptr__, selectors)
		return idToNullOrObject(id) as NodeList<Element>
	}
}
export const document = new Document()
export const DEBUG: boolean = false
export function logDebug(s: string): void {
	if (DEBUG) log('AS DEBUG: ' + s)
}
export function makeObject(type: ObjectType): Object {
	let obj: Object
	if (type == ObjectType.body) obj = new HTMLBodyElement()
	else if (type == ObjectType.div) obj = new HTMLDivElement()
	else if (type == ObjectType.span) obj = new HTMLSpanElement()
	else if (type == ObjectType.p) obj = new HTMLParagraphElement()
	else if (type == ObjectType.a) obj = new HTMLAnchorElement()
	else if (type == ObjectType.script) obj = new HTMLScriptElement()
	else if (type == ObjectType.template) obj = new HTMLTemplateElement()
	else if (type == ObjectType.audio) obj = new Audio()
	else if (type == ObjectType.img) obj = new Image()
	else if (type == ObjectType.h1) obj = new HTMLHeadingElement()
	else if (type == ObjectType.h2) obj = new HTMLHeadingElement()
	else if (type == ObjectType.h3) obj = new HTMLHeadingElement()
	else if (type == ObjectType.h4) obj = new HTMLHeadingElement()
	else if (type == ObjectType.h5) obj = new HTMLHeadingElement()
	else if (type == ObjectType.h6) obj = new HTMLHeadingElement()
	else if (type === ObjectType.unknown) obj = new HTMLUnknownElement()
	else if (type === ObjectType.text) obj = new Text()
	else if (type === ObjectType.htmlCollection) obj = new HTMLCollection()
	else if (type === ObjectType.nodeListOfNode) obj = new NodeList<Node>()
	else if (type === ObjectType.nodeListOfElement) obj = new NodeList<Element>()
	else throw new Error('Hyphenated or custom elements not yet supported.')
	return obj
}
export function idToNullOrObject(id: i32): Object | null {
	logDebug('idToNullOrObject, ' + id.toString())
	if (id == 0) {
		logDebug('idToNullOrObject returning null')
		return null
	}
	else if (id < 0) {
		logDebug('idToNullOrObject id < 0')
		const obj = makeObject(-id)
		trackNextRef(obj.__ptr__)
		return obj
	}
	else {
		logDebug('idToNullOrObject got reference ID: ' + id.toString())
		return refs.get(id) as Object
	}
}
export class ShadowRoot extends DocumentFragment {
	set innerHTML(str: string) {
		setInnerHTML(this.__ptr__, str)
	}
	get innerHTML(): string {
		return getInnerHTML(this.__ptr__)
	}
}
export abstract class Element extends Node {
	get nodeType(): i32 {
		return 1
	}
	get tagName(): string {
		return getTagName(this.__ptr__)
	}
	setAttribute(attr: string, value: string | null): void {
		elSetAttribute(this.__ptr__, attr, value)
	}
	getAttribute(attr: string): string | null {
		return elGetAttribute(this.__ptr__, attr)
	}
	get innerHTML(): string {
		return getInnerHTML(this.__ptr__)
	}
	set innerHTML(value: string | null) {
		setInnerHTML(this.__ptr__, value)
	}
	get innerText(): string {
		return elGetInnerText(this.__ptr__)
	}
	set innerText(value: string | null) {
		elSetInnerText(this.__ptr__, value)
	}
	private __children: HTMLCollection | null = null
	get children(): HTMLCollection {
		let children = this.__children
		if (!children) {
			children = new HTMLCollection()
			this.__children = children
		}
		getChildren(this.__ptr__, children.__ptr__)
		return children
	}
	get firstElementChild(): Element | null {
		const id: i32 = getFirstElementChild(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get lastElementChild(): Element | null {
		const id: i32 = getLastElementChild(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get nextElementSibling(): Element | null {
		const id: i32 = getNextElementSibling(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get previousElementSibling(): Element | null {
		const id: i32 = getPreviousElementSibling(this.__ptr__)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	click(): void {
		elClick(this.__ptr__)
	}
	set onclick(cb: () => void) {
		elOnClick(this.__ptr__, cb.index)
	}
	remove(): void {
		remove(this.__ptr__)
	}
	querySelector(selectors: string): Element | null {
		const id = querySelector(this.__ptr__, selectors)
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelectorAll(selectors: string): NodeList<Element> {
		const id = querySelectorAll(this.__ptr__, selectors)
		return idToNullOrObject(id) as NodeList<Element>
	}
	private __shadowRoot: ShadowRoot | null = null
	get shadowRoot(): ShadowRoot | null {
		return this.__shadowRoot
	}
	attachShadow(options: ShadowRootInit): ShadowRoot {
		const root = new ShadowRoot()
		attachShadow(this.__ptr__, root.__ptr__, options.mode)
		if (options.mode == 'open') this.__shadowRoot = root
		return root
	}
}
export class ShadowRootInit {
	mode: string
}
export abstract class HTMLElement extends Element {
	static observedAttributes: string[] = []
	connectedCallback(): void {}
	disconnectedCallback(): void {}
	adoptedCallback(): void {}
	attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {}
}
export function asdom_connectedCallback(id: usize): void {
	const el = refs.get(id) as HTMLElement
	el.connectedCallback()
}
export function asdom_disconnectedCallback(id: usize): void {
	const el = refs.get(id) as HTMLElement
	el.disconnectedCallback()
}
export function asdom_adoptedCallback(id: usize): void {
	const el = refs.get(id) as HTMLElement
	el.disconnectedCallback()
}
export function asdom_attributeChangedCallback(
	id: usize,
	name: string,
	oldValue: string | null,
	newValue: string | null,
): void {
	const el = refs.get(id) as HTMLElement
	el.attributeChangedCallback(name, oldValue, newValue)
}
export const idof_Arrayi32 = idof<Array<i32>>()
export function start(): void {
  const el = document.createElement("h1");
  el.setAttribute("foo", "bar");
  const s: string = el.getAttribute("foo")!;
  el.innerHTML =  `
  <span style="font-weight: normal; position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%)">
    <em>hello</em> from <strong>AssemblyScript</strong>
  </span>
`;
  document.body!.appendChild(el);
}
export function add(a: i32, b: i32): i32 {
  return a + b;
}

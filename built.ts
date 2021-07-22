// @ts-expect-error
@external('asDOM', 'trackNextRef')
export declare function trackNextRef(id: usize): void
// @ts-expect-error
@external('asDOM', 'log')
export declare function log(msg: string): void
// @ts-expect-error
@external('asDOM_Window', 'getCustomElements')
export declare function getCustomElements(id: usize, ceId: usize): void
// @ts-expect-error
@external('asDOM_Window', 'trackWindow')
export declare function trackWindow(id: usize): void
// @ts-expect-error
@external('asDOM_CustomElementRegistry', 'define')
export declare function define(id: usize, tag: string, factoryIndex: i32, attributes: string[]): void
// @ts-expect-error
@external('asDOM_Document', 'getUrl')
export declare function getUrl(id: usize): string
// @ts-expect-error
@external('asDOM_Document', 'setDocument')
export declare function setDocument(id: usize): void
// @ts-expect-error
@external('asDOM_Document', 'setElement')
export declare function setElement(docId: usize, elId: usize, tag: string): void
// @ts-expect-error
@external('asDOM_Document', 'documentHasBody')
export declare function documentHasBody(doc: usize): boolean
// @ts-expect-error
@external('asDOM_Document', 'createTextNode')
export declare function createTextNode(docId: usize, textId: usize, data: string): void
// // @ts-expect-error
// @external('asDOM_Document', 'trackNextElement')
// export declare function trackNextElement(docId: usize, elId: usize): void
// @ts-expect-error
@external('asDOM_Node', 'nodeAppendChild')
export declare function nodeAppendChild(parentId: usize, childId: usize): void
// @ts-expect-error
@external('asDOM_Node', 'nodeRemoveChild')
export declare function nodeRemoveChild(parentId: usize, childId: usize): void
// @ts-expect-error
@external('asDOM_Node', 'getParentNode')
export declare function getParentNode(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'getParentElement')
export declare function getParentElement(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'getFirstChild')
export declare function getFirstChild(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'getLastChild')
export declare function getLastChild(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'getNextSibling')
export declare function getNextSibling(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'getPreviousSibling')
export declare function getPreviousSibling(id: usize): i32
// @ts-expect-error
@external('asDOM_Node', 'cloneNode')
export declare function cloneNode(id: usize, deep?: boolean): i32
// @ts-expect-error
@external('asDOM_Node', 'getChildNodes')
export declare function getChildNodes(nodeId: usize, listId: usize): void
// @ts-expect-error
@external('asDOM_Element', 'getTagName')
export declare function getTagName(id: usize): string
// @ts-expect-error
@external('asDOM_Element', 'elSetAttribute')
export declare function elSetAttribute(id: usize, attr: string, value: string | null): void
// @ts-expect-error
@external('asDOM_Element', 'elGetAttribute')
export declare function elGetAttribute(id: usize, attr: string): string | null
// @ts-expect-error
@external('asDOM_Element', 'setInnerHTML')
export declare function setInnerHTML(id: usize, value: string | null): void
// @ts-expect-error
@external('asDOM_Element', 'getInnerHTML')
export declare function getInnerHTML(id: usize): string
// @ts-expect-error
@external('asDOM_Element', 'elSetInnerText')
export declare function elSetInnerText(id: usize, value: string | null): void
// @ts-expect-error
@external('asDOM_Element', 'elGetInnerText')
export declare function elGetInnerText(id: usize): string
// @ts-expect-error
@external('asDOM_Element', 'getChildren')
export declare function getChildren(nodeId: usize, listId: usize): void
// @ts-expect-error
@external('asDOM_Element', 'getFirstElementChild')
export declare function getFirstElementChild(id: usize): i32
// @ts-expect-error
@external('asDOM_Element', 'getLastElementChild')
export declare function getLastElementChild(id: usize): i32
// @ts-expect-error
@external('asDOM_Element', 'getNextElementSibling')
export declare function getNextElementSibling(id: usize): i32
// @ts-expect-error
@external('asDOM_Element', 'getPreviousElementSibling')
export declare function getPreviousElementSibling(id: usize): i32
// @ts-expect-error
@external('asDOM_Element', 'elClick')
export declare function elClick(id: usize): void
// @ts-expect-error
@external('asDOM_Element', 'elOnClick')
export declare function elOnClick(id: usize, ptr: number): void
// @ts-expect-error
@external('asDOM_Element', 'remove')
export declare function remove(id: usize): void
// @ts-expect-error
@external('asDOM_Element', 'querySelector')
export declare function querySelector(id: usize, selectors: string): i32
// @ts-expect-error
@external('asDOM_Element', 'querySelectorAll')
export declare function querySelectorAll(id: usize, selectors: string): i32
// @ts-expect-error
@external('asDOM_Element', 'getShadowRoot')
export declare function getShadowRoot(id: usize): i32
// @ts-expect-error
@external('asDOM_Element', 'attachShadow')
export declare function attachShadow(id: usize, rootId: usize, mode: string): i32
// @ts-expect-error
@external('asDOM_Audio', 'initAudio')
export declare function initAudio(id: usize, src: string): void
// @ts-expect-error
@external('asDOM_Audio', 'pauseAudio')
export declare function pauseAudio(id: usize): void
// @ts-expect-error
@external('asDOM_Audio', 'playAudio')
export declare function playAudio(id: usize): void
// @ts-expect-error
@external('asDOM_Audio', 'getAutoplay')
export declare function getAutoplay(id: usize): boolean
// @ts-expect-error
@external('asDOM_Audio', 'setAutoplay')
export declare function setAutoplay(id: usize, toggle: boolean): void
// @ts-expect-error
@external('asDOM_HTMLTemplateElement', 'getContent')
export declare function getContent(id: usize, fragId: usize): void
// @ts-expect-error
@external('asDOM_NodeList', 'getLength')
export declare function getLength(id: usize): i32
// @ts-expect-error
@external('asDOM_NodeList', 'item')
export declare function item(id: usize, index: i32): i32
// TODO Put this in a file shared between glue code and AS code. We need to
// convert the glue code to TypeScript first, or compile the shared file to
// plain JS.
export enum ObjectType {
	// 0 is intentionally skipped, do not use 0
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
	// Text nodes
	text = 100,
	// Node lists
	htmlCollection = 200,
	nodeListOfNode = 201,
	nodeListOfElement = 202,
}
/**
 * The base class that all objects extend from.
 */
export class Object {
	__ptr__: usize = changetype<usize>(this)
	constructor() {
		refs.set(this.__ptr__, this)
	}
}
/**
 * Call this function when you are finished using an object. After calling
 * this, it should never be used again, or the DOM bindings may fail to work
 * properly.
 */
export function unbind(o: Object): void {
	refs.delete(o.__ptr__)
}
// This is for asdom's internal use only.
export const refs: Map<usize, Object> = new Map()
/*
 * Custom elements are a bit too dynamic to easily map an interface to them
 * one-to-one like with other DOM APIs, so currently we have to write a bit more
 * of a custom implementation in AssemblyScript. There are some differences in
 * the final API:
 *
 * - In AS we cannot reference constructors, so the second arg to
 *   customElements.define() is currently a factory function that returns an
 *   instance of your custom element class.
 */
export class CustomElementRegistry extends Object {
	private __defs: Map<string, () => HTMLElement> = new Map()
	define(tag: string, factory: () => HTMLElement, attributes: string[]): void {
		define(this.__ptr__, tag, factory.index, attributes)
		this.__defs.set(tag, factory)
	}
	whenDefined(): void {
		// TODO
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
// TODO Perhaps put these on a new `window` object, to make it more like on the JS side.
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
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as T | null
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
	// This makes TypeScript happy.
	// The name must be "key" in AS (can be anything in TS). Open issue: https://github.com/AssemblyScript/assemblyscript/issues/1972
	readonly [key: number]: T | null
}
/** Node types: https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeType */
enum NodeType {
	ELEMENT_NODE = 1,
	ATTRIBUTE_NODE = 2,
	TEXT_NODE = 3,
	CDATA_SECTION_NODE = 4,
	// 5 and 6 are deprecated and skipped.
	PROCESSING_INSTRUCTION_NODE = 7,
	COMMENT_NODE = 8,
	DOCUMENT_NODE = 9,
	DOCUMENT_TYPE_NODE = 10,
	DOCUMENT_FRAGMENT_NODE = 11,
	// 12 is deprecated and skipped.
}
export abstract class Node extends Object {
	static ELEMENT_NODE: NodeType = NodeType.ELEMENT_NODE
	static ATTRIBUTE_NODE: NodeType = NodeType.ATTRIBUTE_NODE
	static TEXT_NODE: NodeType = NodeType.TEXT_NODE
	static CDATA_SECTION_NODE: NodeType = NodeType.CDATA_SECTION_NODE
	static PROCESSING_INSTRUCTION_NODE: NodeType = NodeType.PROCESSING_INSTRUCTION_NODE
	static COMMENT_NODE: NodeType = NodeType.COMMENT_NODE
	static DOCUMENT_NODE: NodeType = NodeType.DOCUMENT_NODE
	static DOCUMENT_TYPE_NODE: NodeType = NodeType.DOCUMENT_TYPE_NODE
	static DOCUMENT_FRAGMENT_NODE: NodeType = NodeType.DOCUMENT_FRAGMENT_NODE
	appendChild<T extends Node>(child: T): T {
		nodeAppendChild(this.__ptr__, child.__ptr__)
		return child
	}
	removeChild<T extends Node>(child: T): T {
		nodeRemoveChild(this.__ptr__, child.__ptr__)
		return child
	}
	abstract get nodeType(): NodeType
	get parentNode(): Node | null {
		const id: i32 = getParentNode(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	get parentElement(): Node | null {
		const id: i32 = getParentElement(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	get firstChild(): Node | null {
		const id: i32 = getFirstChild(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	get lastChild(): Node | null {
		const id: i32 = getLastChild(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	get nextSibling(): Node | null {
		const id: i32 = getNextSibling(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	get previousSibling(): Node | null {
		const id: i32 = getPreviousSibling(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Node
	}
	cloneNode(deep: boolean = false): Node {
		const id: i32 = cloneNode(this.__ptr__, deep)
		return idToNullOrObject(id) as Node // The result must not be null if we just cloned a Node.
	}
	private __childNodes: NodeList<Node> | null = null
	get childNodes(): NodeList<Node> {
		let childNodes = this.__childNodes
		if (!childNodes) {
			childNodes = new NodeList()
			this.__childNodes = childNodes
		}
		getChildNodes(this.__ptr__, childNodes.__ptr__)
		return childNodes
	}
}
export class DocumentFragment extends Node {
	get nodeType(): i32 {
		return 11
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
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get lastElementChild(): Element | null {
		const id: i32 = getLastElementChild(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelector(selectors: string): Element | null {
		const id = querySelector(this.__ptr__, selectors)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelectorAll(selectors: string): NodeList<Element> {
		const id = querySelectorAll(this.__ptr__, selectors)
		return idToNullOrObject(id) as NodeList<Element>
	}
}
export class HTMLTemplateElement extends HTMLElement {
	private __frag: DocumentFragment | null = null
	get content(): DocumentFragment {
		let frag = this.__frag
		if (!frag) {
			frag = new DocumentFragment()
			this.__frag = frag
		}
		getContent(this.__ptr__, frag.__ptr__)
		return frag
	}
}
// We can move any of these into their own file if/when they need custom implementation.
export class HTMLBodyElement extends HTMLElement {}
export class HTMLDivElement extends HTMLElement {}
export class HTMLSpanElement extends HTMLElement {}
export class HTMLParagraphElement extends HTMLElement {}
export class HTMLAnchorElement extends HTMLElement {}
export class HTMLScriptElement extends HTMLElement {}
export class HTMLImageElement extends HTMLElement {}
export class Image extends HTMLImageElement {}
export class HTMLHeadingElement extends HTMLElement {}
export class HTMLUnknownElement extends HTMLElement {}
export class SVGElement extends Element {}
export class SVGSVGElement extends SVGElement {}
// ...TODO...
/**
 * The CharacterData abstract interface represents a Node object that contains
 * characters. This is an abstract interface, meaning there aren't any object of
 * type CharacterData: it is implemented by other interfaces, like Text,
 * Comment, or ProcessingInstruction which aren't abstract.
 */
export abstract class CharacterData extends Node {
	// data: string
	// readonly length: number
	// readonly ownerDocument: Document
	// appendData(data: string): void
	// deleteData(offset: number, count: number): void
	// insertData(offset: number, data: string): void
	// replaceData(offset: number, count: number, data: string): void
	// substringData(offset: number, count: number): string
}
export class Text extends CharacterData {
	get nodeType(): i32 {
		return 3
	}
	/**
	 * Returns the combined data of all direct Text node siblings.
	 */
	// readonly wholeText: string;
	/**
	 * Splits data at the given offset and returns the remainder as Text node.
	 */
	// splitText(offset: number): Text;
}
export class HTMLCollection extends Object {
	get length(): i32 {
		return getLength(this.__ptr__)
	}
	item(index: i32): Element | null {
		const id: i32 = item(this.__ptr__, index)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	@operator('[]')
	private arrayRead(index: i32): Element | null {
		return this.item(index)
	}
	@operator('[]=')
	private arrayWrite(index: i32, value: Element): void {
		ERROR('NodeList is not writable.')
	}
	// This makes TypeScript happy.
	// The name must be "key" in AS (can be anything in TS). Open issue: https://github.com/AssemblyScript/assemblyscript/issues/1972
	readonly [key: number]: Element | null
}
export class Document extends Node {
	get nodeType(): i32 {
		return 9
	}
	constructor() {
		super()
		setDocument(this.__ptr__)
	}
	get URL(): string {
		return getUrl(this.__ptr__)
	}
	// @ts-expect-error
	get body(): HTMLBodyElement | null {
		let el: HTMLBodyElement
		if (documentHasBody(this.__ptr__)) {
			el = new HTMLBodyElement()
			setElement(this.__ptr__, el.__ptr__, 'body')
		} else {
			return null
		}
		return el
	}
	set body(el: HTMLBodyElement) {
		throw ERROR('TODO: document.body setter is not implemented yet.')
	}
	createElement(tag: string /*, TODO options */): Element {
		let el: Element
		// Don't forget to add Elements here so they can be created with `document.createElement`.
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
	// TODO, for SVG elements.
	// createElementNS(ns, name, options) { }
	/**
	 * Creates a text string from the specified value.
	 * @param data String that specifies the nodeValue property of the text node.
	 */
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
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get lastElementChild(): Element | null {
		const id: i32 = getLastElementChild(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	querySelector(selectors: string): Element | null {
		const id = querySelector(this.__ptr__, selectors)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Node | null
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
	// Elements
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
	// Text nodes
	else if (type === ObjectType.text) obj = new Text()
	// Node lists
	else if (type === ObjectType.htmlCollection) obj = new HTMLCollection()
	else if (type === ObjectType.nodeListOfNode) obj = new NodeList<Node>()
	else if (type === ObjectType.nodeListOfElement) obj = new NodeList<Element>()
	// Anything else
	else throw new Error('Hyphenated or custom elements not yet supported.')
	return obj
}
// Use this only for APIs that return Object or Object|null!
export function idToNullOrObject(id: i32): Object | null {
	logDebug('idToNullOrObject, ' + id.toString())
	// if null, it means there is no element on the JS-side.
	if (id == 0) {
		logDebug('idToNullOrObject returning null')
		return null
	}
	// If negative, there is an element on the JS-side that doesn't have a
	// corresponding AS-side instance yet. In this case we need to
	// create a new instance based on its type.
	else if (id < 0) {
		logDebug('idToNullOrObject id < 0')
		const obj = makeObject(-id)
		// Associate the AS-side instance with the JS-side instance.
		// TODO use this.ownerDocument.__ptr__ instead of document.__ptr__
		// trackNextElement(document.__ptr__, el.__ptr__)
		trackNextRef(obj.__ptr__)
		return obj
	}
	// If we reach here then there is already an AS-side instance
	// associated with a JS-side instance, and the JS side gave us the ID
	// (pointer) of our AS-side object to return. We might reach here, for
	// example, if we use appendChild to pass an existing child within AS
	// instead of using innerHTML. By using innerHTML and sending a string
	// to JS, it can create a whole tree but none of those nodes will be
	// tracked. Finally, if we do try to access them, we lazily associate
	// new AS-side objects in the previous conditional block.
	else {
		logDebug('idToNullOrObject got reference ID: ' + id.toString())
		return refs.get(id) as Object // It must be a Object. Use this function only for APIs that return Object or Object|null.
	}
}
export class ShadowRoot extends DocumentFragment {
	// This is non-standard for ShadowRoot, but every browser has it.
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
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get lastElementChild(): Element | null {
		const id: i32 = getLastElementChild(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get nextElementSibling(): Element | null {
		const id: i32 = getNextElementSibling(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
		const result = idToNullOrObject(id)
		if (!result) return null
		else return result as Element
	}
	get previousElementSibling(): Element | null {
		const id: i32 = getPreviousElementSibling(this.__ptr__)
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
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
		// TODO restore after issue is fixed: https://github.com/AssemblyScript/assemblyscript/issues/1976
		// return idToNullOrObject(id) as Element | null
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
	// The following are for use by custom elements, but not required to be
	// implemented so not abstract. {{{
	static observedAttributes: string[] = []
	connectedCallback(): void {}
	disconnectedCallback(): void {}
	adoptedCallback(): void {}
	attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {}
	// }}}
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
const el = document.createElement("h1");
el.setAttribute("foo", "bar");
const s: string = el.getAttribute("foo")!; // returns "bar"
el.innerHTML = /*html*/ `
  <span style="font-weight: normal; position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%)">
    <em>hello</em> from <strong>AssemblyScript</strong>
  </span>
`;
document.body!.appendChild(el);
export function add(a: i32, b: i32): i32 {
  return a + b;
}

  
//@ts-check
import { instantiateStreaming } from "@assemblyscript/loader";

/**
 * @param {string} url
 * @returns {Promise<WebAssembly.Module>}
 */
export async function compile(url) {
  const respP = fetch(url);
  if ("compileStreaming" in WebAssembly) {
    return WebAssembly.compileStreaming(respP);
  }

  const buffer = await respP.then(r => r.arrayBuffer());
  return WebAssembly.compile(buffer);
}

/**
 * @param {string} url
 * @param {Record<string, Record<string, WebAssembly.ImportValue>>} [importObject]
 * @returns {Promise<WebAssembly.Instance>}
 */
export async function instantiate(url, importObject) {
  return instantiateStreaming(fetch(url), importObject);
}
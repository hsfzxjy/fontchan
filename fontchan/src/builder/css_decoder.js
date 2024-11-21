"use strict";
(function () {
  const global = globalThis || window;
  const origin = global.location ? location.origin : global.$fontchanOrigin;
  const textEncoder = new TextEncoder();
  const wasmDataUrl = 'data:application/wasm;base64,"{%WASM_BASE64%}"';
  const fontSpecs = "{%FONT_SPECS%}";
  for (const spec of fontSpecs) {
    spec.src_template = spec.src.replace(/<ORIGIN>/g, origin).split("<FID>");
  }
  async function getWasmAB() {
    if (getWasmAB.__cache) return getWasmAB.__cache;
    const ab = await (await fetch(wasmDataUrl)).arrayBuffer();
    getWasmAB.__cache = ab;
    return ab;
  }
  function copyStr2M(str, m, ptr) {
    const buf = new Uint8Array(m.buffer, ptr);
    const { written } = textEncoder.encodeInto(str, buf);
    return written + ptr;
  }
  function makeEnv(m) {
    return {
      js_write_font_face_ext(fontid, writer) {
        return copyStr2M(fontSpecs[fontid].ext, m.memory, writer);
      },
      js_write_font_face_src(fontid, hashptr, hashlen, writer) {
        const t = fontSpecs[fontid].src_template;
        writer = copyStr2M(t[0], m.memory, writer);
        const u8arr = new Uint8Array(m.memory.buffer);
        for (let i = 1; i < t.length; i++) {
          u8arr.copyWithin(writer, hashptr, hashptr + hashlen);
          writer += hashlen;
          writer = copyStr2M(t[i], m.memory, writer);
        }
        return writer;
      },
    };
  }
  async function decodeCss() {
    const m = { memory: null };
    const env = makeEnv(m);
    const wasm = await WebAssembly.instantiate(await getWasmAB(), { env });
    const exports = wasm.instance.exports;
    m.memory = exports.memory;
    while (1) {
      try {
        const writer = exports.init_writer();
        const writerEnd = exports.decode_css(fontSpecs.length, writer);
        return new DataView(m.memory.buffer, writer, writerEnd - writer);
      } catch (e) {
        if (
          e instanceof WebAssembly.RuntimeError &&
          e.message === "memory access out of bounds"
        ) {
          exports.memory.grow(4);
          console.error(e);
        } else throw e;
      }
    }
  }
  global.$fontchanDecodeCss = decodeCss;
  async function injectCss() {
    if (injectCss.__started) return;
    injectCss.__started = true;
    const cssData = await decodeCss();
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = URL.createObjectURL(new Blob([cssData], { type: "text/css" }));
    document.head.appendChild(link);
  }
  global.$fontchanInjectCss = injectCss;
})();

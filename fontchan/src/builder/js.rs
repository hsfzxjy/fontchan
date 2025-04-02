use anyhow::Result;
use base64ct::Encoding;
use fontchan_unicode::URange;
use fontchan_util::{AtomicPath, Digester};
use serde::{Deserialize, Serialize};
use serde_json::json;

use std::borrow::Cow;

use crate::paramdef;

use super::font::BuildResults as FontResults;
use super::tmpl::Tmpl;

static JS_TEMPLATE: Tmpl<JSTmplParams> = Tmpl::new_str(include_str!("css_decoder.js"));

paramdef!(
    JSTmplParams,
    js_params,
    wasm_base64 = "\"{%WASM_BASE64%}\"",
    font_specs = "\"{%FONT_SPECS%}\"",
    sha = "\"{%SHA%}\""
);

#[derive(Serialize, Deserialize, Debug)]
pub struct CSSFragments<'a> {
    pub ext: Cow<'a, str>,
    pub src: Cow<'a, str>,
}

pub struct Builder;

impl Builder {
    pub fn build<'f, 'r>(
        &self,
        dest: AtomicPath,
        fragments: impl Iterator<Item = &'f CSSFragments<'f>>,
        ranges: impl Iterator<Item = &'r URange>,
        font_results: &FontResults,
    ) -> Result<()> {
        use fontchan_codec::*;

        let range_data = encode_urange_data(ranges)?;
        let fid_data = encode_fid_data(font_results.entry_minor_iter().map(|r| r.fid.as_str()))?;

        let fragments = fragments.collect::<Vec<_>>();
        let estimated_heap_size = fontchan_codec::StdContext {
            writer: fontchan_codec::CounterWriter::new(),
            urange_data: &range_data,
            fid_data: &fid_data,
            faces: fragments.as_slice(),
            ext_getter: |f| f.ext.as_bytes(),
            src_getter: |f, _| f.src.as_bytes(),
        }
        .decode()
        .value()
            + 65536;

        let wasm_bin =
            fontchan_wasm_mutate::get_wasm_binary(&range_data, &fid_data, estimated_heap_size);
        let font_specs_bin = json!(fragments).to_string();

        let sha = Digester::new()
            .push(&wasm_bin)
            .push(font_specs_bin.as_bytes())
            .base64_result();

        let js = JS_TEMPLATE.render(&js_params!(
            wasm_base64 = base64ct::Base64::encode_string(&wasm_bin),
            font_specs = font_specs_bin,
            sha = sha.as_ref()
        ));

        let dest = dest.into_writable()?;
        std::fs::write(dest.as_ref(), js.as_bytes())?;
        dest.commit()?;
        Ok(())
    }
}

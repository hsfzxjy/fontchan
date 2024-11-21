use anyhow::Result;
use fontchan_unicode::URange;

pub fn encode_urange_data<'a>(partition: impl Iterator<Item = &'a URange>) -> Result<Vec<u8>> {
    use integer_encoding::VarIntWriter;
    let mut out = vec![];
    for urange in partition {
        let single_count = urange.single_count();
        out.write_varint(single_count as u32)?;
        let mut prev = 0;
        for range in &urange.as_ref()[..single_count] {
            let codepoint = range.start as u32;
            out.write_varint(codepoint - prev)?;
            prev = codepoint;
        }
        let multi_count = urange.multi_count();
        out.write_varint(multi_count as u32)?;
        prev = 0;
        for range in &urange.as_ref()[single_count..] {
            out.write_varint(range.start as u32 - prev)?;
            out.write_varint(range.end as u32 - range.start as u32)?;
            prev = range.end as u32;
        }
    }
    Ok(out)
}

pub fn encode_fid_data<'a>(fids: impl Iterator<Item = &'a str>) -> Result<Vec<u8>> {
    use integer_encoding::VarIntWriter;
    let mut out = vec![];
    for fid in fids {
        out.write_varint(fid.len() as u32)?;
        out.extend_from_slice(fid.as_bytes());
    }
    Ok(out)
}

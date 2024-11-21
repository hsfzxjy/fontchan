use std::{cell::RefCell, collections::HashMap, ops::IndexMut};

use wast::{
    core::{Data, DataVal, GlobalKind, Instruction, Limits, MemoryKind, ModuleField, ModuleKind},
    token::Index,
    Wat,
};

static ORIGINAL_WAT: &str = include_str!(concat!(env!("OUT_DIR"), "/decoder.wat"));

macro_rules! matchopt {
    ($expression:expr, $pattern:pat $(if $guard:expr)?, $out:expr) => {
        match $expression {
            $pattern $(if $guard)? => Some($out),
            _ => None
        }
    };
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
enum FieldQuery {
    Memory,
    ExportedGlobal(&'static str),
    Exported(&'static str),
    Global(u32),
    Data,
}

impl FieldQuery {
    fn resolve_index(self, fields: &Vec<ModuleField>) -> Option<usize> {
        match self {
            FieldQuery::Memory => fields
                .iter()
                .position(|field| matches!(field, ModuleField::Memory(_))),

            FieldQuery::Exported(name) => fields.iter().position(|field| {
                matches!(
                   field, ModuleField::Export(e) if e.name == name
                )
            }),
            FieldQuery::ExportedGlobal(name) => {
                let exported_index = Self::Exported(name).resolve_index(fields)?;
                let item = matchopt!(&fields[exported_index], ModuleField::Export(e), e.item)?;
                let id = matchopt!(item, Index::Num(id, _), id)?;
                Self::Global(id).resolve_index(fields)
            }
            FieldQuery::Global(id) => fields
                .iter()
                .enumerate()
                .filter(|(_, field)| matches!(field, ModuleField::Global(_)))
                .nth(id as usize)
                .map(|(index, _)| index),
            FieldQuery::Data => fields
                .iter()
                .enumerate()
                .filter(|(_, field)| matches!(field, ModuleField::Data(_)))
                .nth(1)
                .map(|(index, _)| index),
        }
    }
}

struct Fields<'a> {
    wat: Wat<'a>,
    index: RefCell<HashMap<FieldQuery, usize>>,
}

impl<'a> Fields<'a> {
    fn to_index(&self, query: FieldQuery) -> usize {
        *self.index.borrow_mut().entry(query).or_insert_with(|| {
            query.resolve_index(self.as_ref()).unwrap_or_else(|| {
                panic!("Field not found: {:?}", query);
            })
        })
    }
    fn get(&self, query: FieldQuery) -> &ModuleField {
        &self.as_ref()[self.to_index(query)]
    }
    fn get_mut<'b>(&'b mut self, query: FieldQuery) -> &'b mut ModuleField<'a> {
        let index = self.to_index(query);
        self.as_mut().index_mut(index)
    }
}

impl<'a> Fields<'a> {
    fn new(wat: Wat<'a>) -> Self {
        Self {
            wat,
            index: RefCell::new(HashMap::new()),
        }
    }
}

impl<'a> AsRef<Vec<ModuleField<'a>>> for Fields<'a> {
    fn as_ref(&self) -> &Vec<ModuleField<'a>> {
        let kind = matchopt!(&self.wat, Wat::Module(m), &m.kind).unwrap();
        matchopt!(kind, ModuleKind::Text(fields), fields).unwrap()
    }
}

impl<'a> AsMut<Vec<ModuleField<'a>>> for Fields<'a> {
    fn as_mut(&mut self) -> &mut Vec<ModuleField<'a>> {
        let kind = matchopt!(&mut self.wat, Wat::Module(m), &mut m.kind).unwrap();
        matchopt!(kind, ModuleKind::Text(fields), fields).unwrap()
    }
}

struct FieldAccessor<F: ?Sized>(F);

trait FieldAccess {
    fn access(self) -> FieldAccessor<Self>;
}

impl<'a, 'f> FieldAccess for &'a ModuleField<'f> {
    fn access(self) -> FieldAccessor<Self> {
        FieldAccessor(self)
    }
}

impl<'a, 'f> FieldAccessor<&'a ModuleField<'f>> {
    fn get_global_i32(&self) -> Option<i32> {
        let kind = matchopt!(self.0, ModuleField::Global(g), &g.kind)?;
        let expr = matchopt!(kind, GlobalKind::Inline(expr), expr)?;
        matchopt!(expr.instrs.as_ref(), [Instruction::I32Const(v)], *v)
    }
}

impl<'a, 'f> FieldAccess for &'a mut ModuleField<'f> {
    fn access(self) -> FieldAccessor<Self> {
        FieldAccessor(self)
    }
}

impl<'a, 'f> FieldAccessor<&'a mut ModuleField<'f>> {
    fn get_memory_limits_mut(self) -> Option<&'a mut Limits> {
        let kind = matchopt!(self.0, ModuleField::Memory(m), &mut m.kind)?;
        matchopt!(kind, MemoryKind::Normal(typ), &mut typ.limits)
    }
    fn get_data_mut(self) -> Option<&'a mut Data<'f>> {
        matchopt!(self.0, ModuleField::Data(d), d)
    }
}

struct Mutator<'a> {
    fields: Fields<'a>,
    urange_data: &'a [u8],
    fid_data: &'a [u8],
    heap_size: usize,

    old_data_end: usize,
    urange_start_offset: usize,
    urange_len_offset: usize,
    fid_start_offset: usize,
    fid_len_offset: usize,
    heap_start_offset: usize,
}

impl<'m> Mutator<'m> {
    fn new(wat: Wat<'m>, urange_data: &'m [u8], fid_data: &'m [u8], heap_size: usize) -> Self {
        let fields = Fields::new(wat);
        let [old_data_end, urange_start_offset, urange_len_offset, fid_start_offset, fid_len_offset, heap_start_offset] =
            [
                "__data_end",
                "URANGE_START",
                "URANGE_LEN",
                "FID_START",
                "FID_LEN",
                "HEAP_START",
            ]
            .map(|name| {
                fields
                    .get(FieldQuery::ExportedGlobal(name))
                    .access()
                    .get_global_i32()
                    .unwrap() as usize
            });

        Self {
            fields,
            urange_data,
            fid_data,
            heap_size,

            heap_start_offset,
            urange_start_offset,
            urange_len_offset,
            fid_start_offset,
            fid_len_offset,

            old_data_end,
        }
    }

    fn mutate_data(&mut self) {
        let data = self
            .fields
            .get_mut(FieldQuery::Data)
            .access()
            .get_data_mut()
            .unwrap();
        let offset_expr = matchopt!(
            &data.kind,
            wast::core::DataKind::Active { offset, .. },
            offset
        )
        .unwrap();
        let base = matchopt!(offset_expr.instrs.as_ref(), [Instruction::I32Const(v)], *v).unwrap()
            as usize;
        let dataval = data.data.first_mut().unwrap();
        let old_segment = matchopt!(dataval, DataVal::String(s), s).unwrap();
        let old_segment_len = old_segment.len();
        let mut new_segment = [old_segment, self.urange_data, self.fid_data].concat();

        for (target_offset, newval) in [
            (self.urange_start_offset, base + old_segment_len),
            (self.urange_len_offset, self.urange_data.len()),
            (
                self.fid_start_offset,
                base + old_segment_len + self.urange_data.len(),
            ),
            (self.fid_len_offset, self.fid_data.len()),
            (
                self.heap_start_offset,
                base + old_segment_len + self.urange_data.len() + self.fid_data.len(),
            ),
        ] {
            let rel_offset = target_offset - base;
            new_segment[rel_offset..rel_offset + 4].copy_from_slice(&(newval as u32).to_le_bytes());
        }
        *dataval = DataVal::Integral(new_segment);
    }

    fn mutate_memory(&mut self) {
        let limits = self
            .fields
            .get_mut(FieldQuery::Memory)
            .access()
            .get_memory_limits_mut()
            .unwrap();
        let new_size =
            self.old_data_end + self.heap_size + self.urange_data.len() + self.fid_data.len();
        const PAGE_SIZE: usize = 65536;
        let new_page_size = (new_size + PAGE_SIZE - 1) / PAGE_SIZE;
        limits.min = limits.min.max(new_page_size as u64);
    }

    fn mutate(mut self) -> Wat<'m> {
        self.mutate_data();
        self.mutate_memory();
        self.fields.wat
    }
}

pub fn get_wasm_binary(urange_data: &[u8], fid_data: &[u8], heap_size: usize) -> Vec<u8> {
    let buf = wast::parser::ParseBuffer::new(ORIGINAL_WAT).unwrap();
    let mutator = Mutator::new(
        wast::parser::parse(&buf).unwrap(),
        urange_data,
        fid_data,
        heap_size,
    );
    mutator.mutate().encode().unwrap()
}

#[test]
fn test_mutate() {
    let buf = wast::parser::ParseBuffer::new(ORIGINAL_WAT).unwrap();
    let mutator = Mutator::new(
        wast::parser::parse(&buf).unwrap(),
        &[1, 2, 3],
        &[1, 2, 3],
        666666,
    );
    mutator.mutate();
}

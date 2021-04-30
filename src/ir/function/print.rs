use super::{
    super::module::name::Name,
    super::types::Types,
    super::value::Value,
    basic_block::BasicBlockId,
    data::Data,
    instruction::{Instruction, InstructionId, Opcode, Operand},
    Function,
};
use rustc_hash::FxHashMap;
use std::fmt;

pub type Index = usize;
pub type Indexes = FxHashMap<Ids, Name>;

pub struct FunctionAsmPrinter<'a, 'b: 'a> {
    fmt: &'a mut fmt::Formatter<'b>,
    indexes: Indexes,
    cur_index: Index,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Ids {
    Block(BasicBlockId),
    Inst(InstructionId),
    Arg(usize),
}

impl<'a, 'b: 'a> FunctionAsmPrinter<'a, 'b> {
    pub fn new(fmt: &'a mut fmt::Formatter<'b>) -> Self {
        Self {
            fmt,
            indexes: FxHashMap::default(),
            cur_index: 0,
        }
    }

    pub fn print(&mut self, f: &Function) -> fmt::Result {
        if f.is_prototype {
            write!(self.fmt, "declare ")?
        } else {
            write!(self.fmt, "define ")?
        }

        write!(self.fmt, "{:?} ", f.linkage)?;
        write!(self.fmt, "{:?} ", f.preemption_specifier)?;
        write!(self.fmt, "{:?} ", f.visibility)?;
        for attr in &f.ret_attrs {
            write!(self.fmt, "{:?} ", attr)?
        }
        write!(self.fmt, "{} ", f.types.to_string(f.result_ty))?;
        write!(self.fmt, "@{}(", f.name)?;

        for (i, param) in f.params.iter().enumerate() {
            write!(self.fmt, "{} ", f.types.to_string(param.ty))?;
            for attr in &param.attrs {
                write!(self.fmt, "{:?} ", attr)?;
            }
            match param.name.to_string() {
                Some(name) => {
                    write!(self.fmt, "%{}", name)?;
                    self.indexes.insert(Ids::Arg(i), Name::Name(name.clone()));
                }
                None => {
                    let name = self.new_name_for_arg(i);
                    write!(self.fmt, "%{:?}", name)?
                }
            }
            write!(
                self.fmt,
                "{}",
                if i == f.params.len() - 1 { "" } else { ", " }
            )?;
        }

        if f.is_var_arg {
            write!(self.fmt, ", ...")?;
        }

        write!(self.fmt, ") ")?;

        if let Some(unnamed_addr) = f.unnamed_addr {
            write!(self.fmt, "{:?} ", unnamed_addr)?
        }

        for attr in &f.func_attrs {
            write!(self.fmt, "{:?} ", attr)?
        }

        if let Some((ty, func)) = &f.personality {
            write!(
                self.fmt,
                "personality {} {} ",
                f.types.to_string(*ty),
                func.to_string(&f.types)
            )?
        }

        if f.is_prototype {
            return writeln!(self.fmt);
        }

        write!(self.fmt, "{{\n")?;

        for block_id in f.layout.block_iter() {
            if let Some(name) = &f.data.block_ref(block_id).name {
                match name.to_string() {
                    Some(name) => {
                        self.indexes
                            .insert(Ids::Block(block_id), Name::Name(name.clone()));
                    }
                    None => {
                        self.new_name_for_block(block_id);
                    }
                }
            } else {
                self.new_name_for_block(block_id);
            }

            for inst_id in f.layout.inst_iter(block_id) {
                let inst = f.data.inst_ref(inst_id);
                if matches!(
                    inst.opcode,
                    Opcode::Store | Opcode::Br | Opcode::CondBr | Opcode::Ret | Opcode::Resume
                ) || (inst.operand.call_result_ty() == Some(f.types.base().void()))
                {
                    continue;
                }
                if let Some(name) = &inst.dest {
                    match name {
                        Name::Name(name) => {
                            self.indexes
                                .insert(Ids::Inst(inst_id), Name::Name(name.clone()));
                        }
                        Name::Number(_) => {
                            self.new_name_for_inst(inst_id);
                        }
                    }
                } else {
                    self.new_name_for_inst(inst_id);
                }
            }
        }

        for block_id in f.layout.block_iter() {
            writeln!(
                self.fmt,
                "{:?}:",
                self.indexes.get(&Ids::Block(block_id)).unwrap()
            )?;

            for inst_id in f.layout.inst_iter(block_id) {
                let inst = f.data.inst_ref(inst_id);
                write!(self.fmt, "    ")?;
                self.print_inst(inst, &f.types, &f.data)?;
                writeln!(self.fmt)?;
            }
        }

        write!(self.fmt, "}}\n")
    }

    fn print_inst(&mut self, inst: &Instruction, types: &Types, data: &Data) -> fmt::Result {
        let dest = self
            .indexes
            .get(&Ids::Inst(inst.id.unwrap()))
            .unwrap_or(&Name::Number(usize::MAX));

        match &inst.operand {
            Operand::Alloca {
                tys,
                num_elements,
                align,
            } => {
                write!(
                    self.fmt,
                    "%{:?} = alloca {}, {} {}{}",
                    dest,
                    types.to_string(tys[0]),
                    types.to_string(tys[1]),
                    num_elements.to_string(types),
                    if *align > 0 {
                        format!(", align {}", align)
                    } else {
                        "".to_string()
                    }
                )
            }
            Operand::Phi { ty, args, blocks } => {
                write!(
                    self.fmt,
                    "%{:?} = phi {} {}",
                    dest,
                    types.to_string(*ty),
                    args.iter()
                        .zip(blocks.iter())
                        .fold("".to_string(), |acc, (arg, &block)| {
                            format!(
                                "{}[{}, %{:?}], ",
                                acc,
                                self.value_to_string(data.value_ref(*arg), types),
                                self.indexes[&Ids::Block(block)]
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::Load { tys, addr, align } => {
                write!(
                    self.fmt,
                    "%{:?} = load {}, {} {}, align {}",
                    dest,
                    types.to_string(tys[0]),
                    types.to_string(tys[1]),
                    self.value_to_string(data.value_ref(*addr), types),
                    align
                )
            }
            Operand::Store { tys, args, align } => {
                write!(
                    self.fmt,
                    "store {} {}, {} {}, align {}",
                    types.to_string(tys[0]),
                    self.value_to_string(data.value_ref(args[0]), types),
                    types.to_string(tys[1]),
                    self.value_to_string(data.value_ref(args[1]), types),
                    align
                )
            }
            Operand::InsertValue { tys, args } => {
                write!(
                    self.fmt,
                    "%{:?} = insertvalue {} {}, {} {}, {}",
                    dest,
                    types.to_string(tys[0]),
                    self.value_to_string(data.value_ref(args[0]), types),
                    types.to_string(tys[1]),
                    self.value_to_string(data.value_ref(args[1]), types),
                    args[2..]
                        .iter()
                        .fold("".to_string(), |acc, &arg| {
                            format!(
                                "{}{}, ",
                                acc,
                                self.value_to_string(data.value_ref(arg), types)
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::ExtractValue { ty, args } => {
                write!(
                    self.fmt,
                    "%{:?} = extractvalue {} {}, {}",
                    dest,
                    types.to_string(*ty),
                    self.value_to_string(data.value_ref(args[0]), types),
                    args[1..]
                        .iter()
                        .fold("".to_string(), |acc, &arg| {
                            format!(
                                "{}{}, ",
                                acc,
                                self.value_to_string(data.value_ref(arg), types)
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::IntBinary { ty, nuw, nsw, args } => {
                write!(
                    self.fmt,
                    "%{:?} = {:?}{}{} {} {}, {}",
                    dest,
                    inst.opcode,
                    if *nuw { " nuw" } else { "" },
                    if *nsw { " nsw" } else { "" },
                    types.to_string(*ty),
                    self.value_to_string(data.value_ref(args[0]), types),
                    self.value_to_string(data.value_ref(args[1]), types),
                )
            }
            Operand::ICmp { ty, args, cond } => {
                write!(
                    self.fmt,
                    "%{:?} = icmp {:?} {} {}, {}",
                    dest,
                    cond,
                    types.to_string(*ty),
                    self.value_to_string(data.value_ref(args[0]), types),
                    self.value_to_string(data.value_ref(args[1]), types)
                )
            }
            Operand::Cast { tys, arg } => {
                write!(
                    self.fmt,
                    "%{:?} = {:?} {} {} to {}",
                    dest,
                    inst.opcode,
                    types.to_string(tys[0]),
                    self.value_to_string(data.value_ref(*arg), types),
                    types.to_string(tys[1]),
                )
            }
            Operand::GetElementPtr {
                inbounds,
                tys,
                args,
            } => {
                write!(
                    self.fmt,
                    "%{:?} = getelementptr {}{}, {}",
                    dest,
                    if *inbounds { "inbounds " } else { "" },
                    types.to_string(tys[0]),
                    tys[1..]
                        .iter()
                        .zip(args.iter())
                        .fold("".to_string(), |acc, (ty, arg)| {
                            format!(
                                "{}{} {}, ",
                                acc,
                                types.to_string(*ty),
                                self.value_to_string(data.value_ref(*arg), types),
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::Call {
                tys,
                args,
                param_attrs,
                ret_attrs,
                func_attrs,
                ..
            } => {
                write!(
                    self.fmt,
                    "{}call {}{} {}({}) {}",
                    if tys[0] == types.base().void() {
                        "".to_string()
                    } else {
                        format!("%{:?} = ", dest)
                    },
                    ret_attrs
                        .iter()
                        .fold("".to_string(), |acc, attr| format!("{}{:?} ", acc, attr)),
                    types.to_string(tys[0]),
                    self.value_to_string(data.value_ref(args[0]), types),
                    tys[1..]
                        .iter()
                        .zip(args[1..].iter())
                        .zip(param_attrs.iter())
                        .into_iter()
                        .fold("".to_string(), |acc, ((&ty, &arg), attrs)| {
                            format!(
                                "{}{} {}{}, ",
                                acc,
                                types.to_string(ty),
                                attrs.iter().fold("".to_string(), |acc, attr| {
                                    format!("{}{:?} ", acc, attr)
                                }),
                                self.value_to_string(data.value_ref(arg), types),
                            )
                        })
                        .trim_end_matches(", "),
                    func_attrs
                        .iter()
                        .fold("".to_string(), |acc, attr| format!("{}{:?} ", acc, attr))
                )
            }
            Operand::Invoke {
                tys,
                args,
                param_attrs,
                ret_attrs,
                func_attrs,
                blocks,
            } => {
                write!(
                    self.fmt,
                    "{}invoke {}{} {}({}) {}to label %{:?} unwind label %{:?}",
                    if tys[0] == types.base().void() {
                        "".to_string()
                    } else {
                        format!("%{:?} = ", dest)
                    },
                    ret_attrs
                        .iter()
                        .fold("".to_string(), |acc, attr| format!("{}{:?} ", acc, attr)),
                    types.to_string(tys[0]),
                    self.value_to_string(data.value_ref(args[0]), types),
                    tys[1..]
                        .iter()
                        .zip(args[1..].iter())
                        .zip(param_attrs.iter())
                        .into_iter()
                        .fold("".to_string(), |acc, ((&ty, &arg), attrs)| {
                            format!(
                                "{}{} {}{}, ",
                                acc,
                                types.to_string(ty),
                                attrs.iter().fold("".to_string(), |acc, attr| {
                                    format!("{}{:?} ", acc, attr)
                                }),
                                self.value_to_string(data.value_ref(arg), types),
                            )
                        })
                        .trim_end_matches(", "),
                    func_attrs
                        .iter()
                        .fold("".to_string(), |acc, attr| format!("{}{:?} ", acc, attr)),
                    self.indexes[&Ids::Block(blocks[0])],
                    self.indexes[&Ids::Block(blocks[1])],
                )
            }
            Operand::LandingPad { ty } => {
                write!(
                    self.fmt,
                    "{}landingpad {} cleanup",
                    if ty == &types.base().void() {
                        "".to_string()
                    } else {
                        format!("%{:?} = ", dest)
                    },
                    types.to_string(*ty),
                )
            }
            Operand::Resume { ty, arg } => {
                write!(
                    self.fmt,
                    "resume {} {}",
                    types.to_string(*ty),
                    self.value_to_string(data.value_ref(*arg), types),
                )
            }
            Operand::Br { block } => {
                write!(
                    self.fmt,
                    "br label %{:?}",
                    self.indexes[&Ids::Block(*block)]
                )
            }
            Operand::CondBr { arg, blocks } => {
                write!(
                    self.fmt,
                    "br i1 {}, label %{:?}, label %{:?}",
                    self.value_to_string(data.value_ref(*arg), types),
                    self.indexes[&Ids::Block(blocks[0])],
                    self.indexes[&Ids::Block(blocks[1])],
                )
            }
            Operand::Ret { val: None, .. } => write!(self.fmt, "ret void"),
            Operand::Ret { val: Some(val), ty } => {
                write!(
                    self.fmt,
                    "ret {} {}",
                    types.to_string(*ty),
                    self.value_to_string(data.value_ref(*val), types),
                )
            }
            Operand::Invalid => panic!(),
        }
    }

    fn value_to_string(&self, val: &Value, types: &Types) -> String {
        match val {
            Value::Constant(c) => c.to_string(types),
            Value::Instruction(id) => {
                format!("%{:?}", self.indexes[&Ids::Inst(*id)])
            }
            Value::Argument(n) => format!("%{:?}", self.indexes[&Ids::Arg(*n)]),
        }
    }

    fn new_name_for_block(&mut self, id: BasicBlockId) -> Name {
        let idx = self.cur_index;
        self.cur_index += 1;
        self.indexes.insert(Ids::Block(id), Name::Number(idx));
        Name::Number(idx)
    }

    pub fn new_name_for_inst(&mut self, id: InstructionId) -> Name {
        let idx = self.cur_index;
        self.cur_index += 1;
        self.indexes.insert(Ids::Inst(id), Name::Number(idx));
        Name::Number(idx)
    }

    pub fn new_name_for_arg(&mut self, arg: usize) -> Name {
        let idx = self.cur_index;
        self.cur_index += 1;
        self.indexes.insert(Ids::Arg(arg), Name::Number(idx));
        Name::Number(idx)
    }
}

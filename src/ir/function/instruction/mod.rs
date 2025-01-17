pub mod parser;

pub use parser::parse;

use crate::ir::{
    function::{basic_block::BasicBlockId, data::Data, param_attrs::ParameterAttribute},
    module::{attributes::Attribute, name::Name},
    types::TypeId,
    types::Types,
    value::{ConstantData, ValueId},
};
use id_arena::Id;
use std::{fmt, slice};

pub type InstructionId = Id<Instruction>;

pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Operand,
    pub dest: Option<Name>,
    pub id: Option<InstructionId>,
    pub parent: BasicBlockId,
    // pub result_ty: Option<TypeId>
}

#[derive(Clone, Copy, PartialEq)]
pub enum Opcode {
    Alloca,
    Phi,
    Load,
    Store,
    InsertValue,
    ExtractValue,
    Add,
    Sub,
    Mul,
    ICmp,
    Sext,
    Zext,
    Bitcast,
    Trunc,
    GetElementPtr,
    Call,
    Invoke,
    LandingPad,
    Resume,
    Br,
    CondBr,
    Ret,
    Invalid,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ICmpCond {
    Eq,
    Ne,
    Ugt,
    Uge,
    Ult,
    Ule,
    Sgt,
    Sge,
    Slt,
    Sle,
}

#[derive(Clone)]
pub enum Operand {
    Alloca {
        tys: [TypeId; 2],
        num_elements: ConstantData,
        align: u32,
    },
    Phi {
        ty: TypeId,
        args: Vec<ValueId>,
        blocks: Vec<BasicBlockId>,
    },
    Load {
        tys: [TypeId; 2],
        addr: ValueId,
        align: u32,
    },
    IntBinary {
        ty: TypeId,
        nsw: bool,
        nuw: bool,
        args: [ValueId; 2],
    },
    // IntDiv { .. }
    Store {
        tys: [TypeId; 2],
        args: [ValueId; 2],
        align: u32,
    },
    InsertValue {
        tys: [TypeId; 2],
        args: Vec<ValueId>,
    },
    ExtractValue {
        ty: TypeId,
        args: Vec<ValueId>,
    },
    ICmp {
        ty: TypeId,
        args: [ValueId; 2],
        cond: ICmpCond,
    },
    Cast {
        tys: [TypeId; 2], // from, to
        arg: ValueId,
    },
    GetElementPtr {
        inbounds: bool,
        tys: Vec<TypeId>,
        args: Vec<ValueId>,
    },
    Call {
        args: Vec<ValueId>, // args[0] = callee, args[1..] = arguments
        tys: Vec<TypeId>,   // tys[0] = callee's result type, args[1..] = argument types
        param_attrs: Vec<Vec<ParameterAttribute>>, // param_attrs[0] = attrs of args[1]
        ret_attrs: Vec<ParameterAttribute>,
        func_attrs: Vec<Attribute>,
    },
    Invoke {
        args: Vec<ValueId>, // args[0] = callee, args[1..] = arguments
        tys: Vec<TypeId>,   // tys[0] = callee's result type, args[1..] = argument types
        param_attrs: Vec<Vec<ParameterAttribute>>, // param_attrs[0] = attrs of args[1]
        ret_attrs: Vec<ParameterAttribute>,
        func_attrs: Vec<Attribute>,
        blocks: Vec<BasicBlockId>,
    },
    LandingPad {
        ty: TypeId,
    },
    Resume {
        ty: TypeId,
        arg: ValueId,
    },
    Br {
        block: BasicBlockId,
    },
    CondBr {
        arg: ValueId,
        blocks: [BasicBlockId; 2], // iftrue, iffalse
    },
    Ret {
        ty: TypeId,
        val: Option<ValueId>,
    },
    Invalid,
}

impl Instruction {
    pub fn replace(&mut self, other: Self) {
        assert_eq!(self.opcode, Opcode::Invalid);
        self.opcode = other.opcode;
        self.operand = other.operand;
        self.dest = other.dest;
        self.parent = other.parent;
    }

    pub fn with_operand(mut self, operand: Operand) -> Self {
        self.operand = operand;
        self
    }

    pub fn with_dest(mut self, dest: Name) -> Self {
        self.dest = Some(dest);
        self
    }

    pub fn to_string(&self, data: &Data, types: &Types) -> String {
        match &self.operand {
            Operand::Alloca {
                tys,
                num_elements,
                align,
            } => {
                // TODO: %I{index} or %{self.dest}
                format!(
                    "%I{} = alloca {}, {} {}, align {}",
                    self.id.unwrap().index(),
                    types.to_string(tys[0]),
                    types.to_string(tys[1]),
                    num_elements.to_string(types),
                    align
                )
            }
            Operand::Phi { ty, args, blocks } => {
                format!(
                    "%I{} = phi {} {}",
                    self.id.unwrap().index(),
                    types.to_string(*ty),
                    args.iter()
                        .zip(blocks.iter())
                        .fold("".to_string(), |acc, (arg, block)| {
                            format!(
                                "{}[{}, %B{}], ",
                                acc,
                                data.value_ref(*arg).to_string(data, types),
                                block.index()
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::Load { tys, addr, align } => {
                format!(
                    "%I{} = load {}, {} {}, align {}",
                    self.id.unwrap().index(),
                    types.to_string(tys[0]),
                    types.to_string(tys[1]),
                    data.value_ref(*addr).to_string(data, types),
                    align
                )
            }
            Operand::Store { tys, args, align } => {
                format!(
                    "store {} {}, {} {}, align {}",
                    types.to_string(tys[0]),
                    data.value_ref(args[0]).to_string(data, types),
                    types.to_string(tys[1]),
                    data.value_ref(args[1]).to_string(data, types),
                    align
                )
            }
            Operand::InsertValue { .. } => todo!(),
            Operand::ExtractValue { .. } => todo!(),
            Operand::IntBinary { ty, nuw, nsw, args } => {
                format!(
                    "%I{} = {:?}{}{} {} {}, {}",
                    self.id.unwrap().index(),
                    self.opcode,
                    if *nuw { " nuw" } else { "" },
                    if *nsw { " nsw" } else { "" },
                    types.to_string(*ty),
                    data.value_ref(args[0]).to_string(data, types),
                    data.value_ref(args[1]).to_string(data, types),
                )
            }
            Operand::ICmp { ty, args, cond } => {
                format!(
                    "%I{} = icmp {:?} {} {}, {}",
                    self.id.unwrap().index(),
                    cond,
                    types.to_string(*ty),
                    data.value_ref(args[0]).to_string(data, types),
                    data.value_ref(args[1]).to_string(data, types)
                )
            }
            Operand::Cast { tys, arg } => {
                format!(
                    "%I{} = {:?} {} {} to {}",
                    self.id.unwrap().index(),
                    self.opcode,
                    types.to_string(tys[0]),
                    data.value_ref(*arg).to_string(data, types),
                    types.to_string(tys[1]),
                )
            }
            Operand::GetElementPtr {
                inbounds,
                tys,
                args,
            } => {
                format!(
                    "%I{} = getelementptr {}{}, {}",
                    self.id.unwrap().index(),
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
                                data.value_ref(*arg).to_string(data, types)
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::Call { tys, args, .. } => {
                format!(
                    "%I{} = call {} {}({})",
                    self.id.unwrap().index(),
                    types.to_string(tys[0]),
                    data.value_ref(args[0]).to_string(data, types),
                    tys[1..]
                        .iter()
                        .zip(args[1..].iter())
                        .into_iter()
                        .fold("".to_string(), |acc, (t, a)| {
                            format!(
                                "{}{} {}, ",
                                acc,
                                types.to_string(*t),
                                data.value_ref(*a).to_string(data, types),
                            )
                        })
                        .trim_end_matches(", ")
                )
            }
            Operand::Invoke { .. } => todo!(),
            Operand::LandingPad { .. } => todo!(),
            Operand::Resume { .. } => todo!(),
            Operand::Br { block } => {
                format!("br label %B{}", block.index())
            }
            Operand::CondBr { arg, blocks } => {
                format!(
                    "br i1 {}, label %B{}, label %B{}",
                    data.value_ref(*arg).to_string(data, types),
                    blocks[0].index(),
                    blocks[1].index()
                )
            }
            Operand::Ret { val: None, .. } => format!("ret void"),
            Operand::Ret { val: Some(val), ty } => {
                format!(
                    "ret {} {}",
                    types.to_string(*ty),
                    data.value_ref(*val).to_string(data, types)
                )
            }
            Operand::Invalid => panic!(),
        }
    }
}

impl Opcode {
    pub fn with_block(self, parent: BasicBlockId) -> Instruction {
        Instruction {
            opcode: self,
            operand: Operand::Invalid,
            dest: None,
            id: None,
            parent,
            // users: FxHashSet::default(),
        }
    }

    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Self::Ret | Self::Br | Self::CondBr | Self::Invoke | Self::Resume
        )
    }

    pub fn is_load(&self) -> bool {
        self == &Self::Load
    }

    pub fn is_store(&self) -> bool {
        self == &Self::Store
    }

    pub fn is_alloca(&self) -> bool {
        self == &Self::Alloca
    }

    pub fn is_phi(&self) -> bool {
        self == &Self::Phi
    }

    pub fn is_call(&self) -> bool {
        self == &Self::Call
    }

    pub fn is_invoke(&self) -> bool {
        self == &Self::Invoke
    }

    pub fn has_side_effects(&self) -> bool {
        self.is_load()
            || self.is_store()
            || self.is_alloca()
            || self.is_phi()
            || self.is_call()
            || self.is_invoke()
            || self.is_terminator()
    }
}

impl Operand {
    pub fn args(&self) -> &[ValueId] {
        match self {
            Self::Alloca { .. } => &[],
            Self::Phi { args, .. } => args.as_slice(),
            Self::Ret { val, .. } if val.is_none() => &[],
            Self::Ret { val, .. } => slice::from_ref(val.as_ref().unwrap()),
            Self::Load { addr, .. } => slice::from_ref(addr),
            Self::Store { args, .. } => args,
            Self::InsertValue { args, .. } => args,
            Self::ExtractValue { args, .. } => args,
            Self::IntBinary { args, .. } => args,
            Self::ICmp { args, .. } => args,
            Self::Cast { arg, .. } => slice::from_ref(arg),
            Self::GetElementPtr { args, .. } => args.as_slice(),
            Self::Call { args, .. } | Self::Invoke { args, .. } => args.as_slice(),
            Self::LandingPad { .. } => &[],
            Self::Resume { arg, .. } => slice::from_ref(arg),
            Self::Br { .. } => &[],
            Self::CondBr { arg, .. } => slice::from_ref(arg),
            Self::Invalid => &[],
        }
    }

    pub fn types(&self) -> &[TypeId] {
        match self {
            Self::Alloca { tys, .. } => tys,
            Self::Phi { ty, .. } => slice::from_ref(ty),
            Self::Ret { ty, .. } => slice::from_ref(ty),
            Self::Load { tys, .. } => tys,
            Self::Store { .. } => &[],
            Self::InsertValue { tys, .. } => tys,
            Self::ExtractValue { ty, .. } => slice::from_ref(ty),
            Self::IntBinary { ty, .. } => slice::from_ref(ty),
            Self::ICmp { ty, .. } => slice::from_ref(ty),
            Self::Cast { tys, .. } => tys,
            Self::GetElementPtr { tys, .. } => tys.as_slice(),
            Self::Call { tys, .. } | Self::Invoke { tys, .. } => tys.as_slice(),
            Self::LandingPad { ty } => slice::from_ref(ty),
            Self::Resume { ty, .. } => slice::from_ref(ty),
            Self::Br { .. } => &[],
            Self::CondBr { .. } => &[],
            Self::Invalid => &[],
        }
    }

    pub fn blocks(&self) -> &[BasicBlockId] {
        match self {
            Self::Phi { blocks, .. } => blocks,
            Self::Br { block } => slice::from_ref(block),
            Self::CondBr { blocks, .. } => blocks,
            Self::Invoke { blocks, .. } => blocks,
            _ => &[],
        }
    }

    pub fn call_result_ty(&self) -> Option<TypeId> {
        match self {
            Self::Call { tys, .. } | Self::Invoke { tys, .. } => Some(tys[0]),
            _ => None,
        }
    }
}

impl fmt::Debug for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Opcode::Alloca => "alloca",
                Opcode::Phi => "phi",
                Opcode::Load => "load",
                Opcode::Store => "store",
                Opcode::InsertValue => "insertvalue",
                Opcode::ExtractValue => "extractvalue",
                Opcode::Add => "add",
                Opcode::Sub => "sub",
                Opcode::Mul => "mul",
                Opcode::ICmp => "icmp",
                Opcode::Sext => "sext",
                Opcode::Zext => "zext",
                Opcode::Bitcast => "bitcast",
                Opcode::Trunc => "trunc",
                Opcode::GetElementPtr => "getelementptr",
                Opcode::Call => "call",
                Opcode::Invoke => "invoke",
                Opcode::LandingPad => "landingpad",
                Opcode::Resume => "resume",
                Opcode::Br | Opcode::CondBr => "br",
                Opcode::Ret => "ret",
                Opcode::Invalid => "INVALID",
            }
        )
    }
}

impl fmt::Debug for ICmpCond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Eq => "eq",
                Self::Ne => "ne",
                Self::Ugt => "ugt",
                Self::Uge => "uge",
                Self::Ult => "ult",
                Self::Ule => "ule",
                Self::Sgt => "sgt",
                Self::Sge => "sge",
                Self::Slt => "slt",
                Self::Sle => "sle",
            }
        )
    }
}

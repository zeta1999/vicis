use super::{
    super::{function::parser::ParserContext, types::TypeId},
    ValueId,
};
use nom::{error::VerboseError, IResult};

pub fn parse<'a, 'b>(
    _source: &'a str,
    _ctx: &mut ParserContext<'b>,
    _ty: TypeId,
) -> IResult<&'a str, ValueId, VerboseError<&'a str>> {
    todo!()
}

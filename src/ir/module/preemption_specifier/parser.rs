use super::PreemptionSpecifier;
use nom::{
    branch::alt,
    bytes::complete::tag, //take_until},
    // character::complete::{char, digit1},
    combinator::map,
    error::VerboseError,
    IResult,
};

pub fn parse<'a>(source: &'a str) -> IResult<&'a str, PreemptionSpecifier, VerboseError<&'a str>> {
    alt((
        map(tag("dso_local"), |_| PreemptionSpecifier::DsoLocal),
        map(tag("dso_preemptable"), |_| {
            PreemptionSpecifier::DsoPreemptable
        }),
    ))(source)
}

// use deku::prelude::*;

pub trait Request/*<'a, 'b>: DekuRead<'a> + DekuWrite*/ {
    type Response: Response/*<'b>*/;
}

pub trait Response/*: DekuRead<'_> + DekuWrite*/ {
    type Enum;
}
pub trait Message/*: DekuRead<'_> + DekuWrite*/ {}
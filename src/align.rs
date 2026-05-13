use std::borrow::Cow;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Align {
    Left,
    Center,
}

pub enum TextFragment<'a> {
    Raw(&'a str),
    Ansi(&'a dyn core::fmt::Display), // styling, does not count towards width
}

impl Align {
    pub fn print(fragments: &[TextFragment]) {}
}

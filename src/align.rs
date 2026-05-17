#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Align {
    Left,
    Center,
}

pub enum TextFragment<'a> {
    Raw(&'a str),
    HalfDivisor(&'a str),
    Ansi(&'a dyn core::fmt::Display), // styling, does not count towards width
}

impl Align {
    pub fn print(&self, fragments: &[TextFragment], terminal_width: u16) {
        let cols: u16 = fragments
            .iter()
            .map(|f| match f {
                TextFragment::Raw(s) => s.len() as u16,
                TextFragment::HalfDivisor(s) => s.len() as u16,
                TextFragment::Ansi(_) => 0,
            })
            .sum();

        let padding = match self {
            Align::Left => 0,
            Align::Center => terminal_width.saturating_sub(cols) / 2,
        } as usize;

        if let Align::Center = self {
            if let Some(idx) = fragments
                .iter()
                .position(|f| matches!(f, TextFragment::HalfDivisor(_)))
            {
                let cols_before: u16 = fragments
                    .iter()
                    .take(idx)
                    .map(|f| match f {
                        TextFragment::Raw(s) => s.len() as u16,
                        TextFragment::HalfDivisor(s) => s.len() as u16,
                        TextFragment::Ansi(_) => 0,
                    })
                    .sum();

                let effective_padding =
                    (terminal_width.saturating_sub(0) / 2).saturating_sub(cols_before) as usize;
                print!(
                    "{:effective_padding$}",
                    "",
                    effective_padding = effective_padding
                );
            } else {
                print!("{:padding$}", "", padding = padding);
            }
        } else {
            print!("{:padding$}", "", padding = padding);
        }

        for fragment in fragments {
            match fragment {
                TextFragment::Raw(s) => print!("{}", s),
                TextFragment::HalfDivisor(s) => print!("{}", s),
                TextFragment::Ansi(f) => print!("{}", f),
            }
        }
        println!();
    }
}

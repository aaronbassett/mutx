use anyhow::Result;

pub struct Args;

impl Args {
    pub fn parse() -> Self {
        Args
    }
}

pub fn run(_args: Args) -> Result<()> {
    println!("Stub CLI - to be implemented");
    Ok(())
}

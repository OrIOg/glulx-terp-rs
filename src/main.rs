mod glulx_terp;
use std::{env, fs::File, path::Path};
use crate::glulx_terp::GlulxTerp;

#[derive(Debug)]
pub enum Errors {
    TargetArgNotFound,
    TargetLoading(std::io::Error),
    Interpreter(glulx_terp::Errors),
}


fn main() -> Result<(), Errors> {
    let args: Vec<String> = env::args().collect();

    let Some(path) = args.get(1) else {
        return Err(Errors::TargetArgNotFound)
    };

    let path = Path::new(path);

    println!("Trying to load: {path:?}");

    let mut file = File::open(path).map_err(Errors::TargetLoading)?;

    let mut terp = GlulxTerp::from_reader(&mut file)
        .map_err(Errors::Interpreter)?;
    println!("Successfully loaded target.");

    terp.run();
    
    Ok(())
}

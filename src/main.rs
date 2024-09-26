use app::App;
use color_eyre::Result;
use debug::Logger;

use std::fs::File;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use read_write::*;

pub mod errors;
pub mod tui;
pub mod app;
pub mod read_write;
pub mod debug;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let logger = Rc::new(RefCell::new(Logger::default()));
    let _debug = !true;
    let mut terminal = tui::init()?;

    let path_to_self = env::current_exe()?;
    let path = path_to_self
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p|p.parent())
        .map(|p|p.join("Highscore.bin"))
        .unwrap();
    let number: u64;
    if !path.exists() {
        File::create(&path)?;
        number = 0;
    }
    else {
        number = read(&path)?;
    }

    let mut app = App::new(logger.clone())?;
    app.highscore = number;
    app.run(&mut terminal)?;
    tui::restore()?;
    
    if _debug {
        print!("{}", logger.borrow());
    }

    save(&path, app.highscore)?;
    Ok(())
}


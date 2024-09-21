use app::App;
use color_eyre::Result;

use std::fs::File;
use std::env;

use read_write::*;

pub mod errors;
pub mod tui;
pub mod app;
pub mod read_write;

fn main() -> Result<()> {
    errors::install_hooks()?;
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

    let mut app = App::new()?;
    app.highscore = number;
    app.run(&mut terminal)?;
    tui::restore()?;
    
    save(&path, app.highscore)?;
    Ok(())
}


use crate::tui;

use color_eyre::{
    eyre::WrapErr, Result
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use num::ToPrimitive;
use rand::{thread_rng, Rng};
use ratatui::{
    prelude::*, 
    style::Color, 
    widgets::{block::*, canvas::{Canvas, Rectangle}, Paragraph, *}
};

use std::{env, thread};

use std::time::Duration;

use crate::read_write::*;

#[derive(Debug, Default)]
pub struct App {
    pub score: u64,
    pub highscore: u64,
    exit: bool,
    on_pause: bool,
    dead: bool,
    current_piece: Piece,
    pieces: Vec<Piece>,
    next_piece: Piece,
    padding: f64,
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
        where
            Self: Sized {

                let fg_color: Color;
                let bg_color: Color;

                if self.dead {
                    fg_color = Color::Red;
                    bg_color = Color::Black;
                }
                else {
                    fg_color = Color::White;
                    bg_color = Color::Black;
                }

                let block = Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().bold())
                                .title(Title::from(" Tetris ".bold())
                                        .alignment(Alignment::Center))
                                .bg(bg_color)
                                .fg(fg_color);

                let score_text = Line::from(self.score.to_string().bold());        
                let highscore_text = Line::from(self.highscore.to_string().bold());

                Paragraph::new(Line::from(" Next Piece              "))
                    .block(block.clone())
                    .right_aligned()
                    .render(area, buf);

                Paragraph::new(score_text)
                    .block(block.clone())
                    .right_aligned()
                    .render(area, buf);

                Paragraph::new(highscore_text)
                    .block(block.clone())
                    .left_aligned()
                    .render(area, buf);

                if self.dead {
                    let death_text = Line::from(vec![Span::from(" You died with score "), Span::from(self.score.to_string().bold())]);
                    Paragraph::new(death_text)
                    .block(block.clone())
                    .alignment(Alignment::Center)
                    .render(area, buf);

                }
                
                if self.on_pause {
                    let death_text = Line::from(vec![Span::from(" Paused "), Span::from(self.score.to_string().bold())]);
                    Paragraph::new(death_text)
                    .block(block.clone())
                    .centered()
                    .render(area, buf);

                }

                if !self.dead {
                    Canvas::default()
                        .block(block.clone())
                        .x_bounds([-180.0, 180.0])
                        .y_bounds([-90.0, 90.0])
                        .background_color(Color::Black)
                        .paint(|ctx| {
                            ctx.draw(&Rectangle {
                                x: -70.0, 
                                y: -90.0,
                                width: 140.0,
                                height: 180.0,
                                color: Color::White,
                            });
                            ctx.layer();
                            for component in self.current_piece.components.iter() {
                                ctx.draw(&Rectangle {
                                    x: component.x + self.padding,
                                    y: component.y + self.padding,
                                    width: component.width - self.padding,
                                    height: component.height - self.padding,
                                    color: self.current_piece.color
                                });
                            }
                            ctx.layer();
                            for piece in self.pieces.iter() {
                                for component in piece.components.iter() {
                                    ctx.draw(&Rectangle {
                                        x: component.x + self.padding,
                                        y: component.y + self.padding,
                                        width: component.width - self.padding,
                                        height: component.height - self.padding,
                                        color: piece.color
                                    });
                                }
                            }
                            ctx.layer();
                            for component in self.next_piece.components.iter() {
                                ctx.draw(&Rectangle {
                                    x: component.x + self.padding,
                                    y: component.y + self.padding,
                                    width: component.width - self.padding,
                                    height: component.height - self.padding,
                                    color: self.next_piece.color
                                });
                            }
                        })
                        .render(area, buf);

                    if self.on_pause {
                        Paragraph::new(Line::from("Paused"))
                            .block(block.clone())
                            .centered()
                            .bold()
                            .render(area, buf);
                    }
                }
    }   
}

impl App {

    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render_frame(frame))?;
            let time = 500000;
            if event::poll(Duration::from_micros(time))? {
                self.handle_events().wrap_err("handle events failed")?;
                thread::sleep(Duration::from_micros(50000));
            }
            if self.exit {
                break;
            }
            if self.on_pause || self.dead {
                continue;
            }
            self.handle_piece()?;
            self.highscore();
            self.is_dead()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn highscore(&mut self) {
        if self.score > self.highscore {
            self.highscore = self.score;
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event).wrap_err_with(|| {
                    format!("handling key event failed: \n{key_event:#?}")
                })
            }
           _ => Ok(())
        }
    }

    pub fn new() -> Result<App> {
        let mut app = App {
            score: 0,
            highscore: 0,
            exit: false,
            dead: false,
            on_pause: false,
            current_piece: Piece::placeholder(), // make these random
            pieces: vec![],
            next_piece: Piece::placeholder(),
            padding: 0.0, // 2.0 seems good
        };
        app.init_queue()?;
        app.next_piece()?;
        Ok(app)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Esc => self.pause()?,
            KeyCode::Enter => self.restart()?,
            KeyCode::Right => self.move_current_right()?,
            KeyCode::Left => self.move_current_left()?,
            //KeyCode::Down => self.move_current_down()?,
            KeyCode::Up => self.rotate_current()?,
            _ => {}
        }
        Ok(())
    }

    fn restart(&mut self) -> Result<()> {

        if self.dead {
            let path_to_self = env::current_exe()?;
            let path = path_to_self
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p|p.parent())
                .map(|p|p.join("Highscore.bin"))
                .unwrap();
            save(&path, self.highscore)?;
            
            let num = read(&path)?;

            self.highscore = num;
            self.score = 0;
            self.on_pause = false;
            self.dead = false;
            self.pieces = vec![];
            self.next_piece()?;
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn pause(&mut self) -> Result<()> {
        if self.on_pause {
            self.on_pause = false;
        }
        else {
            self.on_pause = true;
        }
        Ok(())
    }

    fn is_dead(&mut self) -> Result<()> {
        if self.pieces.iter().map(|piece| {
            piece.max_y >= 80.0
        }).any(|x| x) {
            self.dead = true;
        }
        Ok(())
    }

    fn row_clear(&mut self, min: f64, max: f64) -> Result<()> {
        let mut deleted_rows = vec![];
        let start = (min / 10.0).to_i8().unwrap();
        let stop = (max / 10.0).to_i8().unwrap();
        for i in start..(stop + 1) {
            let row = Piece::whole_line((10 * i).to_f64().unwrap());
            if row.components.iter().map(|cmp| {
                self.pieces.iter().map(|piece| {
                    piece.is_blocked(cmp)
                }).any(|x| x)
            }).all(|x| x) { // if the piece is manually moved in the moment it hits the bottom this evauates to false somehow
                self.delete_row((10 * i).to_f64().unwrap())?;
                self.score += 1000;
                deleted_rows.push((10 * i).to_f64().unwrap());
            }
        }
        deleted_rows.reverse();
        if deleted_rows.len() > 1 {
            self.score += 1000 * deleted_rows.len().to_u64().unwrap();
        }
        for val in deleted_rows.iter() {
            self.gravity(*val)?;
        }

        Ok(())
    }

    fn delete_row(&mut self, row: f64) -> Result<()> {
        for piece in self.pieces.iter_mut() {
            if piece.max_y < row || piece.min_y > row {
                continue;
            }
            let mut count = 0;
            for (i, cmp) in piece.components.clone().iter().enumerate() {
                if cmp.y == row {
                    piece.components.remove(i - count);
                    count += 1;
                }
            }
        }
        Ok(())
    }

    fn gravity(&mut self, y: f64) -> Result<()> {
        for piece in self.pieces.iter_mut() {
            for cmp in piece.components.iter_mut() {
                if cmp.y < y {
                    continue;
                }
                cmp.y -= 10.0;
            }
        }
        Ok(())
    }

    fn handle_piece(&mut self) -> Result<()> {
        self.move_current_down()?;
        if self.current_piece_at_bottom()? {
            self.pieces.push(self.current_piece.clone());
            self.row_clear(self.current_piece.min_y, self.current_piece.max_y)?;
            self.next_piece()?;
        }
        Ok(())
    }

    fn current_piece_at_bottom(&mut self) -> Result<bool> {
        let mut current_piece = self.current_piece.clone();
        current_piece.move_down()?;
        Ok(current_piece.components.iter().map(|cmp| {
            self.pieces.iter().map(|piece| {
                piece.is_blocked(cmp)
            }).any(|x| x)
        }).any(|x| x) || self.current_piece.min_y == -90.0)
    }

    fn next_piece(&mut self) -> Result<()> {
        self.current_piece = self.next_piece.clone();
        let mut rng = thread_rng();
        let random_num = rng.gen_range(0..4);
        let colors = vec![Color::White, Color::Cyan, Color::Yellow, Color::Red, Color::Blue, Color::Magenta, Color::Green];
        if random_num == 0 {
            self.next_piece = Piece::long();
        }
        else if random_num == 1 {
            self.next_piece = Piece::square();
        }
        else if random_num == 2 {
            self.next_piece = Piece::t_piece(); 
        }
        else if random_num == 3 {
            self.next_piece = Piece::l_piece();
        }
        self.current_piece.set_center();
        for _ in 0..rng.gen_range(0..3) {
            self.rotate_current()?;
        }
        self.next_piece.color = colors[rng.gen_range(0..colors.len())];

        for _ in 0..12 {
            self.next_piece.move_right(true)?;
            self.current_piece.move_left(true)?;
        }
        for _ in 0..3 {
            self.next_piece.move_down()?;
            self.current_piece.move_up()?;
        }
        Ok(())
    }

    fn init_queue(&mut self) -> Result<()> {
        let mut rng = thread_rng();
        let random_num = rng.gen_range(0..=4);
        let colors = vec![Color::White, Color::Cyan, Color::Yellow, Color::Red, Color::Blue, Color::Magenta, Color::Green];
        if random_num == 0 {
            self.next_piece = Piece::long();
        }
        else if random_num == 1 {
            self.next_piece = Piece::square();
        }
        else if random_num == 2 {
            self.next_piece = Piece::t_piece(); 
        }
        else if random_num == 3 {
            let random_num_for_orientation = rng.gen_range(0.0..1.0);
            if random_num_for_orientation < 0.5 {
                self.next_piece = Piece::inverted_l_piece();
            }
            else {
                self.next_piece = Piece::l_piece();
            }
        }
        else if random_num == 4 {
            let random_num_for_orientation = rng.gen_range(0.0..1.0);
            if random_num_for_orientation < 0.5 {
                self.next_piece = Piece::inverted_z_piece();
            }
            else {
                self.next_piece = Piece::z_piece();
            }
        }
        self.next_piece.color = colors[rng.gen_range(0..colors.len())];

        for _ in 0..12 {
            self.next_piece.move_right(true)?;
        }
        for _ in 0..3 {
            self.next_piece.move_down()?;
        }
        Ok(())
    }

    fn move_current_down(&mut self) -> Result<()> {
        let mut current_piece = self.current_piece.clone();
        current_piece.move_down()?;
        if !current_piece.components.iter().map(|cmp| {
            self.pieces.iter().map(|piece| {
                piece.is_blocked(cmp)
            }).any(|x| x)
        }).any(|x| x) {
            self.current_piece.move_down()?;
        }
        Ok(())
    }

    fn move_current_left(&mut self) -> Result<()> {
        let mut current_piece = self.current_piece.clone();
        current_piece.move_left(false)?;
        if !(current_piece.components.iter().map(|cmp| {
            self.pieces.iter().map(|piece| {
                piece.is_blocked(cmp)
            }).any(|x| x)
        }).any(|x| x) || current_piece.out_of_bounds()) {
            self.current_piece.move_left(false)?;
        }
        Ok(())
    }

    fn move_current_right(&mut self) -> Result<()> {
        let mut current_piece = self.current_piece.clone();
        current_piece.move_right(false)?;
        if !(current_piece.components.iter().map(|cmp| {
            self.pieces.iter().map(|piece| {
                piece.is_blocked(cmp)
            }).any(|x| x)
        }).any(|x| x) || current_piece.out_of_bounds()) {
            self.current_piece.move_right(false)?;
        }
        Ok(())
    }

    fn rotate_current(&mut self) -> Result<()> {
        //TODO
        let mut copy = self.current_piece.clone();
        copy.rotate()?;
        if !(copy.components.iter().map(|cmp| {
            self.pieces.iter().map(|piece| {
                piece.is_blocked(cmp)
            }).any(|x| x)
        }).any(|x| x) || copy.out_of_bounds()) {
            self.current_piece.rotate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
struct Piece {
    color: Color,
    components: Vec<SimplePiece>,
    min_y: f64,    
    max_y: f64,
    center: Vec<f64>,
}

impl Piece {

    fn move_right(&mut self, force: bool) -> Result<()> {
        if self.components.clone().iter().any(|cmp|cmp.x >= 60.0) && !force {
            return Ok(());
        }
        for piece in self.components.iter_mut() {
            piece.x += 10.0;
            piece.center[0] += 10.0;
        }
        self.set_center();
        //self.center[0] -= 10.0;
        Ok(())
    }

    fn move_left(&mut self, force: bool) -> Result<()> {
        if self.components.clone().iter().any(|cmp|cmp.x <= -70.0) && !force {
            return Ok(());
        }
        for piece in self.components.iter_mut() {
            piece.x -= 10.0;
            piece.center[0] -= 10.0;
        }
        //self.center[0] -= 10.0;
        self.set_center();
        Ok(())
    }

    fn move_down(&mut self) -> Result<()> {
        if self.components.clone().iter().any(|cmp|cmp.y <= -90.0) {
            return Ok(());
        }
        for piece in self.components.iter_mut() {
            piece.y -= 10.0;
            piece.center[1] -= 10.0;
        }
        self.min_y -= 10.0;
        self.max_y -= 10.0;
        //self.center[1] -= 10.0;
        self.set_center();
        Ok(())
    }

    fn move_up(&mut self) -> Result<()> {
        if self.components.clone().iter().any(|cmp|cmp.y >= 80.0) {
            return Ok(());
        }
        for piece in self.components.iter_mut() {
            piece.y += 10.0;
            piece.center[1] += 10.0;
        }
        self.min_y += 10.0;
        self.max_y += 10.0;
        //self.center[1] -= 10.0;
        self.set_center();
        Ok(())
    }


    fn rotate(&mut self) -> Result<()> {
        //TODO
        // In order to rotate the shape properly, it needs to be centered in the orign -> center, rotate, decenter
        let angle: f64 = std::f64::consts::FRAC_PI_2;
        //self.set_center();
        for cmp in self.components.iter_mut() {
            let x_shift = self.center[0];
            let y_shift = self.center[1];
            cmp.x -= x_shift;
            cmp.y -= y_shift;
            let x = cmp.x;
            cmp.x = cmp.x * angle.cos() - cmp.y * angle.sin() + x_shift;
            cmp.y = x * angle.sin() + cmp.y * angle.cos() + y_shift;
        }
        self.set_center();
        self.min_y = get_min_y(self.components.clone()); 
        self.max_y = get_max_y(self.components.clone());
        let y_diff = round_to_tenths(self.min_y);
        let x_diff = round_to_tenths(get_min_x(self.components.clone()));
        self.min_y -= y_diff;
        self.max_y -= y_diff;
        for cmp in self.components.iter_mut() {
            cmp.x -= x_diff;
            cmp.y -= y_diff;
            cmp.center[1] -= y_diff;
            cmp.center[0] -= x_diff;
        }
        Ok(())
    }

    fn is_blocked(&self, piece: &SimplePiece) -> bool {
        self.components.iter().map(|cmp| {
            cmp.is_equal(piece)
        }).any(|x| x)
    }

    fn out_of_bounds(&self) -> bool {
        self.components.iter().map(|cmp| {
            cmp.y < -90.0 || cmp.y > 80.0 || cmp.x < -70.0 || cmp.x > 60.0
        }).any(|cmp| cmp)
    }

    fn long() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(0.0, 70.0),
                SimplePiece::new(0.0, 60.0)
            ],
            min_y: 60.0,
            max_y: 90.0,
            center: vec![0.0, 75.0],
        }
    }

    fn square() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(10.0, 90.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(10.0, 80.0)
            ],
            min_y: 80.0,
            max_y: 90.0,
            center: vec![0.0, 85.0],
        }
    }

    fn t_piece() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(-10.0, 90.0),
                SimplePiece::new(10.0, 90.0),
                SimplePiece::new(0.0, 80.0)
            ],
            min_y: 80.0,
            max_y: 90.0,
            center: vec![0.0, 85.0],
        }
    }

    fn l_piece() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(0.0, 70.0),
                SimplePiece::new(10.0, 70.0)
            ],
            min_y: 70.0,
            max_y: 90.0,
            center: vec![0.0, 80.0],
        }
    }
    
    fn inverted_l_piece() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(0.0, 70.0),
                SimplePiece::new(-10.0, 70.0)
            ],
            min_y: 70.0,
            max_y: 90.0,
            center: vec![0.0, 80.0],
        }
    }

    fn z_piece() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(-10.0, 90.0),
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(10.0, 80.0)
            ],
            min_y: 80.0,
            max_y: 90.0,
            center: vec![0.0, 90.0],
        }
    }
    fn inverted_z_piece() -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(-10.0, 80.0),
                SimplePiece::new(0.0, 80.0),
                SimplePiece::new(0.0, 90.0),
                SimplePiece::new(10.0, 90.0)
            ],
            min_y: 80.0,
            max_y: 90.0,
            center: vec![0.0, 90.0],
        }
    }

    fn whole_line(y: f64) -> Piece {
        Piece {
            color: Color::White,
            components: vec![
                SimplePiece::new(-70.0, y),
                SimplePiece::new(-60.0, y),
                SimplePiece::new(-50.0, y),
                SimplePiece::new(-40.0, y),
                SimplePiece::new(-30.0, y),
                SimplePiece::new(-20.0, y),
                SimplePiece::new(-10.0, y),
                SimplePiece::new(0.0, y),
                SimplePiece::new(10.0, y),
                SimplePiece::new(20.0, y),
                SimplePiece::new(30.0, y),
                SimplePiece::new(40.0, y),
                SimplePiece::new(50.0, y),
                SimplePiece::new(60.0, y),
            ],
            min_y: y,
            max_y: y,
            center: vec![5.0, y + 5.0],
        }
    }

    fn placeholder() -> Piece {
        Piece {
            color: Color::White,
            components: vec![],
            min_y: 0.0,
            max_y: 0.0,
            center: vec![0.0, 0.0],
        }
    }

    fn set_center(&mut self) {
        self.center = get_center(self.components.clone());
    }
}

#[derive(Debug, Default, Clone)]
struct SimplePiece {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    center: Vec<f64>,
}

impl SimplePiece {
    
    fn new(x: f64, y: f64) -> SimplePiece {
        SimplePiece {
            x,
            y, 
            width: 10.0,
            height: 10.0,
            center: vec![x + 5.0, y + 5.0],
        }
    }

    fn is_equal(&self, piece: &SimplePiece) -> bool {
        self.x.to_i8() == piece.x.to_i8() && self.y.to_i8() == piece.y.to_i8()
    }
}

fn get_center(cmps: Vec<SimplePiece>) -> Vec<f64> {
    vec![cmps.iter().map(|cmp| cmp.center[0]).sum::<f64>() / cmps.len().to_f64().unwrap(),
        cmps.iter().map(|cmp| cmp.center[1]).sum::<f64>() / cmps.len().to_f64().unwrap()]
}

fn get_min_y(cmps: Vec<SimplePiece>) -> f64 {
    let mut min = f64::INFINITY;
    for cmp in cmps.iter() {
        if cmp.y < min {
            min = cmp.y;
        }
    }
    min
}

fn get_max_y(cmps: Vec<SimplePiece>) -> f64 {
    let mut max = -f64::INFINITY;
    for cmp in cmps.iter() {
        if cmp.y > max {
            max = cmp.y;
        }
    }
    max
}

fn get_min_x(cmps: Vec<SimplePiece>) -> f64 {
    let mut min = f64::INFINITY;
    for cmp in cmps.iter() {
        if cmp.x < min {
            min = cmp.x;
        }
    }
    min
}

fn round_to_tenths(num: f64) -> f64 {
    let int = num.round().to_i64().unwrap();
    let diff = num / 10.0 - (int / 10).to_f64().unwrap();
    diff * 10.0
}
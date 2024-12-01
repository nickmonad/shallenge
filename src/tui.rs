use crate::hash;
use core_affinity::CoreId;
use crossbeam::{channel, select};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
    DefaultTerminal, Frame,
};
use std::{io, thread};

pub fn run(mut app: App) -> io::Result<hash::WithNonce> {
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);

    ratatui::restore();
    result
}

pub struct App {
    minimum: hash::WithNonce,
    results: channel::Receiver<hash::Result>,
    display: Vec<hash::Result>,
    prefix: String,
}

impl App {
    pub fn new(
        cores: Vec<CoreId>,
        results: channel::Receiver<hash::Result>,
        prefix: String,
    ) -> Self {
        let minimum = hash::max();
        let display: Vec<hash::Result> = cores.iter().map(|core| (core.clone(), None)).collect();

        Self {
            minimum,
            results,
            display,
            prefix,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<hash::WithNonce> {
        // event thread
        let (q, quit) = channel::unbounded::<bool>();
        let _ = thread::spawn(move || {
            // block on reading keyboard events
            // quit on 'Esc'
            loop {
                match event::read() {
                    Ok(Event::Key(e)) if e.kind == KeyEventKind::Press => {
                        if e.code == KeyCode::Esc {
                            let _ = q.send(true);
                        }
                    }
                    _ => {}
                }
            }
        });

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            select! {
                recv(self.results) -> result => {
                    if let Ok((core, hash)) = result {
                        if let Some(ref h) = hash {
                            // check for global minimum
                            if hash::is_less(&h.0, &self.minimum.0) {
                                self.minimum = h.clone()
                           }

                            // set display value for core
                            self.display[core.id] = (core, hash);
                        }
                    }
                }
                recv(quit) -> _ => {
                    break;
                }
            }
        }

        Ok(self.minimum.clone())
    }

    fn draw(&self, frame: &mut Frame) {
        let results: Vec<Line> = self
            .display
            .iter()
            .map(|result| match result {
                (core, None) => Line::from(format!("core {:>2} : {}", core.id, "-".repeat(64))),
                (core, Some((hash, nonce))) => {
                    let mut leading = true;
                    let styled: Vec<Span> = hex::encode(hash)
                        .char_indices()
                        .map(|(i, c)| {
                            if i % 8 == 0 {
                                if c == '0' && leading {
                                    format!(" {}", c).magenta()
                                } else {
                                    leading = false;
                                    format!(" {}", c).green()
                                }
                            } else {
                                if c == '0' && leading {
                                    format!("{}", c).magenta()
                                } else {
                                    leading = false;
                                    format!("{}", c).green()
                                }
                            }
                        })
                        .collect();

                    let mut line = vec![Span::from(format!("core {:>2} : ", core.id))];
                    line.extend(styled);
                    line.push(Span::from(format!(" -> {}/{}", self.prefix, nonce)));

                    Line::from(line)
                }
            })
            .collect();

        frame.render_widget(Paragraph::new(results), frame.area());
    }
}

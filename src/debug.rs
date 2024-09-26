use std::fmt::{self, Debug};

#[derive (Default, Debug)]
pub struct LogEntry<T: Debug> {
    entry: T
}

#[derive (Default, Debug)]
pub struct Logger {
    log: Vec<LogEntry<Box<dyn Debug>>>
}

impl Logger {
    pub fn push<T: Debug + 'static>(&mut self, val: T) {
        self.log.push(LogEntry {entry: Box::new(val)});
    }
}

impl fmt::Display for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for log in self.log.iter() {
            writeln!(f, "{:#?}", log.entry.as_ref())?;
        }
        Ok(())
    }
}

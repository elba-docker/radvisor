use crate::cli::{Opts, ParseFailure};
use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;

use termcolor::{self, Color, ColorSpec, StandardStream, WriteColor};

/// Inspiration/partial implementations taken from the Cargo source at
/// [cargo/core/shell.rs](https://github.com/rust-lang/cargo/blob/53094e32b11c57a917f3ec3a48f29f388583ca3b/src/cargo/core/shell.rs)

/// Maximum length of status string when being justified
const JUSTIFY_STATUS_LEN: usize = 12usize;

/// The requested verbosity of the program output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verbosity {
    Verbose,
    Normal,
    Quiet,
}

impl Verbosity {
    /// Determines the appropriate verbosity setting for the specified CLI
    /// options
    fn from_opts(opts: &Opts) -> Self {
        match opts.quiet {
            true => Verbosity::Quiet,
            false => match opts.verbose {
                true => Verbosity::Verbose,
                false => Verbosity::Normal,
            },
        }
    }
}

/// Mode of the color output of the process, controllable via a CLI flag
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for ColorMode {
    type Err = ParseFailure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err(ParseFailure::new(String::from("color mode"), s.to_owned())),
        }
    }
}

impl ColorMode {
    fn into_termcolor(self, stream: atty::Stream) -> termcolor::ColorChoice {
        match self {
            ColorMode::Always => termcolor::ColorChoice::Always,
            ColorMode::Never => termcolor::ColorChoice::Never,
            ColorMode::Auto => {
                if atty::is(stream) {
                    termcolor::ColorChoice::Auto
                } else {
                    termcolor::ColorChoice::Never
                }
            },
        }
    }
}

/// Thread-safe handle to formatted stderr/stdout output (implements `Sync`)
pub struct Shell {
    verbosity: Verbosity,
    out:       Mutex<OutSink>,
    err:       Mutex<OutSink>,
}

#[allow(dead_code)]
impl Shell {
    /// Creates a new instance of the Shell handle, initializing all fields from
    /// the CLI options as necessary. Should only be called once per process.
    pub fn new(opts: &Opts) -> Self {
        Shell {
            verbosity: Verbosity::from_opts(opts),
            out:       Mutex::new(OutSink::Stream {
                color_mode:  opts.color_mode,
                is_tty:      atty::is(atty::Stream::Stdout),
                stream_type: atty::Stream::Stdout,
                stream:      StandardStream::stdout(
                    opts.color_mode.into_termcolor(atty::Stream::Stdout),
                ),
            }),
            err:       Mutex::new(OutSink::Stream {
                color_mode:  opts.color_mode,
                is_tty:      atty::is(atty::Stream::Stderr),
                stream_type: atty::Stream::Stderr,
                stream:      StandardStream::stderr(
                    opts.color_mode.into_termcolor(atty::Stream::Stderr),
                ),
            }),
        }
    }

    /// Creates a shell from plain writable objects, with no color, and max
    /// verbosity.
    pub fn from_write(stdout: Box<dyn Write + Send>, stderr: Box<dyn Write + Send>) -> Self {
        Shell {
            out:       Mutex::new(OutSink::Write(stdout)),
            err:       Mutex::new(OutSink::Write(stderr)),
            verbosity: Verbosity::Verbose,
        }
    }

    /// Shortcut to right-align and color green a status message.
    pub fn status<T, U>(&self, status: T, message: U)
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        self.print(&status, Some(&message), Color::Green, None, true);
    }

    pub fn status_header<T>(&self, status: T)
    where
        T: fmt::Display,
    {
        self.print(&status, None, Color::Cyan, None, true);
    }

    /// Prints a message, where the status will have `color` color, and can be
    /// justified. The messages follows without color.
    fn print(
        &self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        status_color: Color,
        text_color: Option<Color>,
        justified: bool,
    ) {
        match self.verbosity {
            Verbosity::Quiet => (),
            _ => {
                let mut out = self
                    .out
                    .lock()
                    .expect("Could not unwrap stdout lock: mutex poisoned");
                let _ = out.print(status, message, status_color, text_color, justified);
            },
        }
    }

    /// Prints a red 'error' message.
    pub fn error<T: fmt::Display>(&self, message: T) {
        let mut err = self
            .err
            .lock()
            .expect("Could not unwrap stderr lock: mutex poisoned");
        let _ = err.print(
            &"(error)",
            Some(&message),
            Color::Red,
            Some(Color::Red),
            true,
        );
    }

    /// Prints an amber 'warning' message.
    pub fn warn<T: fmt::Display>(&self, message: T) {
        match self.verbosity {
            Verbosity::Quiet => (),
            _ => self.print(&"(warning)", Some(&message), Color::Yellow, None, true),
        };
    }

    /// Prints a cyan 'info' message.
    pub fn info<T: fmt::Display>(&self, message: T) {
        self.print(&"(info)", Some(&message), Color::Cyan, None, true);
    }

    /// Gets the current color mode.
    ///
    /// If we are not using a color stream, this will always return `Never`,
    /// even if the color mode has been set to something else.
    pub fn color_mode(&self) -> ColorMode {
        let out = self
            .out
            .lock()
            .expect("Could not unwrap stdout lock: mutex poisoned");
        match *out {
            OutSink::Stream { color_mode, .. } => color_mode,
            OutSink::Write(_) => ColorMode::Never,
        }
    }

    /// Whether the shell supports color.
    pub fn supports_color(&self) -> bool {
        let out = self
            .out
            .lock()
            .expect("Could not unwrap stdout lock: mutex poisoned");
        match &*out {
            OutSink::Write(_) => false,
            OutSink::Stream { stream, .. } => stream.supports_color(),
        }
    }

    /// Executes the given callback with a reference to the shell object handle
    /// if the shell is in verbose mode
    pub fn verbose<F>(&self, callback: F)
    where
        F: Fn(&Shell) -> (),
    {
        if let Verbosity::Verbose = self.verbosity {
            callback(self);
        }
    }
}

enum OutSink {
    Write(Box<dyn Write + Send>),
    Stream {
        color_mode:  ColorMode,
        stream:      StandardStream,
        stream_type: atty::Stream,
        is_tty:      bool,
    },
}

impl OutSink {
    /// Prints out a message with a status. The status comes first, and is bold
    /// plus the given color. The status can be justified, in which case the
    /// max width that will right align is JUSTIFY_STATUS_LEN chars.
    fn print(
        &mut self,
        status: &dyn fmt::Display,
        message: Option<&dyn fmt::Display>,
        status_color: Color,
        text_color: Option<Color>,
        justified: bool,
    ) -> io::Result<()> {
        let width: Option<usize> = self.width();
        match *self {
            OutSink::Stream {
                ref mut stream,
                is_tty,
                ..
            } => {
                stream.reset()?;
                stream.set_color(ColorSpec::new().set_bold(true).set_fg(Some(status_color)))?;

                // Calculate the offset based on the line header
                let offset = if justified && is_tty {
                    write!(stream, "{:>width$}", status, width = JUSTIFY_STATUS_LEN)?;
                    JUSTIFY_STATUS_LEN
                } else {
                    let status_str = format!("{}", status);
                    write!(stream, "{}", status_str)?;
                    stream.set_color(ColorSpec::new().set_bold(true))?;
                    write!(stream, ":")?;
                    status_str.len() + 1
                };

                stream.reset()?;
                if let Some(color) = text_color {
                    stream.set_color(ColorSpec::new().set_fg(Some(color)))?;
                }

                match message {
                    None => write!(stream, " ")?,
                    Some(message) => {
                        // If width can be found, then wrap/indent
                        match width {
                            None => writeln!(stream, " {}", message)?,
                            Some(width) => {
                                let formatted: String = format!("{}", message);
                                let lines = textwrap::wrap_iter(&formatted, width - (offset + 1));
                                let mut is_first = true;
                                let indent = " ".repeat(offset);
                                for line in lines {
                                    if is_first {
                                        is_first = false;
                                        writeln!(stream, " {}", line)?;
                                    } else {
                                        writeln!(stream, "{} {}", indent, line)?;
                                    }
                                }
                            },
                        }
                    },
                }

                stream.reset()?;
            },
            OutSink::Write(ref mut w) => {
                if justified {
                    write!(w, "{:width$}", status, width = JUSTIFY_STATUS_LEN)?;
                } else {
                    write!(w, "{}:", status)?;
                }
                match message {
                    Some(message) => writeln!(w, " {}", message)?,
                    None => write!(w, " ")?,
                }
            },
        }
        Ok(())
    }

    /// Gets width of terminal, if applicable
    fn width(&self) -> Option<usize> {
        match self {
            OutSink::Stream {
                is_tty: true,
                stream_type,
                ..
            } => imp::width(*stream_type),
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::mem;

    pub fn width(stream: atty::Stream) -> Option<usize> {
        unsafe {
            let mut winsize: libc::winsize = mem::zeroed();

            // Resolve correct fileno for the stream type
            let fileno = match stream {
                atty::Stream::Stdout => libc::STDOUT_FILENO,
                _ => libc::STDERR_FILENO,
            };

            if libc::ioctl(fileno, libc::TIOCGWINSZ, &mut winsize) < 0 {
                return None;
            }
            if winsize.ws_col > 0 {
                Some(winsize.ws_col as usize)
            } else {
                None
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::{cmp, mem, ptr};
    use winapi::um::fileapi::*;
    use winapi::um::handleapi::*;
    use winapi::um::processenv::*;
    use winapi::um::winbase::*;
    use winapi::um::wincon::*;
    use winapi::um::winnt::*;

    pub fn width(_stream: atty::Stream) -> Option<usize> {
        unsafe {
            let stdout = GetStdHandle(STD_ERROR_HANDLE);
            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            if GetConsoleScreenBufferInfo(stdout, &mut csbi) != 0 {
                return Some((csbi.srWindow.Right - csbi.srWindow.Left) as usize);
            }

            // On mintty/msys/cygwin based terminals, the above fails with
            // INVALID_HANDLE_VALUE. Use an alternate method which works
            // in that case as well.
            let h = CreateFileA(
                "CONOUT$\0".as_ptr() as *const CHAR,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = mem::zeroed();
            let rc = GetConsoleScreenBufferInfo(h, &mut csbi);
            CloseHandle(h);
            if rc != 0 {
                let width = (csbi.srWindow.Right - csbi.srWindow.Left) as usize;
                // Unfortunately cygwin/mintty does not set the size of the
                // backing console to match the actual window size. This
                // always reports a size of 80 or 120 (not sure what
                // determines that). Use a conservative max of 60 which should
                // work in most circumstances. ConEmu does some magic to
                // resize the console correctly, but there's no reasonable way
                // to detect which kind of terminal we are running in, or if
                // GetConsoleScreenBufferInfo returns accurate information.
                return Some(cmp::min(60, width));
            }
            None
        }
    }
}

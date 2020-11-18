use crate::cli::ParseFailure;
use crate::util;
use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;

use clap::Clap;
use termcolor::{self, Color, ColorSpec, StandardStream, WriteColor};

/// Inspiration/partial implementations taken from the Cargo source at
/// [cargo/core/shell.rs](https://github.com/rust-lang/cargo/blob/53094e32b11c57a917f3ec3a48f29f388583ca3b/src/cargo/core/shell.rs)

/// Maximum length of status string when being justified
const JUSTIFY_STATUS_LEN: usize = 12_usize;

/// The requested verbosity of the program output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verbosity {
    Verbose,
    Normal,
    Quiet,
}

// All clap-compatible configuration parameters for the Shell
#[derive(Clap, Clone)]
pub struct Options {
    /// Whether to run in quiet mode (minimal output)
    #[clap(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,

    /// Whether to run in verbose mode (maximum output)
    #[clap(short = 'v', long = "verbose", global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Color display mode for stdout/stderr output
    #[clap(short = 'c', long = "color", default_value = "auto", global = true)]
    pub color_mode: ColorMode,
}

impl Verbosity {
    /// Determines the appropriate verbosity setting for the specified CLI
    /// options
    const fn from_opts(opts: &Options) -> Self {
        match opts.quiet {
            true => Self::Quiet,
            false => match opts.verbose {
                true => Self::Verbose,
                false => Self::Normal,
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
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(ParseFailure::new(String::from("color mode"), s.to_owned())),
        }
    }
}

impl ColorMode {
    fn into_termcolor(self, stream: atty::Stream) -> termcolor::ColorChoice {
        match self {
            Self::Always => termcolor::ColorChoice::Always,
            Self::Never => termcolor::ColorChoice::Never,
            Self::Auto => {
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
    pub verbosity: Verbosity,
    out:           Mutex<OutSink>,
    err:           Mutex<OutSink>,
}

#[allow(dead_code)]
impl Shell {
    /// Creates a new instance of the Shell handle, initializing all fields from
    /// the CLI options as necessary. Should only be called once per process.
    #[must_use]
    pub fn new(opts: &Options) -> Self {
        Self {
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
    #[must_use]
    pub fn from_write(stdout: Box<dyn Write + Send>, stderr: Box<dyn Write + Send>) -> Self {
        Self {
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
        if self.verbosity != Verbosity::Quiet {
            let mut out = self
                .out
                .lock()
                .expect("Could not unwrap stdout lock: mutex poisoned");
            let _ = out.print(status, message, status_color, text_color, justified);
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
        F: FnOnce(&Self),
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
    /// max width that will right align is `JUSTIFY_STATUS_LEN` chars.
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
            Self::Stream {
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
            Self::Write(ref mut w) => {
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
    #[must_use]
    fn width(&self) -> Option<usize> {
        match self {
            Self::Stream {
                is_tty: true,
                stream_type,
                ..
            } => util::terminal_width(*stream_type),
            _ => None,
        }
    }
}

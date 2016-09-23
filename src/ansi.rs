// Copyright 2016 Joe Wilm, The Alacritty Project Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//! ANSI Terminal Stream Parsing
//!
//! The `Parser` implementation is largely based on the suck-less _simple terminal_ parser. Because
//! this is Rust and Rust has a fantastic type system, some improvements are possible. First,
//! `Parser` is a struct, and its data is stored internally instead of statically. Second, there's
//! no terminal updates hard-coded into the parser. Instead, `Parser` is generic over a `Handler`
//! type which has methods for all of the actions supported by the parser. Because Parser is
//! generic, it should be possible (with proper inlining) to have equivalent performance to the
//! hard-coded version.
//!
//! In addition to using _simple terminal_ as a reference, there's a doc in Alacritty's repository
//! `docs/ansicode.txt`, a summary of the ANSI terminal protocol, which has been referenced
//! extensively.
//!
//! There's probably a large number escapes we don't handle, and that's ok. There's a lot that
//! aren't necessary for everyday terminal usage. If you feel like something that's not supported
//! should be, feel free to add it. Please try not to become overzealous and adding support for
//! sequences only used by folks trapped in 1988.
use std::ops::Range;

use vte;

use index::{Column, Line};

use ::Rgb;

/// The processor wraps a vte::Parser to ultimately call methods on a Handler
pub struct Processor {
    state: ProcessorState,
    parser: vte::Parser,
}

/// Internal state for VTE processor
struct ProcessorState;

/// Helper type that implements vte::Perform.
///
/// Processor creates a Performer when running advance and passes the Performer
/// to vte::Parser.
struct Performer<'a, H: Handler + TermInfo + 'a> {
    state: &'a mut ProcessorState,
    handler: &'a mut H
}

impl<'a, H: Handler + TermInfo + 'a> Performer<'a, H> {
    /// Create a performer
    #[inline]
    pub fn new<'b>(state: &'b mut ProcessorState, handler: &'b mut H) -> Performer<'b, H> {
        Performer {
            state: state,
            handler: handler
        }
    }
}

impl Processor {
    pub fn new() -> Processor {
        Processor {
            state: ProcessorState,
            parser: vte::Parser::new(),
        }
    }

    #[inline]
    pub fn advance<H: Handler + TermInfo>(&mut self, handler: &mut H, byte: u8) {
        let mut performer = Performer::new(&mut self.state, handler);
        self.parser.advance(&mut performer, byte);
    }
}


/// Trait that provides properties of terminal
pub trait TermInfo {
    fn lines(&self) -> Line;
    fn cols(&self) -> Column;
}

/// Type that handles actions from the parser
///
/// XXX Should probably not provide default impls for everything, but it makes
/// writing specific handler impls for tests far easier.
pub trait Handler {
    /// A character to be displayed
    fn input(&mut self, _c: char) {}

    /// Set cursor to position
    fn goto(&mut self, Line, Column) {}

    /// Set cursor to specific row
    fn goto_line(&mut self, Line) {}

    /// Set cursor to specific column
    fn goto_col(&mut self, Column) {}

    /// Insert blank characters in current line starting from cursor
    fn insert_blank(&mut self, usize) {}

    /// Move cursor up `rows`
    fn move_up(&mut self, Line) {}

    /// Move cursor down `rows`
    fn move_down(&mut self, Line) {}

    /// Identify the terminal (should write back to the pty stream)
    fn identify_terminal(&mut self) {}

    /// Move cursor forward `cols`
    fn move_forward(&mut self, Column) {}

    /// Move cursor backward `cols`
    fn move_backward(&mut self, Column) {}

    /// Move cursor down `rows` and set to column 1
    fn move_down_and_cr(&mut self, Line) {}

    /// Move cursor up `rows` and set to column 1
    fn move_up_and_cr(&mut self, Line) {}

    /// Put `count` tabs
    fn put_tab(&mut self, _count: i64) {}

    /// Backspace `count` characters
    fn backspace(&mut self) {}

    /// Carriage return
    fn carriage_return(&mut self) {}

    /// Linefeed
    fn linefeed(&mut self) {}

    /// Ring the bell
    ///
    /// Hopefully this is never implemented
    fn bell(&mut self) {}

    /// Substitute char under cursor
    fn substitute(&mut self) {}

    /// Newline
    fn newline(&mut self) {}

    /// Set current position as a tabstop
    fn set_horizontal_tabstop(&mut self) {}

    /// Scroll up `rows` rows
    fn scroll_up(&mut self, Line) {}

    /// Scroll down `rows` rows
    fn scroll_down(&mut self, Line) {}

    /// Insert `count` blank lines
    fn insert_blank_lines(&mut self, Line) {}

    /// Delete `count` lines
    fn delete_lines(&mut self, Line) {}

    /// Erase `count` chars in current line following cursor
    ///
    /// Erase means resetting to the default state (default colors, no content, no mode flags)
    fn erase_chars(&mut self, Column) {}

    /// Delete `count` chars
    ///
    /// Deleting a character is like the delete key on the keyboard - everything to the right of the
    /// deleted things is shifted left.
    fn delete_chars(&mut self, Column) {}

    /// Move backward `count` tabs
    fn move_backward_tabs(&mut self, _count: i64) {}

    /// Move forward `count` tabs
    fn move_forward_tabs(&mut self, _count: i64) {}

    /// Save current cursor position
    fn save_cursor_position(&mut self) {}

    /// Restore cursor position
    fn restore_cursor_position(&mut self) {}

    /// Clear current line
    fn clear_line(&mut self, _mode: LineClearMode) {}

    /// Clear screen
    fn clear_screen(&mut self, _mode: ClearMode) {}

    /// Clear tab stops
    fn clear_tabs(&mut self, _mode: TabulationClearMode) {}

    /// Reset terminal state
    fn reset_state(&mut self) {}

    /// Reverse Index
    ///
    /// Move the active position to the same horizontal position on the preceding line. If the
    /// active position is at the top margin, a scroll down is performed
    fn reverse_index(&mut self) {}

    /// set a terminal attribute
    fn terminal_attribute(&mut self, _attr: Attr) {}

    /// Set mode
    fn set_mode(&mut self, _mode: Mode) {}

    /// Unset mode
    fn unset_mode(&mut self, Mode) {}

    /// DECSTBM - Set the terminal scrolling region
    fn set_scrolling_region(&mut self, Range<Line>) {}

    /// DECKPAM - Set keypad to applications mode (ESCape instead of digits)
    fn set_keypad_application_mode(&mut self) {}

    /// DECKPNM - Set keypad to numeric mode (digits intead of ESCape seq)
    fn unset_keypad_application_mode(&mut self) {}
}

/// Terminal modes
#[derive(Debug, Eq, PartialEq)]
pub enum Mode {
    /// ?1
    CursorKeys = 1,
    /// ?6
    Origin = 6,
    /// ?12
    BlinkingCursor = 12,
    /// ?25
    ShowCursor = 25,
    /// ?1049
    SwapScreenAndSetRestoreCursor = 1049,
}

impl Mode {
    /// Create mode from a primitive
    ///
    /// TODO lots of unhandled values..
    pub fn from_primitive(private: bool, num: i64) -> Option<Mode> {
        if private {
            Some(match num {
                1 => Mode::CursorKeys,
                6 => Mode::Origin,
                12 => Mode::BlinkingCursor,
                25 => Mode::ShowCursor,
                1049 => Mode::SwapScreenAndSetRestoreCursor,
                _ => return None
            })
        } else {
            // TODO
            None
        }
    }
}

/// Mode for clearing line
///
/// Relative to cursor
#[derive(Debug)]
pub enum LineClearMode {
    /// Clear right of cursor
    Right,
    /// Clear left of cursor
    Left,
    /// Clear entire line
    All,
}

/// Mode for clearing terminal
///
/// Relative to cursor
#[derive(Debug)]
pub enum ClearMode {
    /// Clear below cursor
    Below,
    /// Clear above cursor
    Above,
    /// Clear entire terminal
    All,
}

/// Mode for clearing tab stops
#[derive(Debug)]
pub enum TabulationClearMode {
    /// Clear stop under cursor
    Current,
    /// Clear all stops
    All,
}

/// Standard colors
///
/// The order here matters since the enum should be castable to a `usize` for
/// indexing a color list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    /// Black
    Black = 0,
    /// Red
    Red,
    /// Green
    Green,
    /// Yellow
    Yellow,
    /// Blue
    Blue,
    /// Magenta
    Magenta,
    /// Cyan
    Cyan,
    /// White
    White,
    /// Bright black
    BrightBlack,
    /// Bright red
    BrightRed,
    /// Bright green
    BrightGreen,
    /// Bright yellow
    BrightYellow,
    /// Bright blue
    BrightBlue,
    /// Bright magenta
    BrightMagenta,
    /// Bright cyan
    BrightCyan,
    /// Bright white
    BrightWhite,
}

/// Terminal character attributes
#[derive(Debug, Eq, PartialEq)]
pub enum Attr {
    /// Clear all special abilities
    Reset,
    /// Bold text
    Bold,
    /// Dim or secondary color
    Dim,
    /// Italic text
    Italic,
    /// Underscore text
    Underscore,
    /// Blink cursor slowly
    BlinkSlow,
    /// Blink cursor fast
    BlinkFast,
    /// Invert colors
    Reverse,
    /// Do not display characters
    Hidden,
    /// Strikethrough text
    Strike,
    /// Cancel bold and dim
    CancelBoldDim,
    /// Cancel italic
    CancelItalic,
    /// Cancel underline
    CancelUnderline,
    /// Cancel blink
    CancelBlink,
    /// Cancel inversion
    CancelReverse,
    /// Cancel text hiding
    CancelHidden,
    /// Cancel strike through
    CancelStrike,
    /// Set indexed foreground color
    Foreground(Color),
    /// Set specific foreground color
    ForegroundSpec(Rgb),
    /// Set indexed background color
    Background(Color),
    /// Set specific background color
    BackgroundSpec(Rgb),
    /// Set default foreground
    DefaultForeground,
    /// Set default background
    DefaultBackground,
}

impl<'a, H: Handler + TermInfo + 'a> vte::Perform for Performer<'a, H> {
    #[inline]
    fn print(&mut self, c: char) {
        self.handler.input(c);
    }

    #[inline]
    fn execute(&mut self, byte: u8) {
        match byte {
            C0::HT => self.handler.put_tab(1),
            C0::BS => self.handler.backspace(),
            C0::CR => self.handler.carriage_return(),
            C0::LF | C0::VT | C0::FF => self.handler.linefeed(),
            C0::BEL => self.handler.bell(),
            C0::SUB => self.handler.substitute(),
            C1::NEL => self.handler.newline(),
            C1::HTS => self.handler.set_horizontal_tabstop(),
            C1::DECID => self.handler.identify_terminal(),
            _ => (),
        }
    }

    #[inline]
    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, byte: u8) {
        err_println!("[unhandled hook] params={:?}, ints: {:?}, ignore: {:?}, byte={:?}",
                     params, intermediates, ignore, byte as char);
    }

    #[inline]
    fn put(&mut self, byte: u8) {
        err_println!("[unhandled put] byte={:?}", byte);
    }

    #[inline]
    fn unhook(&mut self, byte: u8) {
        err_println!("[unhandled unhook] byte={:?}", byte);
    }

    #[inline]
    fn osc_start(&mut self) {
        err_println!("[unhandled osc_start]");
    }

    #[inline]
    fn osc_put(&mut self, byte: u8) {
        err_println!("[unhandled osc_put] byte={:?}", byte as char);
    }

    #[inline]
    fn osc_end(&mut self, byte: u8) {
        err_println!("[unhandled osc_end] byte={:?}", byte);
    }

    #[inline]
    fn csi_dispatch(&mut self, args: &[i64], intermediates: &[u8], _ignore: bool, action: char) {
        let private = intermediates.get(0).map(|b| *b == b'?').unwrap_or(false);
        let handler = &mut self.handler;


        macro_rules! unhandled {
            () => {{
                err_println!("[Unhandled CSI] action={:?}, args={:?}, intermediates={:?}",
                             action, args, intermediates);
                return;
            }}
        }

        macro_rules! arg_or_default {
            (idx: $idx:expr, default: $default:expr) => {
                args.get($idx).and_then(|v| {
                    if *v == 0 {
                        None
                    } else {
                        Some(*v)
                    }
                }).unwrap_or($default)
            }
        }

        match action {
            '@' => handler.insert_blank(arg_or_default!(idx: 0, default: 1) as usize),
            'A' => {
                handler.move_up(Line(arg_or_default!(idx: 0, default: 1) as usize));
            },
            'B' | 'e' => handler.move_down(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'c' => handler.identify_terminal(),
            'C' | 'a' => handler.move_forward(Column(arg_or_default!(idx: 0, default: 1) as usize)),
            'D' => handler.move_backward(Column(arg_or_default!(idx: 0, default: 1) as usize)),
            'E' => handler.move_down_and_cr(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'F' => handler.move_up_and_cr(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'g' => {
                let mode = match arg_or_default!(idx: 0, default: 0) {
                    0 => TabulationClearMode::Current,
                    3 => TabulationClearMode::All,
                    _ => unhandled!(),
                };

                handler.clear_tabs(mode);
            },
            'G' | '`' => handler.goto_col(Column(arg_or_default!(idx: 0, default: 1) as usize - 1)),
            'H' | 'f' => {
                let y = arg_or_default!(idx: 0, default: 1) as usize;
                let x = arg_or_default!(idx: 1, default: 1) as usize;
                handler.goto(Line(y - 1), Column(x - 1));
            },
            'I' => handler.move_forward_tabs(arg_or_default!(idx: 0, default: 1)),
            'J' => {
                let mode = match arg_or_default!(idx: 0, default: 0) {
                    0 => ClearMode::Below,
                    1 => ClearMode::Above,
                    2 => ClearMode::All,
                    _ => unhandled!(),
                };

                handler.clear_screen(mode);
            },
            'K' => {
                let mode = match arg_or_default!(idx: 0, default: 0) {
                    0 => LineClearMode::Right,
                    1 => LineClearMode::Left,
                    2 => LineClearMode::All,
                    _ => unhandled!(),
                };

                handler.clear_line(mode);
            },
            'S' => handler.scroll_up(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'T' => handler.scroll_down(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'L' => handler.insert_blank_lines(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'l' => {
                let mode = Mode::from_primitive(private, arg_or_default!(idx: 0, default: 0));
                match mode {
                    Some(mode) => handler.unset_mode(mode),
                    None => unhandled!(),
                }
            },
            'M' => handler.delete_lines(Line(arg_or_default!(idx: 0, default: 1) as usize)),
            'X' => handler.erase_chars(Column(arg_or_default!(idx: 0, default: 1) as usize)),
            'P' => handler.delete_chars(Column(arg_or_default!(idx: 0, default: 1) as usize)),
            'Z' => handler.move_backward_tabs(arg_or_default!(idx: 0, default: 1)),
            'd' => handler.goto_line(Line(arg_or_default!(idx: 0, default: 1) as usize - 1)),
            'h' => {
                let mode = Mode::from_primitive(private, arg_or_default!(idx: 0, default: 0));
                match mode {
                    Some(mode) => handler.set_mode(mode),
                    None => unhandled!(),
                }
            },
            'm' => {
                // Sometimes a C-style for loop is just what you need
                let mut i = 0; // C-for initializer
                if args.len() == 0 {
                    handler.terminal_attribute(Attr::Reset);
                    return;
                }
                loop {
                    // println!("args.len = {}; i={}", args.len(), i);
                    if i >= args.len() { // C-for condition
                        break;
                    }

                    let attr = match args[i] {
                        0 => Attr::Reset,
                        1 => Attr::Bold,
                        2 => Attr::Dim,
                        3 => Attr::Italic,
                        4 => Attr::Underscore,
                        5 => Attr::BlinkSlow,
                        6 => Attr::BlinkFast,
                        7 => Attr::Reverse,
                        8 => Attr::Hidden,
                        9 => Attr::Strike,
                        22 => Attr::CancelBoldDim,
                        23 => Attr::CancelItalic,
                        24 => Attr::CancelUnderline,
                        25 => Attr::CancelBlink,
                        27 => Attr::CancelReverse,
                        28 => Attr::CancelHidden,
                        29 => Attr::CancelStrike,
                        30 => Attr::Foreground(Color::Black),
                        31 => Attr::Foreground(Color::Red),
                        32 => Attr::Foreground(Color::Green),
                        33 => Attr::Foreground(Color::Yellow),
                        34 => Attr::Foreground(Color::Blue),
                        35 => Attr::Foreground(Color::Magenta),
                        36 => Attr::Foreground(Color::Cyan),
                        37 => Attr::Foreground(Color::White),
                        38 => {
                            if let Some(spec) = parse_color(&args[i..], &mut i) {
                                Attr::ForegroundSpec(spec)
                            } else {
                                break;
                            }
                        },
                        39 => Attr::DefaultForeground,
                        40 => Attr::Background(Color::Black),
                        41 => Attr::Background(Color::Red),
                        42 => Attr::Background(Color::Green),
                        43 => Attr::Background(Color::Yellow),
                        44 => Attr::Background(Color::Blue),
                        45 => Attr::Background(Color::Magenta),
                        46 => Attr::Background(Color::Cyan),
                        47 => Attr::Background(Color::White),
                        48 =>  {
                            if let Some(spec) = parse_color(&args[i..], &mut i) {
                                Attr::BackgroundSpec(spec)
                            } else {
                                break;
                            }
                        },
                        49 => Attr::DefaultBackground,
                        90 => Attr::Foreground(Color::BrightBlack),
                        91 => Attr::Foreground(Color::BrightRed),
                        92 => Attr::Foreground(Color::BrightGreen),
                        93 => Attr::Foreground(Color::BrightYellow),
                        94 => Attr::Foreground(Color::BrightBlue),
                        95 => Attr::Foreground(Color::BrightMagenta),
                        96 => Attr::Foreground(Color::BrightCyan),
                        97 => Attr::Foreground(Color::BrightWhite),
                        100 => Attr::Foreground(Color::BrightBlack),
                        101 => Attr::Foreground(Color::BrightRed),
                        102 => Attr::Foreground(Color::BrightGreen),
                        103 => Attr::Foreground(Color::BrightYellow),
                        104 => Attr::Foreground(Color::BrightBlue),
                        105 => Attr::Foreground(Color::BrightMagenta),
                        106 => Attr::Foreground(Color::BrightCyan),
                        107 => Attr::Foreground(Color::BrightWhite),
                        _ => unhandled!(),
                    };

                    handler.terminal_attribute(attr);

                    i += 1; // C-for expr
                }
            }
            'n' => handler.identify_terminal(),
            'r' => {
                if private {
                    unhandled!();
                }
                let arg0 = arg_or_default!(idx: 0, default: 1) as usize;
                let top = Line(arg0 - 1);
                // Bottom should be included in the range, but range end is not
                // usually included.  One option would be to use an inclusive
                // range, but instead we just let the open range end be 1
                // higher.
                let arg1 = arg_or_default!(idx: 1, default: handler.lines().0 as _) as usize;
                let bottom = Line(arg1);

                handler.set_scrolling_region(top..bottom);
            },
            's' => handler.save_cursor_position(),
            'u' => handler.restore_cursor_position(),
            _ => unhandled!(),
        }
    }

    #[inline]
    fn esc_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, byte: u8) {
        let private = intermediates.get(0).map(|b| *b == b'?').unwrap_or(false);

        match byte {
            b'D' => self.handler.linefeed(),
            b'E' => self.handler.newline(),
            b'H' => self.handler.set_horizontal_tabstop(),
            b'M' => self.handler.reverse_index(),
            b'Z' => self.handler.identify_terminal(),
            b'c' => self.handler.reset_state(),
            b'7' => self.handler.save_cursor_position(),
            b'8' => self.handler.restore_cursor_position(),
            b'=' => self.handler.set_keypad_application_mode(),
            b'>' => self.handler.unset_keypad_application_mode(),
            _ => err_println!("[unhandled] esc_dispatch params={:?}, ints={:?}, byte={:?} ({:02x})",
                              params, intermediates, byte as char, byte),
        }
    }
}


/// Parse a color specifier from list of attributes
fn parse_color(attrs: &[i64], i: &mut usize) -> Option<Rgb> {
    if attrs.len() < 2 {
        return None;
    }

    match attrs[*i+1] {
        2 => {
            // RGB color spec
            if attrs.len() < 5 {
                err_println!("Expected RGB color spec; got {:?}", attrs);
                return None;
            }

            let r = attrs[*i+2];
            let g = attrs[*i+3];
            let b = attrs[*i+4];

            *i = *i + 4;

            let range = 0...255;
            if !range.contains(r) || !range.contains(g) || !range.contains(b) {
                err_println!("Invalid RGB color spec: ({}, {}, {})", r, g, b);
                return None;
            }

            Some(Rgb {
                r: r as u8,
                g: g as u8,
                b: b as u8
            })
        },
        _ => {
            err_println!("Unexpected color attr: {}", attrs[*i+1]);
            None
        }
    }
}

/// C0 set of 7-bit control characters (from ANSI X3.4-1977).
#[allow(non_snake_case)]
pub mod C0 {
    /// Null filler, terminal should ignore this character
    pub const NUL: u8 = 0x00;
    /// Start of Header
    pub const SOH: u8 = 0x01;
    /// Start of Text, implied end of header
    pub const STX: u8 = 0x02;
    /// End of Text, causes some terminal to respond with ACK or NAK
    pub const ETX: u8 = 0x03;
    /// End of Transmission
    pub const EOT: u8 = 0x04;
    /// Enquiry, causes terminal to send ANSWER-BACK ID
    pub const ENQ: u8 = 0x05;
    /// Acknowledge, usually sent by terminal in response to ETX
    pub const ACK: u8 = 0x06;
    /// Bell, triggers the bell, buzzer, or beeper on the terminal
    pub const BEL: u8 = 0x07;
    /// Backspace, can be used to define overstruck characters
    pub const BS: u8 = 0x08;
    /// Horizontal Tabulation, move to next predetermined position
    pub const HT: u8 = 0x09;
    /// Linefeed, move to same position on next line (see also NL)
    pub const LF: u8 = 0x0A;
    /// Vertical Tabulation, move to next predetermined line
    pub const VT: u8 = 0x0B;
    /// Form Feed, move to next form or page
    pub const FF: u8 = 0x0C;
    /// Carriage Return, move to first character of current line
    pub const CR: u8 = 0x0D;
    /// Shift Out, switch to G1 (other half of character set)
    pub const SO: u8 = 0x0E;
    /// Shift In, switch to G0 (normal half of character set)
    pub const SI: u8 = 0x0F;
    /// Data Link Escape, interpret next control character specially
    pub const DLE: u8 = 0x10;
    /// (DC1) Terminal is allowed to resume transmitting
    pub const XON: u8 = 0x11;
    /// Device Control 2, causes ASR-33 to activate paper-tape reader
    pub const DC2: u8 = 0x12;
    /// (DC2) Terminal must pause and refrain from transmitting
    pub const XOFF: u8 = 0x13;
    /// Device Control 4, causes ASR-33 to deactivate paper-tape reader
    pub const DC4: u8 = 0x14;
    /// Negative Acknowledge, used sometimes with ETX and ACK
    pub const NAK: u8 = 0x15;
    /// Synchronous Idle, used to maintain timing in Sync communication
    pub const SYN: u8 = 0x16;
    /// End of Transmission block
    pub const ETB: u8 = 0x17;
    /// Cancel (makes VT100 abort current escape sequence if any)
    pub const CAN: u8 = 0x18;
    /// End of Medium
    pub const EM: u8 = 0x19;
    /// Substitute (VT100 uses this to display parity errors)
    pub const SUB: u8 = 0x1A;
    /// Prefix to an ESCape sequence
    pub const ESC: u8 = 0x1B;
    /// File Separator
    pub const FS: u8 = 0x1C;
    /// Group Separator
    pub const GS: u8 = 0x1D;
    /// Record Separator (sent by VT132 in block-transfer mode)
    pub const RS: u8 = 0x1E;
    /// Unit Separator
    pub const US: u8 = 0x1F;
    /// Delete, should be ignored by terminal
    pub const DEL: u8 = 0x7f;
}


/// C1 set of 8-bit control characters (from ANSI X3.64-1979)
///
/// 0x80 (@), 0x81 (A), 0x82 (B), 0x83 (C) are reserved
/// 0x98 (X), 0x99 (Y) are reserved
/// 0x9a (Z) is resezved, but causes DEC terminals to respond with DA codes
#[allow(non_snake_case)]
pub mod C1 {
    /// Reserved
    pub const PAD: u8 = 0x80;
    /// Reserved
    pub const HOP: u8 = 0x81;
    /// Reserved
    pub const BPH: u8 = 0x82;
    /// Reserved
    pub const NBH: u8 = 0x83;
    /// Index, moves down one line same column regardless of NL
    pub const IND: u8 = 0x84;
    /// NEw Line, moves done one line and to first column (CR+LF)
    pub const NEL: u8 = 0x85;
    /// Start of Selected Area to be  as charsent to auxiliary output device
    pub const SSA: u8 = 0x86;
    /// End of Selected Area to be sent to auxiliary output device
    pub const ESA: u8 = 0x87;
    /// Horizontal Tabulation Set at current position
    pub const HTS: u8 = 0x88;
    /// Hor Tab Justify, moves string to next tab position
    pub const HTJ: u8 = 0x89;
    /// Vertical Tabulation Set at current line
    pub const VTS: u8 = 0x8A;
    /// Partial Line Down (subscript)
    pub const PLD: u8 = 0x8B;
    /// Partial Line Up (superscript)
    pub const PLU: u8 = 0x8C;
    /// Reverse Index, go up one line, reverse scroll if necessary
    pub const RI: u8 = 0x8D;
    /// Single Shift to G2
    pub const SS2: u8 = 0x8E;
    /// Single Shift to G3 (VT100 uses this for sending PF keys)
    pub const SS3: u8 = 0x8F;
    /// Device Control String, terminated by ST (VT125 enters graphics)
    pub const DCS: u8 = 0x90;
    /// Private Use 1
    pub const PU1: u8 = 0x91;
    /// Private Use 2
    pub const PU2: u8 = 0x92;
    /// Set Transmit State
    pub const STS: u8 = 0x93;
    /// Cancel CHaracter, ignore previous character
    pub const CCH: u8 = 0x94;
    /// Message Waiting, turns on an indicator on the terminal
    pub const MW: u8 = 0x95;
    /// Start of Protected Area
    pub const SPA: u8 = 0x96;
    /// End of Protected Area
    pub const EPA: u8 = 0x97;
    /// SOS
    pub const SOS: u8 = 0x98;
    /// SGCI
    pub const SGCI: u8 = 0x99;
    /// DECID - Identify Terminal
    pub const DECID: u8 = 0x9a;
    /// Control Sequence Introducer (described in a seperate table)
    pub const CSI: u8 = 0x9B;
    /// String Terminator (VT125 exits graphics)
    pub const ST: u8 = 0x9C;
    /// Operating System Command (reprograms intelligent terminal)
    pub const OSC: u8 = 0x9D;
    /// Privacy Message (password verification), terminated by ST
    pub const PM: u8 = 0x9E;
    /// Application Program Command (to word processor), term by ST
    pub const APC: u8 = 0x9F;
}

// Tests for parsing escape sequences
//
// Byte sequences used in these tests are recording of pty stdout.
#[cfg(test)]
mod tests {
    use index::{Line, Column};
    use super::{Processor, Handler, Attr, TermInfo};
    use ::Rgb;

    #[derive(Default)]
    struct AttrHandler {
        attr: Option<Attr>,
    }

    impl Handler for AttrHandler {
        fn terminal_attribute(&mut self, attr: Attr) {
            self.attr = Some(attr);
        }
    }

    impl TermInfo for AttrHandler {
        fn lines(&self) -> Line {
            Line(24)
        }

        fn cols(&self) -> Column {
            Column(80)
        }
    }

    #[test]
    fn parse_control_attribute() {
        static BYTES: &'static [u8] = &[
            0x1b, 0x5b, 0x31, 0x6d
        ];

        let mut parser = Processor::new();
        let mut handler = AttrHandler::default();

        for byte in &BYTES[..] {
            parser.advance(&mut handler, *byte);
        }

        assert_eq!(handler.attr, Some(Attr::Bold));
    }

    #[test]
    fn parse_truecolor_attr() {
        static BYTES: &'static [u8] = &[
            0x1b, 0x5b, 0x33, 0x38, 0x3b, 0x32, 0x3b, 0x31, 0x32,
            0x38, 0x3b, 0x36, 0x36, 0x3b, 0x32, 0x35, 0x35, 0x6d
        ];

        let mut parser = Processor::new();
        let mut handler = AttrHandler::default();

        for byte in &BYTES[..] {
            parser.advance(&mut handler, *byte);
        }

        let spec = Rgb {
            r: 128,
            g: 66,
            b: 255
        };

        assert_eq!(handler.attr, Some(Attr::ForegroundSpec(spec)));
    }

    /// No exactly a test; useful for debugging
    #[test]
    fn parse_zsh_startup() {
        static BYTES: &'static [u8] = &[
            0x1b, 0x5b, 0x31, 0x6d, 0x1b, 0x5b, 0x37, 0x6d, 0x25, 0x1b, 0x5b, 0x32, 0x37, 0x6d,
            0x1b, 0x5b, 0x31, 0x6d, 0x1b, 0x5b, 0x30, 0x6d, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x0d, 0x20, 0x0d, 0x0d, 0x1b, 0x5b, 0x30, 0x6d, 0x1b, 0x5b, 0x32,
            0x37, 0x6d, 0x1b, 0x5b, 0x32, 0x34, 0x6d, 0x1b, 0x5b, 0x4a, 0x6a, 0x77, 0x69, 0x6c,
            0x6d, 0x40, 0x6a, 0x77, 0x69, 0x6c, 0x6d, 0x2d, 0x64, 0x65, 0x73, 0x6b, 0x20, 0x1b,
            0x5b, 0x30, 0x31, 0x3b, 0x33, 0x32, 0x6d, 0xe2, 0x9e, 0x9c, 0x20, 0x1b, 0x5b, 0x30,
            0x31, 0x3b, 0x33, 0x32, 0x6d, 0x20, 0x1b, 0x5b, 0x33, 0x36, 0x6d, 0x7e, 0x2f, 0x63,
            0x6f, 0x64, 0x65
        ];

        let mut handler = AttrHandler::default();
        let mut parser = Processor::new();

        for byte in &BYTES[..] {
            parser.advance(&mut handler, *byte);
        }
    }
}

use libc;
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::RawFd;
use std::process;
use termios::*;

fn get_stdin_fd() -> RawFd {
    libc::STDIN_FILENO
}

fn get_stdout_fd() -> RawFd {
    libc::STDOUT_FILENO
}

fn get_col() -> u16 {
    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { libc::ioctl(get_stdout_fd(), libc::TIOCGWINSZ, &mut winsize) } == 0 {
        winsize.ws_col
    } else {
        80
    }
}

mod keys {
    pub(crate) const CTRL_A: u8 = 1;
    pub(crate) const CTRL_B: u8 = 2;
    pub(crate) const CTRL_C: u8 = 3;
    pub(crate) const CTRL_D: u8 = 4;
    pub(crate) const CTRL_E: u8 = 5;
    pub(crate) const CTRL_F: u8 = 6;
    pub(crate) const CTRL_H: u8 = 8;
    pub(crate) const CTRL_I: u8 = 9;
    pub(crate) const CTRL_J: u8 = 10;
    pub(crate) const CTRL_K: u8 = 11;
    pub(crate) const CTRL_L: u8 = 12;
    pub(crate) const CTRL_M: u8 = 13;
    pub(crate) const ESC: u8 = 27;
    pub(crate) const ONE: u8 = 49;
    pub(crate) const TWO: u8 = 50;
    pub(crate) const THREE: u8 = 51;
    pub(crate) const FOUR: u8 = 52;
    pub(crate) const FIVE: u8 = 53;
    pub(crate) const SIX: u8 = 54;
    pub(crate) const A: u8 = 65;
    pub(crate) const B: u8 = 66;
    pub(crate) const C: u8 = 67;
    pub(crate) const D: u8 = 68;
    // This char is `[`.
    pub(crate) const LEFT_BRACKET: u8 = 91;
    pub(crate) const BACKSPACE: u8 = 127;
}

type Completion = fn(&Vec<u8>, &mut Vec<&[u8]>);

struct Line<'a> {
    backup: Termios,
    position: usize,
    buffer: &'a mut Vec<u8>,
    prompt: &'a [u8],
    completion: &'a Option<Completion>,
    multi: bool,
    row: usize,
}

impl<'a> Line<'a> {
    fn new(
        buffer: &'a mut Vec<u8>,
        prompt: &'a [u8],
        completion: &'a Option<Completion>,
        multi: bool,
    ) -> Self {
        let backup = Termios::from_fd(get_stdin_fd()).unwrap();
        Line::enable_raw_mode();
        Line {
            backup,
            position: 0,
            buffer,
            prompt,
            completion,
            multi,
            row: 0,
        }
    }

    fn enable_raw_mode() {
        let fd = get_stdin_fd();
        let mut termios = Termios::from_fd(fd).unwrap();

        termios.c_iflag &= !(BRKINT | INPCK | ISTRIP | ICRNL | IXON);
        termios.c_oflag &= !OPOST;
        termios.c_cflag |= CS8;
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 1;
        termios.c_cc[VTIME] = 0;

        tcsetattr(fd, TCSANOW, &termios).unwrap();
        tcflush(fd, TCIOFLUSH).unwrap();
    }

    fn disable_raw_mode(&self) {
        let fd = get_stdin_fd();
        tcsetattr(fd, TCSANOW, &self.backup).unwrap();
        tcflush(fd, TCIOFLUSH).unwrap();
    }

    fn refresh_single_line(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(
            &[
                b"\x1b[0G\x1b[K",
                self.prompt,
                &self.buffer[..],
                format!("\r\x1b[{}C", self.position + self.prompt.len()).as_bytes(),
            ]
            .concat(),
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn refresh_multi_line(&mut self) -> io::Result<()> {
        let col = get_col() as usize;
        let mut stdout = io::stdout();
        if self.row == 0 {
            stdout.write_all(b"\x1b[0G\x1b[J")?;
        } else {
            stdout.write_all(format!("\x1b[0G\x1b[{}A\x1b[J", self.row).as_bytes())?;
        }
        let mut cnt = 0;
        let mut row: usize = 0;
        for c in self.prompt.iter().chain(self.buffer.iter()) {
            stdout.write_all(&[*c])?;
            cnt += 1;
            if cnt == col {
                stdout.write_all(b"\n\x1b[0G")?;
                cnt = 0;
                row += 1;
            }
        }
        stdout.write_all(b"\r")?;
        if row == 0 {
            stdout.write_all(b"\x1b[0G")?;
        } else {
            stdout.write_all(format!("\x1b[0G\x1b[{}A", row).as_bytes())?;
        }
        let pos = self.prompt.len() + self.position;
        let m = pos % col;
        self.row = pos / col;
        if self.row > 0 {
            stdout.write_all(format!("\x1b[{}B", self.row).as_bytes())?;
        }
        if m > 0 {
            stdout.write_all(format!("\x1b[{}C", m).as_bytes())?;
        }
        stdout.flush()?;
        Ok(())
    }

    fn refresh_line(&mut self) -> io::Result<()> {
        if self.multi {
            self.refresh_multi_line()
        } else {
            self.refresh_single_line()
        }
    }

    fn completion(&mut self, callback: &Completion) -> io::Result<u8> {
        let mut completions = Vec::new();
        callback(self.buffer, &mut completions);
        if completions.len() == 0 {
            return Ok(0);
        }
        let mut stdin = io::stdin();
        let bk = self.buffer.clone();
        let mut buf = vec![0; 1];
        loop {
            for &comp in completions.iter() {
                self.buffer.clear();
                self.buffer.extend(comp);
                self.position = self.buffer.len();
                self.refresh_line()?;

                let n = stdin.read(&mut buf)?;
                assert_eq!(n, 1);

                match buf[0] {
                    keys::CTRL_I => {
                        continue;
                    }
                    keys::ESC => {
                        self.buffer.clear();
                        self.buffer.extend(&bk);
                        self.position = self.buffer.len();
                        self.refresh_line()?;
                        return Ok(buf[0]);
                    }
                    _ => {
                        return Ok(buf[0]);
                    }
                }
            }
        }
    }

    fn fetch(&mut self) -> io::Result<()> {
        let mut stdin = io::stdin();

        self.refresh_line()?;

        let mut buf = vec![0; 1];

        loop {
            let n = stdin.read(&mut buf)?;
            assert_eq!(n, 1);

            // Tab
            if buf[0] == keys::CTRL_I {
                match self.completion {
                    Some(callback) => {
                        let c = self.completion(callback)?;
                        if c == 0 {
                            continue;
                        }
                        buf[0] = c;
                    }
                    None => continue,
                }
            }

            if buf[0] == keys::ESC {
                let mut buf2 = vec![0; 3];
                let n = stdin.read(&mut buf2[0..1])?;
                assert_eq!(n, 1);
                match buf2[0] {
                    // arrows, home, end or del
                    keys::LEFT_BRACKET => {
                        let n = stdin.read(&mut buf2[1..2])?;
                        assert_eq!(n, 1);
                        match buf2[1] {
                            // HOME
                            keys::ONE => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                buf[0] = keys::CTRL_A;
                            }
                            // INS
                            keys::TWO => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                continue;
                            }
                            // DEL
                            keys::THREE => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                if self.position < self.buffer.len() {
                                    buf[0] = keys::CTRL_D;
                                } else {
                                    continue;
                                }
                            }
                            // END
                            keys::FOUR => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                buf[0] = keys::CTRL_E;
                            }
                            // PgUp
                            keys::FIVE => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                continue;
                            }
                            // PgDn
                            keys::SIX => {
                                let _ = stdin.read(&mut buf2[2..3])?;
                                continue;
                            }
                            // Up
                            keys::A => {
                                continue;
                            }
                            // Down
                            keys::B => {
                                continue;
                            }
                            // Right
                            keys::C => {
                                buf[0] = keys::CTRL_F;
                            }
                            // Left
                            keys::D => {
                                buf[0] = keys::CTRL_B;
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                    _ => {
                        // handle to esc
                        // ...
                        buf[0] = buf2[0];
                    }
                }
            }

            match buf[0] {
                // Move the cursor start of line.
                keys::CTRL_A => {
                    self.position = 0;
                    self.refresh_line()?;
                }
                // Move the cursor forward 1 column.
                keys::CTRL_B => {
                    if self.position == 0 {
                        continue;
                    }
                    self.position -= 1;
                    self.refresh_line()?;
                }
                // Exit the process.
                keys::CTRL_C => {
                    self.disable_raw_mode();
                    process::exit(0);
                }
                keys::CTRL_D => {
                    // If the buffer is empty, exit the process.
                    if self.buffer.len() == 0 {
                        self.disable_raw_mode();
                        process::exit(0);
                    // Delete a char at the cursor.
                    } else if self.position < self.buffer.len() {
                        self.buffer.remove(self.position);
                        self.refresh_line()?;
                    }
                }
                // Move the cursor end of line.
                keys::CTRL_E => {
                    self.position = self.buffer.len();
                    self.refresh_line()?;
                }
                // Move the cursor backward 1 column.
                keys::CTRL_F => {
                    if self.position == self.buffer.len() {
                        continue;
                    }
                    self.position += 1;
                    self.refresh_line()?;
                }
                keys::CTRL_H | keys::BACKSPACE => {
                    if self.position == 0 || self.buffer.len() == 0 {
                        continue;
                    }
                    self.position -= 1;
                    self.buffer.remove(self.position);
                    self.refresh_line()?;
                }
                // Enter
                keys::CTRL_J | keys::CTRL_M => {
                    break;
                }
                keys::CTRL_K => {
                    self.buffer.truncate(self.position);
                    self.refresh_line()?;
                }
                keys::CTRL_L => {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"\x1b[H\x1b[2J")?;
                    self.refresh_line()?;
                }
                // esc,
                keys::ESC => {
                    continue;
                }
                _ => {
                    if self.position < self.buffer.len() {
                        self.buffer[self.position] = buf[0];
                    } else {
                        self.buffer.extend(&buf);
                    }
                    self.position += 1;
                    self.refresh_line()?;
                }
            }
        }
        let mut stdout = io::stdout();
        stdout.write_all(format!("\n\x1b[{}D", self.prompt.len() + self.position).as_bytes())?;
        stdout.flush()?;
        Ok(())
    }
}

impl<'a> Drop for Line<'a> {
    fn drop(&mut self) {
        self.disable_raw_mode()
    }
}

pub struct Interaction<'a> {
    prompt: &'a [u8],
    completion: Option<Completion>,
    pub multi: bool,
}

impl<'a> Interaction<'a> {
    pub fn new(prompt: &'a [u8], completion: Option<Completion>, multi: bool) -> Self {
        Interaction {
            prompt,
            completion,
            multi,
        }
    }

    pub fn from(prompt: &'a [u8]) -> Self {
        Interaction::new(prompt, None, true)
    }

    pub fn from_str(prompt: &'a str) -> Self {
        Interaction::new(prompt.as_bytes(), None, true)
    }

    pub fn line(&self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; 0];
        Line::new(&mut buffer, &self.prompt, &self.completion, self.multi).fetch()?;
        Ok(buffer)
    }

    pub fn set_prompt(&mut self, prompt: &'a [u8]) {
        self.prompt = prompt
    }

    pub fn set_completion(&mut self, completion: Completion) {
        self.completion = Some(completion)
    }
}

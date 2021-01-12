use std::io;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::process;
use termios::*;

fn get_stdin_fd() -> RawFd {
    let stdin = io::stdin();
    stdin.as_raw_fd()
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
    pub(crate) const ENTER: u8 = 13;
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

struct Line<'a> {
    backup: Termios,
    position: usize,
    buffer: &'a mut Vec<u8>,
    prompt: &'a [u8],
}

impl<'a> Line<'a> {
    fn new(buffer: &'a mut Vec<u8>, prompt: &'a [u8]) -> Self {
        let backup = Termios::from_fd(get_stdin_fd()).unwrap();
        Line::enable_raw_mode();
        Line {
            backup,
            position: 0,
            buffer,
            prompt,
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

    fn refresh_line(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(
            &[
                format!("\x1b[{}D\x1b[K", self.prompt.len() + self.buffer.len() + 1).as_bytes(),
                self.prompt,
                &self.buffer[..],
                format!("\r\x1b[{}C", self.position + self.prompt.len()).as_bytes(),
            ]
            .concat(),
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn get(&mut self) -> io::Result<()> {
        let mut stdin = io::stdin();

        self.refresh_line()?;

        loop {
            let mut buf = vec![0; 1];
            let n = stdin.read(&mut buf)?;
            assert_eq!(n, 1);

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
                keys::CTRL_I => {
                    // TODO
                    continue;
                }
                keys::CTRL_J | keys::ENTER => {
                    break;
                }
                keys::CTRL_K => {
                    // TODO
                    continue;
                }
                // esc,
                keys::ESC => {
                    // TODO
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

pub struct Interaction {
    prompt: Vec<u8>,
}

impl Interaction {
    pub fn new() -> Self {
        Interaction { prompt: vec![0; 0] }
    }

    pub fn from(prompt: &[u8]) -> Self {
        Interaction {
            prompt: prompt.to_vec(),
        }
    }

    pub fn from_str(prompt: &str) -> Self {
        Interaction {
            prompt: prompt.as_bytes().to_vec(),
        }
    }

    pub fn line(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; 0];
        Line::new(&mut buffer, &self.prompt).get()?;
        Ok(buffer)
    }

    pub fn set_prompt(&mut self, prompt: &[u8]) {
        self.prompt = prompt.to_vec()
    }
}

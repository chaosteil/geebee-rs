pub struct Joypad {
    selection: Option<Selection>,
    buttons: [bool; 8],
    flag: u8,
    interrupts: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Up = 0,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

#[derive(Debug, Clone, Copy)]
enum Selection {
    Direction,
    Buttons,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            selection: None,
            buttons: [false; 8],
            flag: 0xff,
            interrupts: false,
        }
    }

    pub fn check_interrupts(&mut self) -> bool {
        let i = self.interrupts;
        self.interrupts = false;
        i
    }

    pub fn press(&mut self, button: Button) {
        if !self.buttons[button as usize] {
            self.buttons[button as usize] = true;
            self.interrupts = true;
        }
    }

    pub fn release(&mut self, button: Button) {
        self.buttons[button as usize] = false;
    }

    pub fn select(&mut self, flag: u8) {
        self.selection = match flag & 0x30 {
            0x10 => Some(Selection::Buttons),
            0x20 => Some(Selection::Direction),
            _ => None,
        };
        self.flag = (self.flag & 0xcf) | (flag & 0x30);
    }

    pub fn value(&self) -> u8 {
        (self.flag & 0xf0)
            | match self.selection {
                Some(Selection::Direction) => {
                    (if !self.buttons[Button::Right as usize] {
                        0x01
                    } else {
                        0
                    }) | (if !self.buttons[Button::Left as usize] {
                        0x02
                    } else {
                        0
                    }) | (if !self.buttons[Button::Up as usize] {
                        0x04
                    } else {
                        0
                    }) | (if !self.buttons[Button::Down as usize] {
                        0x08
                    } else {
                        0
                    })
                }
                Some(Selection::Buttons) => {
                    (if !self.buttons[Button::A as usize] {
                        0x01
                    } else {
                        0
                    }) | (if !self.buttons[Button::B as usize] {
                        0x02
                    } else {
                        0
                    }) | (if !self.buttons[Button::Select as usize] {
                        0x04
                    } else {
                        0
                    }) | (if !self.buttons[Button::Start as usize] {
                        0x08
                    } else {
                        0
                    })
                }
                None => self.flag & 0x0f,
            }
    }
}

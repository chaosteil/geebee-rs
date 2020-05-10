pub struct Joypad {
    selection: Selection,
    buttons: [bool; 8],
    flag: u8,
}

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

enum Selection {
    Direction,
    Buttons,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            selection: Selection::Direction,
            buttons: [false; 8],
            flag: 0xff,
        }
    }

    pub fn press(&mut self, button: Button) {
        self.buttons[button as usize] = true;
    }

    pub fn release(&mut self, button: Button) {
        self.buttons[button as usize] = false;
    }

    pub fn select(&mut self, flag: u8) {
        if flag & 0x10 == 0 {
            self.selection = Selection::Direction;
        } else if flag & 0x20 == 0 {
            self.selection = Selection::Buttons;
        }
        self.flag = flag & 0xf0;
    }

    pub fn value(&self) -> u8 {
        (self.flag & 0xf0)
            | match self.selection {
                Selection::Direction => {
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
                Selection::Buttons => {
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
                _ => 0x3f,
            }
    }
}

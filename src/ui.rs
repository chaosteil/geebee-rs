use crate::cpu::CPU;
use druid::{
    widget::{prelude::*, Flex, Label},
    AppLauncher, Color, Data, Lens, LocalizedString, PlatformError, Point, Rect, TimerToken,
    Widget, WindowDesc,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct LCDWidget {
    size: Size,
    serial: String,
    timer_id: TimerToken,
}

impl LCDWidget {
    fn new() -> Self {
        Self {
            size: Size::new(160.0, 144.0),
            serial: String::new(),
            timer_id: TimerToken::INVALID,
        }
    }
}

impl Widget<AppState> for LCDWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, _env: &Env) {
        match event {
            Event::WindowConnected => {
                ctx.request_paint();
                self.timer_id = ctx.request_timer(
                    Instant::now()
                        .checked_add(Duration::from_millis(16))
                        .unwrap(),
                );
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    ctx.request_paint();
                    data.cpu.borrow_mut().step();
                    let cpu = data.cpu.borrow();
                    let serial = cpu.serial();
                    if self.serial.len() != serial.len() {
                        self.serial = std::str::from_utf8(serial).unwrap().to_string();
                    }
                    self.timer_id = ctx.request_timer(
                        Instant::now()
                            .checked_add(Duration::from_millis(16))
                            .unwrap(),
                    );
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppState,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppState, _data: &AppState, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppState,
        _env: &Env,
    ) -> Size {
        bc.constrain(self.size)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &AppState, _env: &Env) {
        let rect = Rect::from_origin_size(Point::ORIGIN, self.size);
        ctx.fill(rect, &Color::WHITE);
    }
}

#[derive(Clone, Data, Lens)]
struct AppState {
    cpu: Rc<RefCell<CPU>>,
}

fn make_widget() -> impl Widget<AppState> {
    Flex::column().with_child(Flex::row().with_child(LCDWidget::new()))
}

pub fn launch(cpu: CPU) -> Result<(), PlatformError> {
    let window = WindowDesc::new(make_widget)
        .title(LocalizedString::new("geebee-window-title").with_placeholder("GeeBee"))
        .resizable(false);
    AppLauncher::with_window(window).launch(AppState {
        cpu: Rc::new(RefCell::new(cpu)),
    })
}

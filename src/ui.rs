use druid::{
    widget::prelude::*, AppLauncher, Color, LocalizedString, PlatformError, Point, Rect, Widget,
    WindowDesc,
};

struct LCDWidget {
    size: Size,
}

impl LCDWidget {
    fn new() -> Self {
        Self {
            size: Size::new(160.0, 144.0),
        }
    }
}

impl Widget<()> for LCDWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut (), _env: &Env) {}
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &(), _env: &Env) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &(), _data: &(), _env: &Env) {}
    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &(),
        _env: &Env,
    ) -> Size {
        bc.constrain(self.size)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), _env: &Env) {
        let rect = Rect::from_origin_size(Point::ORIGIN, self.size);
        ctx.fill(rect, &Color::WHITE);
    }
}

pub fn launch() -> Result<(), PlatformError> {
    let window = WindowDesc::new(|| LCDWidget::new())
        .title(LocalizedString::new("geebee-window-title").with_placeholder("GeeBee"))
        .resizable(false);
    AppLauncher::with_window(window).launch(())
}

use gpui::{
	App, Bounds, ClickEvent, Context, FontWeight, Render, Window, WindowBounds,
	WindowOptions, div, prelude::*, px, rgb, size,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
	Library,
	Discover,
	Servers,
	Skins,
	Screenshots,
	Settings,
}

impl Page {
	fn label(self) -> &'static str {
		match self {
			Self::Library => "Library",
			Self::Discover => "Discover",
			Self::Servers => "Servers",
			Self::Skins => "Skins",
			Self::Screenshots => "Screenshots",
			Self::Settings => "Settings",
		}
	}
}

struct AppShell {
	active_page: Page,
}

impl AppShell {
	fn navigation_item(
		&self,
		page: Page,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let active = self.active_page == page;
		let background = if active { rgb(0x293829) } else { rgb(0x202120) };
		let foreground = if active { rgb(0xb9e8a7) } else { rgb(0xd0d0cc) };

		div()
			.id(format!("nav-{}", page.label().to_lowercase()))
			.w_full()
			.px_3()
			.py_2()
			.rounded(px(8.))
			.bg(background)
			.text_color(foreground)
			.font_weight(FontWeight::MEDIUM)
			.child(page.label())
			.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
				this.active_page = page;
				cx.notify();
			}))
	}
}

impl Render for AppShell {
	fn render(
		&mut self,
		_: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		div()
			.size_full()
			.flex()
			.bg(rgb(0x171817))
			.text_color(rgb(0xf1f1ec))
			.child(
				div()
					.w(px(248.))
					.h_full()
					.flex()
					.flex_col()
					.gap_2()
					.px_4()
					.py_5()
					.bg(rgb(0x1d1f1d))
					.border_r_1()
					.border_color(rgb(0x343734))
					.child(
						div()
							.px_3()
							.pb_4()
							.text_size(px(17.))
							.font_weight(FontWeight::SEMIBOLD)
							.text_color(rgb(0xb9e8a7))
							.child("AstralRinth"),
					)
					.child(
						div()
							.px_3()
							.pb_2()
							.text_size(px(11.))
							.font_weight(FontWeight::MEDIUM)
							.text_color(rgb(0x92968f))
							.child("NAVIGATION"),
					)
					.child(self.navigation_item(Page::Library, cx))
					.child(self.navigation_item(Page::Discover, cx))
					.child(self.navigation_item(Page::Servers, cx))
					.child(self.navigation_item(Page::Skins, cx))
					.child(self.navigation_item(Page::Screenshots, cx))
					.child(self.navigation_item(Page::Settings, cx)),
			)
			.child(
				div()
					.flex_1()
					.flex()
					.flex_col()
					.p_8()
					.child(
						div()
							.text_size(px(25.))
							.font_weight(FontWeight::SEMIBOLD)
							.child(self.active_page.label()),
					)
					.child(
						div()
							.mt_3()
							.text_size(px(14.))
							.text_color(rgb(0xa6aaa3))
							.child(
								"GPUI migration shell — launcher features are not connected yet.",
							),
					),
			)
	}
}

fn main() {
	gpui_platform::application().run(|cx: &mut App| {
		let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
		cx.open_window(
			WindowOptions {
				window_bounds: Some(WindowBounds::Windowed(bounds)),
				window_min_size: Some(size(px(1100.), px(700.))),
				app_id: Some("AstralRinthApp".into()),
				..Default::default()
			},
			|_, cx| {
				cx.new(|_| AppShell {
					active_page: Page::Library,
				})
			},
		)
		.expect("failed to open the AstralRinth GPUI window");
		cx.activate(true);
	});
}

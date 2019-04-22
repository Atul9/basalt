use keyboard::Qwery;
use std::time::Duration;
use std::thread;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use keyboard;
use mouse;
use Engine;

pub type InputHookID = u64;
pub type InputHookFn = Arc<FnMut(&InputHookData) -> InputHookRes + Send + Sync>;

pub enum InputHookRes {
	Remove,
	Success,
	Warning(String),
	Error(String),
}

pub enum InputHookTy {
	Press,
	Hold,
	Release,
	Character,
	MouseEnter,
	MouseLeave,
	MouseMove,
	MouseScroll,
	WindowFocused,
	WindowLostFocus,
}

pub enum InputHook {
	Press {
		keys: Vec<Qwery>,
		mouse_buttons: Vec<mouse::Button>,
	},
	Hold {
		keys: Vec<Qwery>,
		mouse_buttons: Vec<mouse::Button>,
		initial_delay: Duration,
		interval: Duration,
		accel: f32,
	},
	Release {
		keys: Vec<Qwery>,
		mouse_buttons: Vec<mouse::Button>,
	},
	Character,
	MouseEnter,
	MouseLeave,
	MouseMove,
	MouseScroll,
	WindowFocused,
	WindowLostFocus,
}

pub enum InputHookData {
	Press {
		mouse_x: f32,
		mouse_y: f32,
		key_active: HashMap<Qwery, bool>,
		mouse_active: HashMap<mouse::Button, bool>,
	},
	Hold {
		mouse_x: f32,
		mouse_y: f32,
		first_call: Instant,
		last_call: Instant,
		is_first_call: bool,
		initial_delay: Duration,
		initial_delay_wait: bool,
		initial_delay_elapsed: bool,
		interval: Duration,
		accel: f32,
		key_active: HashMap<Qwery, bool>,
		mouse_active: HashMap<mouse::Button, bool>,
	},
	Release {
		pressed: bool,
		key_active: HashMap<Qwery, bool>,
		mouse_active: HashMap<mouse::Button, bool>,
	},
	Character {
		char_ty: keyboard::CharType,
	},
	MouseEnter {
		mouse_x: f32,
		mouse_y: f32,
	},
	MouseLeave {
		mouse_x: f32,
		mouse_y: f32,
	},
	MouseMove {
		mouse_x: f32,
		mouse_y: f32,
		mouse_dx: f32,
		mouse_dy: f32,
	},
	MouseScroll {
		scroll_amt: f32,
	},
	WindowFocused,
	WindowLostFocus,
}

pub enum Event {
	KeyPress(Qwery),
	KeyRelease(Qwery),
	MousePress(mouse::Button),
	MouseRelease(mouse::Button),
	MouseMove(f32, f32),
	MouseScroll(f32),
}

pub struct Input {
	engine: Arc<Engine>,
}

impl Input {
	pub(crate) fn new(engine: Arc<Engine>) -> Arc<Self> {
		let input_ret = Arc::new(Input {
			engine,
		});
		
		thread::spawn(move || {
		
		});
		
		input_ret
	}
	
	pub fn send_event(&self, event: Event) {
	
	}
	
	pub fn add_hook(&self, hook: InputHook, func: InputHookFn) -> InputHookID {
		0
	}
}


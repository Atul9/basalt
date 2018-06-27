use Engine;
use std::sync::Arc;
use super::bin::{KeepAlive,Bin,BinInner};
use parking_lot::Mutex;
use std::thread;

/// Simple checkbox. Provides a change hook and the ability to get the state.
/// When checked, the inner box is set to being visible and vise versa.

impl KeepAlive for CheckBox {}

pub struct CheckBox {
	pub engine: Arc<Engine>,
	pub inner_box: Arc<Bin>,
	pub outer_box: Arc<Bin>,
	checked: Mutex<bool>,
	on_change: Mutex<Vec<Arc<Fn(bool) + Send + Sync>>>,
}

impl CheckBox {
	pub fn is_checked(&self) -> bool {
		*self.checked.lock()
	}
	
	pub fn set(&self, check: bool) {
		*self.checked.lock() = check;
		self.update(Some(check));
		self.call_on_change(Some(check));
	}
	
	pub fn check(&self) {
		self.set(true);
	}
	
	pub fn uncheck(&self) {
		self.set(false);
	}
	
	pub fn toggle(&self) {
		let mut checked = self.checked.lock();
		*checked = !*checked;
		self.update(Some(*checked));
		self.call_on_change(Some(*checked));
	}
	
	pub fn on_change(&self, func: Arc<Fn(bool) + Send + Sync>) {
		self.on_change.lock().push(func);
	}
	
	fn call_on_change(&self, checked_op: Option<bool>) {
		let checked = match checked_op {
			Some(some) => some,
			None => self.is_checked()
		};
	
		let on_change = self.on_change.lock().clone().into_iter();
		
		thread::spawn(move || {
			for func in on_change {
				func(checked);
			}
		});
	}
	
	fn update(&self, checked_op: Option<bool>) {
		let checked = match checked_op {
			Some(some) => some,
			None => self.is_checked()
		};
	
		self.inner_box.inner_update(BinInner {
			hidden: Some(!checked),
			.. self.inner_box.inner_copy()
		});
	}
	
	pub fn new(engine: Arc<Engine>) -> Arc<Self> {
		let itf_ = engine.interface();
		let mut bins = itf_.lock().new_bins(2);

		let checkbox = Arc::new(CheckBox {
			engine: engine,
			inner_box: bins.pop().unwrap(),
			outer_box: bins.pop().unwrap(),
			checked: Mutex::new(false),
			on_change: Mutex::new(Vec::new()),
		});
		
		checkbox.outer_box.add_child(checkbox.inner_box.clone());
		let checkbox_wk = Arc::downgrade(&checkbox);
		
		checkbox.outer_box.on_left_mouse_press(Arc::new(move || {
			match checkbox_wk.upgrade() {
				Some(checkbox) => checkbox.toggle(),
				None => return
			}
		}));
		
		checkbox
	}
}
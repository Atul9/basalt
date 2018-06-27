use Engine;
use std::sync::Arc;
use super::bin::{KeepAlive,Bin,BinInner,PositionTy,Color};
use std::sync::atomic::{self,AtomicBool};
use parking_lot::Mutex;
use mouse;
use super::interface::Interface;

impl KeepAlive for ScrollBar {}

pub struct ScrollBar {
	pub engine: Arc<Engine>,
	pub to_scroll: Arc<Bin>,
	pub container: Arc<Bin>,
	pub up_button: Arc<Bin>,
	pub down_button: Arc<Bin>,
	pub slidy_bit: Arc<Bin>,
	hooks: Mutex<Hooks>,
}

pub enum ScrollTo {
	Same,
	Top,
	Bottom,
	Amt(f32),
	Percent(f32),
}

#[derive(Default)]
struct Hooks {
	ms: Vec<u64>,
	kb: Vec<u64>,
}

const DEFAULT_WIDTH: f32 = 15.0;

impl Drop for ScrollBar {
	fn drop(&mut self) {
		let mut hooks = self.hooks.lock();
		
		for id in hooks.ms.split_off(0) {
			self.engine.mouse().delete_hook(id);
		}
		
		for id in hooks.kb.split_off(0) {
			self.engine.keyboard().delete_hook(id);
		}
	}
}

impl ScrollBar {
	pub fn new(engine: Arc<Engine>, interface_op: Option<&mut Interface>, parent_: Option<Arc<Bin>>, to_scroll: Arc<Bin>) -> Arc<Self> {
		let mut bins: Vec<_> = match interface_op {
			Some(mut itf) => { (0..4).into_iter().map(|_| itf.new_bin()).collect() },
			None => {
				let itf_ = engine.interface();
				let mut itf = itf_.lock();
				(0..4).into_iter().map(|_| itf.new_bin()).collect()
			}
		}; bins.reverse();
	
		let scroll_bar = Arc::new(ScrollBar {
			engine: engine.clone(),
			to_scroll: to_scroll,
			container: bins.pop().unwrap(),
			up_button: bins.pop().unwrap(),
			down_button: bins.pop().unwrap(),
			slidy_bit: bins.pop().unwrap(),
			hooks: Mutex::new(Hooks::default()),
		});
		
		if let &Some(ref parent) = &parent_ {
			parent.add_child(scroll_bar.container.clone());
		}
		
		scroll_bar.container.add_child(scroll_bar.up_button.clone());
		scroll_bar.container.add_child(scroll_bar.down_button.clone());
		scroll_bar.container.add_child(scroll_bar.slidy_bit.clone());
		
		scroll_bar.up_button.inner_update(BinInner {
			position_t: Some(PositionTy::FromParent),
			pos_from_t: Some(0.0),
			pos_from_l: Some(0.0),
			pos_from_r: Some(0.0),
			height: Some(DEFAULT_WIDTH),
			back_image: Some(String::from("./assets/icons/scroll_arrow_up.png")),
			back_color: Some(Color::from_hex("ffffff10")),
			.. BinInner::default()
		});
		
		scroll_bar.down_button.inner_update(BinInner {
			position_t: Some(PositionTy::FromParent),
			pos_from_b: Some(0.0),
			pos_from_l: Some(0.0),
			pos_from_r: Some(0.0),
			height: Some(DEFAULT_WIDTH),
			back_image: Some(String::from("./assets/icons/scroll_arrow_down.png")),
			back_color: Some(Color::from_hex("ffffff10")),
			.. BinInner::default()
		});
		
		let position_t = match parent_.is_some() {
			true => Some(PositionTy::FromParent),
			false => None
		};
		
		scroll_bar.container.inner_update(BinInner {
			position_t: position_t,
			border_size_l: Some(1.0),
			border_color_l: Some(Color::from_hex("a0a0a0")),
			back_color: Some(Color::from_hex("f8f8f8")),
			overflow_y: Some(true),
			overflow_x: Some(true),
			.. BinInner::default()
		});
		
		scroll_bar.slidy_bit.inner_update(BinInner {
			position_t: Some(PositionTy::FromParent),
			pos_from_t: (Some(DEFAULT_WIDTH)),
			pos_from_b: (Some(DEFAULT_WIDTH+50.0)),
			pos_from_l: (Some(2.0)),
			pos_from_r: (Some(2.0)),
			border_size_t: Some(1.0),
			border_size_b: Some(1.0),
			back_color: Some(Color::from_hex("a0a0a0")), 
			.. BinInner::default()
		});
		
		struct SlideStart {
			from_t: f32,
			mouse_y: f32,
		}
		
		let sliding = Arc::new(AtomicBool::new(false));
		let slide_start = Arc::new(Mutex::new(SlideStart {
			from_t: 0.0,
			mouse_y: 0.0,
		}));
		
		let _scroll_bar = Arc::downgrade(&scroll_bar);
		let _sliding = sliding.clone();
		let _slide_start = slide_start.clone();
		
		{
			let mut hooks = scroll_bar.hooks.lock();
		
			hooks.ms.push(engine.mouse().on_press(mouse::Button::Left, Arc::new(move |_, info| {
				let _scroll_bar = match _scroll_bar.upgrade() {
					Some(some) => some,
					None => return
				};
		
				if _scroll_bar.slidy_bit.mouse_inside(info.window_x, info.window_y) {
					let inner = _scroll_bar.slidy_bit.inner_copy();
			
					*_slide_start.lock() = SlideStart {
						from_t: inner.pos_from_t.unwrap_or(0.0),
						mouse_y: info.window_y,
					};
			
					_sliding.store(true, atomic::Ordering::Relaxed);
				}
			})));
		
			let _scroll_bar = Arc::downgrade(&scroll_bar);
		
			hooks.ms.push(engine.mouse().on_scroll(Arc::new(move |engine, x, y, amt| {
				let _scroll_bar = match _scroll_bar.upgrade() {
					Some(some) => some,
					None => return
				};
				
				let bin_id = match engine.interface_ref().lock().get_bin_id_atop(x, y) {
					Some(some) => some,
					None => return
				};
				
				let mut scroll_on = _scroll_bar.container.children_recursive();
				scroll_on.append(&mut _scroll_bar.to_scroll.children_recursive());
				let ids: Vec<_> = scroll_on.into_iter().map(|b| b.id()).collect();
				
				if ids.contains(&bin_id) {
					let cur = _scroll_bar.to_scroll.inner_copy().scroll_y.unwrap_or(0.0);
					_scroll_bar.set_scroll_amt(cur + (amt * 5.0));
				}
			})));
		
			let _scroll_bar = Arc::downgrade(&scroll_bar);
			let _sliding = sliding.clone();
			let _slide_start = slide_start.clone();
		
			hooks.ms.push(engine.mouse().on_move(Arc::new(move |_, _, _, _, mouse_y| {
				let _scroll_bar = match _scroll_bar.upgrade() {
					Some(some) => some,
					None => return
				};
			
				if _sliding.load(atomic::Ordering::Relaxed) {
					let slide_start = _slide_start.lock();
					let inner = _scroll_bar.slidy_bit.inner_copy();
					let mouse_diff = slide_start.mouse_y - mouse_y;
					let mut new_from_t = slide_start.from_t - mouse_diff;
					//let mut new_from_b = slide_start.from_b + mouse_diff;
					let min_from_t = _scroll_bar.up_button.inner_copy().height.unwrap_or(0.0);
					let min_from_b = _scroll_bar.down_button.inner_copy().height.unwrap_or(0.0);
					let container_bps = _scroll_bar.container.box_points();
					let container_height = container_bps.bli[1] - container_bps.tli[1];
					let overflow_amt = _scroll_bar.to_scroll.calc_overflow();
					let gap = f32::ceil(overflow_amt / 10.0);
					let mut new_height = container_height - min_from_t - min_from_b - gap;
				
					if new_height < 15.0 {
						new_height = 15.0;
					}
				
					let mut new_from_b = container_height - new_height - new_from_t;
				
					if new_from_t < min_from_t {
						let diff = min_from_t - new_from_t;
						new_from_t += diff;
						new_from_b -= diff;
					} else if new_from_b < min_from_b {
						let diff = min_from_b - new_from_b;
						new_from_t -= diff;
						new_from_b += diff;
					}
				
					let height = container_height - new_from_t - new_from_b;
					let max_from_t = container_height - height - min_from_b;
					let percent = (new_from_t - min_from_t) / (max_from_t - min_from_t);
					let scroll_amt = overflow_amt * percent;
				
					_scroll_bar.slidy_bit.inner_update(BinInner {
						pos_from_t: Some(new_from_t),
						pos_from_b: Some(new_from_b),
						.. inner
					});
				
					_scroll_bar.to_scroll.inner_update(BinInner {
						scroll_y: Some(scroll_amt),
						.. _scroll_bar.to_scroll.inner_copy()
					});
				
					_scroll_bar.to_scroll.update_children();
				}
			})));
		}
		
		let _scroll_bar = Arc::downgrade(&scroll_bar);
		
		scroll_bar.to_scroll.on_update(Arc::new(move || {
			let _scroll_bar = match _scroll_bar.upgrade() {
				Some(some) => some,
				None => return
			};
			
			_scroll_bar.force_update(ScrollTo::Same);
		}));
		
		let _sliding = sliding.clone();
		
		engine.mouse().on_release(mouse::Button::Left, Arc::new(move |_| {
			_sliding.store(false, atomic::Ordering::Relaxed);
		}));
		
		let _scroll_bar = Arc::downgrade(&scroll_bar);
		
		scroll_bar.up_button.on_left_mouse_press(Arc::new(move || {
			let _scroll_bar = match _scroll_bar.upgrade() {
				Some(some) => some,
				None => return
			};
			
			let set_to = _scroll_bar.to_scroll.inner_copy().scroll_y.unwrap_or(0.0) - 10.0;
			_scroll_bar.set_scroll_amt(set_to);
		}));
		
		let _scroll_bar = Arc::downgrade(&scroll_bar);
		
		scroll_bar.down_button.on_left_mouse_press(Arc::new(move || {
			let _scroll_bar = match _scroll_bar.upgrade() {
				Some(some) => some,
				None => return
			};
			
			let set_to = _scroll_bar.to_scroll.inner_copy().scroll_y.unwrap_or(0.0) + 10.0;
			_scroll_bar.set_scroll_amt(set_to);
		}));
		
		scroll_bar
	}
	
	pub fn force_update(&self, scroll_to: ScrollTo) {
		let min_from_t = self.up_button.inner_copy().height.unwrap_or(0.0);
		let min_from_b = self.down_button.inner_copy().height.unwrap_or(0.0);
		let container_bps = self.container.box_points();
		let container_height = container_bps.bli[1] - container_bps.tli[1];
		let overflow_amt = self.to_scroll.calc_overflow();
		let mut update_to_scroll = false;
		
		let amt = match scroll_to {
			ScrollTo::Same => {
				self.to_scroll.inner_copy().scroll_y.unwrap_or(0.0)
			}, ScrollTo::Top => {
				update_to_scroll = true;
				0.0
			}, ScrollTo::Bottom => {
				update_to_scroll = true;
				overflow_amt
			}, ScrollTo::Percent(p) => {
				update_to_scroll = true;
				overflow_amt * p
			}, ScrollTo::Amt(a) => {
				update_to_scroll = true;
				
				if a < 0.0 {
					0.0
				} else if a > overflow_amt {
					overflow_amt
				} else {
					a
				}
			}
		};
		
		if update_to_scroll {
			self.to_scroll.inner_update(BinInner {
				scroll_y: Some(amt),
				.. self.to_scroll.inner_copy()
			});
			
			self.to_scroll.update_children();
		}
		
		let slidy_inner = self.slidy_bit.inner_copy();
		let from_t = slidy_inner.pos_from_t.unwrap_or(0.0);
		let gap = f32::ceil(overflow_amt / 10.0);
		let mut height = container_height - min_from_t - min_from_b - gap;
		
		if height < 15.0 {
			height = 15.0;
		}
		
		let from_b = container_height - from_t - height;
		let max_from_t = container_height - height - min_from_b;
		let percent = amt / overflow_amt;
		let new_from_t = ((max_from_t - min_from_t) * percent) + min_from_t;
		let new_from_b = (from_t - new_from_t) + from_b;
		
		self.slidy_bit.inner_update(BinInner {
			pos_from_t: Some(new_from_t),
			pos_from_b: Some(new_from_b),
			.. slidy_inner
		});
	}
	
	pub fn set_scroll_amt(&self, amt: f32) {
		self.force_update(ScrollTo::Amt(amt));
	}
}
use crossbeam::sync::MsQueue;
use std::sync::{Arc,Barrier};
use std::thread;
use std::collections::BTreeMap;
use std::time::Instant;
use misc::BTreeMapExtras;
use winit;
use std::time::Duration;
use Engine;
use parking_lot::Mutex;

type OnMoveFunc = Arc<Fn(&Arc<Engine>, f32, f32, f32, f32) + Send + Sync>;
type OnPressFunc = Arc<Fn(&Arc<Engine>, PressInfo) + Send + Sync>;
type WhilePressFunc = Arc<Fn(&Arc<Engine>, f32, PressInfo) + Send + Sync>;
type OnReleaseFunc = Arc<Fn(&Arc<Engine>) + Send + Sync>;
type OnScrollFunc = Arc<Fn(&Arc<Engine>, f32, f32, f32) + Send + Sync>;

pub struct Mouse {
	event_queue: Arc<MsQueue<Event>>,
	func_queue: Arc<MsQueue<(u64, AddFunc)>>,
	hook_i: Mutex<u64>,
}

#[derive(Clone,Debug)]
pub struct PressInfo {
	pub button: Button,
	pub window_x: f32,
	pub window_y: f32,
	pub world_x: f32,
	pub world_y: f32,
	pub world_z: f32,
	pub normal_x: f32,
	pub normal_y: f32,
	pub normal_z: f32,
}

#[derive(PartialOrd,Ord,PartialEq,Eq,Clone,Debug)]
pub enum Button {
	Left,
	Right,
	Middle,
	Other(u8),
}

impl Button {
	pub(crate) fn from_winit(wb: winit::MouseButton) -> Self {
		match wb {
			winit::MouseButton::Left => Button::Left,
			winit::MouseButton::Right => Button::Right,
			winit::MouseButton::Middle => Button::Middle,
			winit::MouseButton::Other(v) => Button::Other(v),
		}
	}
}

enum Event {
	Press(Button),
	Release(Button),
	Barrier(Arc<Barrier>),
	Position(f32, f32),
	Delta(f32, f32),
	Scroll(f32),
	CenterWorldPos(f32, f32, f32, f32, f32, f32),
	DeleteHook(u64),
}

enum AddFunc {
	OnMove(OnMoveFunc),
	OnPress((Button, OnPressFunc)),
	WhilePressed((Button, WhilePressFunc, u64)),
	OnRelease((Button, OnReleaseFunc)),
	OnScroll(OnScrollFunc),
	OnAnyPress(OnPressFunc),
}

impl Mouse {
	pub(crate) fn new(engine: Arc<Engine>) -> Self {
		let event_queue = Arc::new(MsQueue::new());
		let func_queue = Arc::new(MsQueue::new());
		let _event_queue = event_queue.clone();
		let _func_queue = func_queue.clone();
		
		thread::spawn(move || {
			let event_queue = _event_queue;
			let func_queue = _func_queue;
			
			enum HookTy {
				OnMove(OnMoveFunc),
				OnPress(Button, OnPressFunc),
				OnHold(Button, WhilePressFunc, u64, Instant),
				OnRelease(Button, OnReleaseFunc),
				OnScroll(OnScrollFunc),
				OnAnyPress(OnPressFunc),
			}
			
			let mut hooks: Vec<(u64, HookTy)> = Vec::new();
			let mut pressed: BTreeMap<Button, bool> = BTreeMap::new();
			let default_instant = Instant::now();
			let mut mouse_at = [0.0; 2];
			let mut world_pos = [0.0; 3];
			let mut normal = [0.0; 3];
			
			loop {
				while let Some((hook_id, add_func)) = func_queue.try_pop() {
					match add_func {
						AddFunc::OnMove(func) => {
							hooks.push((hook_id, HookTy::OnMove(func)));
						}, AddFunc::OnPress((button, func)) => {
							hooks.push((hook_id, HookTy::OnPress(button, func)));
						}, AddFunc::WhilePressed((button, func, every)) => {
							hooks.push((hook_id, HookTy::OnHold(button, func, every, default_instant.clone())));
						}, AddFunc::OnRelease((button, func)) => {
							hooks.push((hook_id, HookTy::OnRelease(button, func)));
						}, AddFunc::OnScroll(func) => {
							hooks.push((hook_id, HookTy::OnScroll(func)));
						}, AddFunc::OnAnyPress(func) => {
							hooks.push((hook_id, HookTy::OnAnyPress(func)));
						}
					}
				}
			
				let mut new_events = BTreeMap::new();
				let mut delta_x = 0.0;
				let mut delta_y = 0.0;
				let mut barriers = Vec::new();
				let mut moved = false;
				let mut scroll_amt = 0.0;

				while let Some(event) = event_queue.try_pop() {
					match event {
						Event::Press(button) => {
							*new_events.get_mut_or_create(&button, true) = true;
							pressed.get_mut_or_create(&button, false);
						}, Event::Release(button) => {
							*new_events.get_mut_or_create(&button, false) = false;
							pressed.get_mut_or_create(&button, true);
						}, Event::Barrier(barrier) => {
							barriers.push(barrier);
						}, Event::Position(x, y) => {
							mouse_at[0] = x;
							mouse_at[1] = y;
							moved = true;
						}, Event::Delta(x, y) => {
							delta_x += x;
							delta_y += y;
							moved = true;
						}, Event::Scroll(amt) => {
							scroll_amt += amt;
						}, Event::CenterWorldPos(x, y, z, nx, ny, nz) => {
							world_pos[0] = x;
							world_pos[1] = y;
							world_pos[2] = z;
							normal[0] = nx;
							normal[1] = ny;
							normal[2] = nz;
						}, Event::DeleteHook(hook_id) => {
							let mut found = None;
							for (i, &(id, _)) in hooks.iter().enumerate() {
								if id == hook_id {
									found = Some(i);
									break;
								}
							} if let Some(i) = found {
								hooks.swap_remove(i);
							} else {
								println!("[ENGINE]: Mouse failed to remove hook id: {}", hook_id);
							}
						}
					}
				}
				
				if moved {
					for &(_, ref hook) in &hooks {
						if let &HookTy::OnMove(ref func) = hook {
							func(&engine, delta_x, delta_y, mouse_at[0], mouse_at[1]);
						}
					}
				}
				
				if scroll_amt != 0.0 {
					for &(_, ref hook) in &hooks {
						if let &HookTy::OnScroll(ref func) = hook {
							func(&engine, mouse_at[0], mouse_at[1], scroll_amt);
						}
					}
				}

				for (button, state) in &mut pressed {
					match new_events.get(button) {
						Some(new_state) => if state != new_state {
							if *new_state {
								*state = *new_state;
								
								for &mut (_, ref mut hook) in &mut hooks {
									if let &mut HookTy::OnHold(ref b, _, _, ref mut last) = hook {
										if b == button {
											*last = Instant::now();
										}
									}
								}
								
								for &(_, ref hook) in &hooks {
									match hook {
										&HookTy::OnPress(ref b, ref func) => {
											if b == button {
												func(&engine, PressInfo {
													button: button.clone(),
													window_x: mouse_at[0],
													window_y: mouse_at[1],
													world_x: world_pos[0],
													world_y: world_pos[1],
													world_z: world_pos[2],
													normal_x: normal[0],
													normal_y: normal[1],
													normal_z: normal[2],
												});
											}
										}, &HookTy::OnAnyPress(ref func) => {
											func(&engine, PressInfo {
												button: button.clone(),
												window_x: mouse_at[0],
												window_y: mouse_at[1],
												world_x: world_pos[0],
												world_y: world_pos[1],
												world_z: world_pos[2],
												normal_x: normal[0],
												normal_y: normal[1],
												normal_z: normal[2],
											});
										}, _ => ()
									}
								}
							} else {
								*state = *new_state;
								
								for &(_, ref hook) in &hooks {
									if let &HookTy::OnRelease(ref b, ref func) = hook {
										if b == button {
											func(&engine);
										}
									}
								}
							}
						}, None => {
							if *state {
								for &mut (_, ref mut hook) in &mut hooks {
									if let &mut HookTy::OnHold(ref b, ref func, ref every, ref mut last) = hook {
										if b == button {
											let duration = last.elapsed();
											let millis = (duration.as_secs()*1000) as f32 + (duration.subsec_nanos() as f32/1000000.0);
										
											if millis as u64 >= *every {
												*last = Instant::now();
												func(&engine, millis, PressInfo {
													button: button.clone(),
													window_x: mouse_at[0],
													window_y: mouse_at[1],
													world_x: world_pos[0],
													world_y: world_pos[1],
													world_z: world_pos[2],
													normal_x: normal[0],
													normal_y: normal[1],
													normal_z: normal[2],
												});
											}
										}
									}
								}
							}
						}
					}
				}
				
				for barrier in barriers {
					barrier.wait();
				}
				
				thread::sleep(Duration::from_millis(5));
			}
		});
		
		Mouse {
			event_queue: event_queue,
			func_queue: func_queue,
			hook_i: Mutex::new(0),
		}
	}
	
	pub fn delay_test(&self) -> f64 {
		let barrier = Arc::new(Barrier::new(2));
		let now = Instant::now();
		self.event_queue.push(Event::Barrier(barrier.clone()));
		barrier.wait();
		let elapsed = now.elapsed();
		((elapsed.as_secs() * 1000000000) + elapsed.subsec_nanos() as u64) as f64 / 1000000.0
	}
	
	pub(crate) fn set_center_world_pos(&self, x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32) {
		self.event_queue.push(Event::CenterWorldPos(x, y, z, nx, ny, nz));
	} pub(crate) fn press(&self, button: Button) {
		self.event_queue.push(Event::Press(button));
	} pub(crate) fn release(&self, button: Button) {
		self.event_queue.push(Event::Release(button));
	} pub(crate) fn scroll(&self, amt: f32) {
		self.event_queue.push(Event::Scroll(amt));
	} pub(crate) fn set_position(&self, x: f32, y: f32) {
		self.event_queue.push(Event::Position(x, y));
	} pub(crate) fn add_delta(&self, x: f32, y: f32) {
		self.event_queue.push(Event::Delta(x, y));
	}

	fn next_hook_id(&self) -> u64 {
		let mut hook_i = self.hook_i.lock();
		let out = *hook_i;
		*hook_i += 1;
		out
	}
	
	pub fn delete_hook(&self, hook_id: u64) {
		self.event_queue.push(Event::DeleteHook(hook_id));
	}
	
	pub fn on_move(&self, func: OnMoveFunc) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::OnMove(func)));
		id
	}
	
	pub fn on_any_press(&self, func: OnPressFunc) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::OnAnyPress(func)));
		id
	}
	
	pub fn on_press(&self, button: Button, func: OnPressFunc) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::OnPress((button, func))));
		id
	}
	
	pub fn while_pressed(&self, button: Button, func: WhilePressFunc, every: u64) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::WhilePressed((button, func, every))));
		id
	}
	
	pub fn on_release(&self, button: Button, func: OnReleaseFunc) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::OnRelease((button, func))));
		id
	}
	
	pub fn on_scroll(&self, func: OnScrollFunc) -> u64 {
		let id = self.next_hook_id();
		self.func_queue.push((id, AddFunc::OnScroll(func)));
		id
	}
}

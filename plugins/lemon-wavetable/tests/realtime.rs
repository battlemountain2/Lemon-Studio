use lemon_wavetable::dsp::{Engine, MAX_VOICES, Settings};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

fn record_allocation() {
    let _ = ALLOCATIONS.try_with(|count| {
        if let Some(previous) = count.get() {
            count.set(Some(previous + 1));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn render_release_steal_and_choke_do_not_allocate() {
    let mut engine = Engine::new(48000.0);
    let settings = Settings { attack: 0.001, release: 0.001, unison: 16, ..Settings::default() };
    let mut left = [0.0; 256];
    let mut right = [0.0; 256];
    for identity in 0..MAX_VOICES {
        engine.note_on(Some(identity as i32), 0, 60, 1.0, &settings);
    }
    ALLOCATIONS.with(|count| count.set(Some(0)));
    let stolen = engine.note_on(Some(100), 0, 60, 1.0, &settings);
    engine.render_block(&settings, &mut left, &mut right, |_, _| {});
    engine.all_off(0);
    let mut endings = 0;
    engine.render_block(&settings, &mut left, &mut right, |_, _| endings += 1);
    engine.note_on(Some(101), 1, 60, 1.0, &settings);
    engine.choke(1, None, None, |_| endings += 1);
    let allocations = ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert!(stolen.is_some());
    assert_eq!(endings, MAX_VOICES + 1);
    assert_eq!(engine.active_voice_count(), 0);
    assert_eq!(allocations, 0);
}

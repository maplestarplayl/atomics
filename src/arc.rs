use std::{
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering, fence},
};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        let arc = Box::leak(Box::new(ArcData {
            ref_count: AtomicUsize::new(1),
            data,
        }));
        Arc {
            ptr: NonNull::from(arc),
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Ordering::Relaxed) == 1 {
            fence(Ordering::Acquire);
            unsafe { return Some(&mut arc.ptr.as_mut().data) }
        }
        None
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.data().ref_count.fetch_add(1, Ordering::Relaxed);
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

mod tests {
    
    
    #[test]
    fn test() {
        use super::*;
        use std::thread;
        static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

        struct DectorDrop;

        impl Drop for DectorDrop {
            fn drop(&mut self) {
                NUM_DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        let a = Arc::new(("a", DectorDrop));
        let b = a.clone();

        let t = thread::spawn(move || {
            assert_eq!(a.0, "a");
        });

        t.join().unwrap();

        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 0);

        drop(b);

        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 1);
    }
}

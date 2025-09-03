use std::{sync::{atomic::{AtomicUsize, Ordering}, Arc}, thread};

mod lockfreequeue;


pub fn run() {
        println!("--- 开始执行 Wait-Free 示例 ---");

        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // 同样启动 10 个线程
        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                // 每个线程增加 10000 次
                for _ in 0..10000 {
                    // 调用 fetch_add，这是一个单一的、保证能完成的指令。
                    // 它没有重试循环，每个线程都能在有限的步骤内完成自己的操作。
                    // 这就是 Wait-Free 的保证。
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let final_value = counter.load(Ordering::Relaxed);
        println!("Wait-Free 最终计数值: {}", final_value);
        assert_eq!(final_value, 100000);
        println!("--- Wait-Free 示例执行完毕 ---");
    }

mod tests {
    use crate::lfqueue::run;

    #[test]
    fn test_counter() {
        run();
    }
}
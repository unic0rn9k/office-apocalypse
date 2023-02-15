use std::sync::mpsc::*;
use std::thread::*;
use std::time::*;

use crate::rhi::*;

pub struct Profiler {
    task: Option<&'static str>,
    cpu_profiler: (Option<Instant>, Option<Instant>),
    gpu_profiler: (u32, u32),
    sender: Option<Sender<(&'static str, f64, f64)>>,
    thread: Option<JoinHandle<()>>,
}

impl Profiler {
    pub fn new(print: bool) -> Self {
        let cpu_profiler = (None, None);

        let gpu_profiler = unsafe {
            let mut queries = [u32::MAX, u32::MAX];
            gl!(gl::CreateQueries(gl::TIMESTAMP, 2, queries.as_mut_ptr())).unwrap();
            (queries[0], queries[1])
        };

        let (sender, thread) = if print {
            let (sender, receiver) = channel();
            let thread = std::thread::spawn(move || {
                while let Ok((task, cpu_time, gpu_time)) = receiver.recv() {
                    println!("Task {task}:");
                    println!("    CPU: {cpu_time}ms");
                    println!("    GPU: {gpu_time}ms");
                }
            });

            (Some(sender), Some(thread))
        } else {
            (None, None)
        };

        Self {
            task: None,
            cpu_profiler,
            gpu_profiler,
            sender,
            thread,
        }
    }

    pub fn begin_profile(&mut self, task: &'static str) {
        let _ = self.task.insert(task);
        let (cpu_start, _) = &mut self.cpu_profiler;
        let (gpu_start, _) = &mut self.gpu_profiler;

        if cpu_start.is_none() {
            let _ = cpu_start.insert(Instant::now());

            unsafe { gl!(gl::QueryCounter(*gpu_start, gl::TIMESTAMP)).unwrap() };
        }
    }

    pub fn end_profile(&mut self, task: &'static str) -> Option<(f64, f64)> {
        const RESULT: gl::types::GLenum = gl::QUERY_RESULT;
        const AVAILABLE: gl::types::GLenum = gl::QUERY_RESULT_AVAILABLE;

        let (cpu_start, cpu_end) = &mut self.cpu_profiler;
        let (gpu_start, gpu_end) = &mut self.gpu_profiler;

        if cpu_end.is_none() {
            let _ = cpu_end.insert(Instant::now());
            unsafe { gl!(gl::QueryCounter(*gpu_end, gl::TIMESTAMP)).unwrap() };
        }

        let mut completed = gl::FALSE as _;
        unsafe { gl!(gl::GetQueryObjectiv(*gpu_end, AVAILABLE, &mut completed)).unwrap() };
        if completed as u8 == gl::TRUE {
            let gpu_time = {
                let mut start = 0;
                unsafe { gl!(gl::GetQueryObjectui64v(*gpu_start, RESULT, &mut start)).unwrap() };

                let mut end = 0;
                unsafe { gl!(gl::GetQueryObjectui64v(*gpu_end, RESULT, &mut end)).unwrap() };

                (end - start) as f64 / 1_000_000.0
            };

            let cpu_time = {
                let start = cpu_start.expect("Measurement hasn't been started yet");
                let end = cpu_end.unwrap();
                end.duration_since(start).as_secs_f64() * 1000.0
            };

            if let Some(sender) = &self.sender {
                sender.send((task, cpu_time, gpu_time)).unwrap();
            }

            self.task = None;
            self.cpu_profiler = (None, None);

            Some((cpu_time, gpu_time))
        } else {
            None
        }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        drop(self.sender.take());

        let start_query = self.gpu_profiler.0;
        let end_query = self.gpu_profiler.1;
        loop {
            let mut available = 0;
            unsafe {
                gl!(gl::GetQueryObjectiv(
                    end_query,
                    gl::QUERY_RESULT_AVAILABLE,
                    &mut available
                ))
                .unwrap()
            }

            if available as u8 == gl::TRUE {
                break;
            }
        }

        unsafe { gl!(gl::DeleteQueries(2, [start_query, end_query].as_mut_ptr())).unwrap() };

        if let Some(handle) = self.thread.take() {
            handle.join().unwrap();
        }
    }
}

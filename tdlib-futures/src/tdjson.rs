use ::std;
use tdjson_sys::*;

use std::os::raw::{
    c_void,
    c_char,
};

use std::ffi::{CStr, CString};

use std::time::Duration;
use std::ops::Drop;

pub struct Client {
    client_ptr: *mut c_void
}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

impl Client {
    pub fn new() -> Self {
        unsafe {
            Client {
                client_ptr: td_json_client_create()
            }
        }
    }

    pub fn send(&self, request: &str) {
        let crequest = CString::new(request).unwrap();
        unsafe {
            td_json_client_send(
                self.client_ptr,
                crequest.as_ptr() as *const c_char
            )
        }
    }

    pub fn receive(&self, timeout: Duration) -> Option<&str> {
        let timeout = timeout.as_secs() as f64;

        unsafe {
            let answer = td_json_client_receive(
                self.client_ptr,
                timeout
            );

            let answer = answer as *const c_char;
            if answer == std::ptr::null() {
                return None;
            }
            let answer = CStr::from_ptr(answer);

            Some(answer.to_str().expect("JSON should be utf-8"))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            td_json_client_destroy(self.client_ptr)
        }
    }
}

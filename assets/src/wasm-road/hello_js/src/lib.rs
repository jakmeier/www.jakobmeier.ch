extern "C" {
    #[link_name = "print_char"]
    fn js_print_wrapper(c: u32);

    #[link_name = "print_string"]
    fn js_print_wrapper2(start: *const u8, len: u32);
}

#[export_name = "hello_world"]
pub fn hello_world() {
    for c in "hello world!".chars() {
        unsafe {
            js_print_wrapper(c as u32);
        }
    }
}

#[export_name = "hello_world_2"]
pub fn hello_world_2() {
    static MSG: &str = "hello world!";
    unsafe {
        js_print_wrapper2(MSG.as_ptr(), MSG.len() as u32);
    }
}

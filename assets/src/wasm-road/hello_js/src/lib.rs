extern "C" {
    #[link_name = "print_char"]
    fn js_print_wrapper(c: u32);
}

#[export_name = "hello_world"]
pub fn hello_world() {
    for c in "hello world!".chars() {
        unsafe {
            js_print_wrapper(c as u32);
        }
    }
}

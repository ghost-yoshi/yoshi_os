use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::println;



lazy_static! {
    static ref  IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt
    };
}

//1.FIRST HANDLING
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame){
    println!("Exception produite: {:?}", stack_frame)
}
//2. SECOND

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> !{
    panic!("Exception double fault \n {:#?}", stack_frame)
}

pub fn init_idt() {
    IDT.load();
}

/*
test implementations
*/

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
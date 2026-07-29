/* crates export + configurations */
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![test_runner(yoshi_os::test_runner)]
#![no_std]
#![no_main]
use core::panic::PanicInfo;
use yoshi_os::{println};


/*
utilities 
*/

pub trait Testable {
    fn run(&self) ;
}


/*
Actually main
*/

#[unsafe(no_mangle)]

pub extern "C" fn _start() -> ! {

    let special_char: &str = ":!";
    println!("Yosh Os :) {}", special_char);
    yoshi_os::init();

    x86_64::instructions::interrupts::int3();// crash voulu

    println!("still alive bruuh !");

        #[cfg(test)]
        test_main();
        loop {}
}

/*
Testing Implementations goes here
*/

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}

}

//
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yoshi_os::test_panic_handler(info);
}



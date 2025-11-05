use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::println;
use crate::memory::ALLOCATOR;
use core::alloc::{GlobalAlloc,Layout};
const KB4: usize = 4096;
const MB2: usize = 512 * 4096; // 2MB

// Allocate 4 KB
unsafe fn test_alloc_4k() -> *mut u8 {
    let layout = Layout::from_size_align(KB4, KB4).unwrap();
    ALLOCATOR.alloc(layout)
}

// Allocate 2 MB
unsafe fn test_alloc_2m() -> *mut u8 {
    let layout = Layout::from_size_align(MB2, MB2).unwrap();
    ALLOCATOR.alloc(layout)
}

// Free 4 KB memory
unsafe fn test_free_4k(ptr: *mut u8) {
    let layout = Layout::from_size_align(KB4, KB4).unwrap();
    ALLOCATOR.dealloc(ptr, layout)
}

// Free 2 MB memory
unsafe fn test_free_2m(ptr: *mut u8) {
    let layout = Layout::from_size_align(MB2, MB2).unwrap();
    ALLOCATOR.dealloc(ptr, layout)
}

fn simple_allocation() -> bool {
    let v1 = 41;
    let v2 = 33;
    let heap_value_1 = Box::new(v1);
    let heap_value_2 = Box::new(v2);

    if(*heap_value_1 != v1)
    {
        println!("@ {} {} != {}", heap_value_1, *heap_value_1, v1);
        return false;
    }

    if(*heap_value_2 != v2)
    {
        println!("@ {} {} != {}", heap_value_2, *heap_value_2, v2);
        return false;
    }

    return true;
}

unsafe fn check(page:*mut u8, v: u8, size: usize) -> bool
{

    println!("checking page value {:?} v:{}" , page, v);
    for i in 0 .. size {
        let pv = *page.add(i);
        if(pv != v){
            println!(" -> failed @{:?}[{}] = {}" , page, i, pv);
            return false;
        }
    }
    return true;
}


unsafe fn write(page:*mut u8, v: u8, size: usize)
{
    for i in 0..size {
        *page.add(i) = v;
    }
}

unsafe fn test_allocator() -> bool {
    let page_sz_4k = 4096;
    let page_sz_2m = 4096*512;
    let v = 1;

    println!("Allocate some pages...");

    let test_page_4k = test_alloc_4k();
    println!("alloc 4k @{:?}", test_page_4k);
    write(test_page_4k,v, page_sz_4k);

    let test_page_2m = test_alloc_2m();
    println!("alloc 2m @{:?}", test_page_2m);
    write(test_page_2m,v, page_sz_2m);


    if(!check(test_page_4k, v, page_sz_4k)){
        println!("4k failed");
        return false;
    }

    if(!check(test_page_2m, v, page_sz_2m)){
        println!("2m failed");
        return false;
    }

    // 4096/8 = 512
    let arr_p= test_alloc_4k();
    let arr = arr_p as *mut *mut u8;

    let arr_p2= test_alloc_4k();
    let arr2 = arr_p2 as *mut *mut u8;

    println!("Allocating 2mb as 512 4k...");
    for i in 0 .. 512 {
        let page = test_alloc_4k();
        write(page, 0xa, page_sz_4k);
        *arr.add(i) = page;
    }

    println!("Allocating 2mb as 512 4k...");
    for i in 0 .. 512 {
        let page = test_alloc_4k();
        for j in 0 .. 512 {
            if(*arr.add(j) == page) 
            {
                println!("allocator leaks, should never get {:?}", page);
                return false;
            }
        }
        write(page, 0xb, page_sz_4k);
        *arr2.add(i) = page;
    }

    for i in (0..512).step_by(255) {
        if !check(*arr.add(i), 0xa, page_sz_4k) {
            return false;
        }

        if !check(*arr2.add(i), 0xb, page_sz_4k) {
            return false;
        }
    }

    println!("Freeing first half....");
    for i in 0 .. 256 {
        test_free_4k(*arr.add(i));
    }

    for i in 0 .. 256 {
        test_free_4k(*arr2.add(i));
    }

    println!("Freeing second half....");
    for i in 256.. 512 {
        test_free_4k(*arr.add(i));
    }

    for i in 256 .. 512 {
        test_free_4k(*arr2.add(i));
    }    

    println!("Checking values...");
    if(!check(test_page_4k, v, page_sz_4k)){
        println!("4k failed after spray");
        return false;
    }

    if(!check(test_page_2m, v, page_sz_2m)){
        println!("2m failed after spray");
        return false;
    }

    test_free_4k(test_page_4k);
    test_free_2m(test_page_2m);
    test_free_4k(arr_p);
    test_free_4k(arr_p2);


    return true;
}

fn large_vec() -> bool{
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    let sum = vec.iter().sum::<u64>();
    let expected = (n - 1) * n / 2;
    if (sum != expected)
    {
        println!("vec expected: {}, got: {}", expected, sum);
        return false;
    }

    return true;
}

fn small_vec() -> bool{
    let n = 10;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    let sum = vec.iter().sum::<u64>();
    let expected = (n - 1) * n / 2;
    if (sum != expected)
    {
        println!("vec expected: {}, got: {}", expected, sum);
        return false;
    }

    return true;
}

pub fn test_all()
{
    if(simple_allocation())
    {
        println!("---- box test PASSED ----");
    }

    if(small_vec()) 
    {
        println!("---- small vec test PASSED ----");
    }


    if(large_vec()) 
    {
        println!("---- large vec test PASSED ----");
    }


    unsafe{
        if(test_allocator())
        {
            println!("---- allocator test PASSED ----");
        }
    }
}
